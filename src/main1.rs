use byte_strings::c_str;
use indymilter::{
    Actions, Callbacks, Context, ContextActions, EomContext, MacroStage, NegotiateContext,
    SocketInfo, Status,
};
use std::{
    collections::HashMap,
    env,
    net::IpAddr,
    process,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, signal};

// Data that is shared across milter callbacks needs a representation. For
// example, a struct such as this one. This struct contains the client’s IP
// address and TLS version information.
struct TlsData {
    ip: Option<IpAddr>,
    tls_version: Option<String>,
}

type Frequencies = HashMap<String, usize>;

// A milter is an executable program that starts with the `main` function. For
// an async program `tokio::main` provides an asynchronous `main` function.
#[tokio::main]
async fn main() {
    // This program takes one argument, the TCP socket to bind to. We obtain
    // this socket and bind it at the start of the program.

    let args = env::args().collect::<Vec<_>>();

    if args.len() != 2 {
        eprintln!("usage: {} <socket>", args[0]);
        process::exit(1);
    }

    let listener = TcpListener::bind(&args[1])
        .await
        .expect("cannot open milter socket");

    // Global data does not need to be technically global or `static`. Instead,
    // create a value on the stack, and move an `Arc` reference to it into the
    // callback closures. We do this for our ‘global’ table of distinct TLS
    // versions.

    let tls_versions = Arc::new(Mutex::new(Frequencies::new()));
    let tls_versions_eom = tls_versions.clone();

    // The behaviour of the milter is implemented using callbacks: one callback
    // closure for each milter stage. `Callbacks` has a fluent builder API. Note
    // the use of `Box::pin` to obtain the proper type when delegating to
    // `async fn`s.
    //
    // For the `eom` (end-of-message) callback, the reference-counted
    // `tls_versions_eom` is moved into the closure, and then a clone is moved
    // into the async future.

    let callbacks = Callbacks::new()
        .on_negotiate(|context, _, _| Box::pin(negotiate(context)))
        .on_connect(|context, _, socket_info| Box::pin(connect(context, socket_info)))
        .on_helo(|context, _| Box::pin(helo(context)))
        .on_eom(move |context| Box::pin(eom(tls_versions_eom.clone(), context)))
        .on_close(|context| Box::pin(close(context)));

    let config = Default::default();

    // For shutting down the milter, we create a future that is ready when
    // SIGINT (the Control-C signal) is received. So, shut down the milter by
    // pressing Control-C. Any other shutdown mechanism could be used instead.

    let shutdown = signal::ctrl_c();

    // With these preparations we can start the milter and `await` the returned
    // future. The milter runs until it is shut down.

    indymilter::run(listener, callbacks, config, shutdown)
        .await
        .expect("milter execution failed");

    // At this point, the milter has been shut down and all sessions have been
    // terminated. Now, before exiting, print the global stats: the frequencies
    // of the different TLS versions seen.

    let tls_versions = tls_versions.lock().unwrap();
    println!("Frequencies of TLS versions seen:");
    println!("{:#?}", tls_versions);
}

async fn negotiate(context: &mut NegotiateContext<TlsData>) -> Status {
    // During negotiation we can customise some things for the current
    // connection. Namely, the actions that we are going to execute, and the
    // macros that we are going to look at.

    // First, we are interested in executing action `add_header` during the
    // end-of-message stage, so we have to request it from the MTA to make sure
    // it is available.

    context.requested_actions |= Actions::ADD_HEADER;

    // Second, we are going to look at macro `{tls_version}`, so once again we
    // request it. According to its documentation, Postfix can make this macro
    // available during the HELO stage.

    let macros = c_str!("{tls_version}");
    context.requested_macros.insert(MacroStage::Helo, macros.into());

    Status::Continue
}

async fn connect(context: &mut Context<TlsData>, socket_info: SocketInfo) -> Status {
    // During the `connect` stage, we create our context data struct containing
    // the client’s IP address. Extract the IP address from the callback
    // arguments and store it in the struct.

    let ip = match socket_info {
        SocketInfo::Inet(addr) => Some(addr.ip()),
        _ => None,
    };

    let tls_data = TlsData {
        ip,
        tls_version: None,
    };

    // Then, store the data in the callback context. The following callbacks can
    // access the data through this handle.

    context.data = Some(tls_data);

    Status::Continue
}

async fn helo(context: &mut Context<TlsData>) -> Status {
    // During the `helo` stage, we are interested in TLS version information
    // that should be available in macro `{tls_version}`.

    if let Some(tls_data) = &mut context.data {
        // We requested macro `{tls_version}` earlier during negotiation. Of
        // course, a client connecting in the clear is not using TLS, so this
        // macro is only available for secure connections.

        if let Some(tls_version) = context.macros.get(c_str!("{tls_version}")) {
            // TLS version info is available. Update the context data struct by
            // storing the TLS version in the appropriate field for later use.

            let tls_version = tls_version.to_string_lossy();

            tls_data.tls_version = Some(tls_version.into());
        }
    }

    Status::Continue
}

async fn eom(tls_versions: Arc<Mutex<Frequencies>>, context: &mut EomContext<TlsData>) -> Status {
    // Time to finish this message. First, remove the context data from the
    // context, and proceed if it is available.

    if let Some(TlsData { ip, tls_version }) = context.data.take() {
        // Convert the data to strings for its final use. We want to both add
        // this data to the message header, and use it to update the frequencies
        // in the global table.

        let ip = ip.map_or_else(|| "unknown".to_owned(), |ip| ip.to_string());
        let tls_version = tls_version.unwrap_or_else(|| "none".to_owned());

        // Now let’s try to add a header with the TLS version information we got
        // for this connection.

        let name = "TLS-Version-Info";
        let value = format!("ip={} tls-version={}", ip, tls_version);

        if let Err(e) = context.actions.add_header(name, value).await {
            // An error occurred while trying to communicate with the MTA. Print
            // the error and assume that the client will later retry after we
            // reject it with transient error `Tempfail`.

            eprintln!("failed to add header: {}", e);

            return Status::Tempfail;
        }

        // The message was successfully modified. Our header was added. Now we
        // also record the TLS version info in our global table, by incrementing
        // the count for this particular TLS version.

        let mut tls_versions = tls_versions.lock().unwrap();
        *tls_versions.entry(tls_version).or_insert(0) += 1;
    }

    Status::Continue
}

async fn close(context: &mut Context<TlsData>) -> Status {
    // When the connection is closed, drop the context data. This is not really
    // necessary, since the data will be dropped anyway when the session ends.
    // We do it here to have explicit control of the context data life cycle.

    context.data = None;

    Status::Continue
}
