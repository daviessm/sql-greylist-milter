use std::{ffi::CString, net::IpAddr, str::FromStr, sync::Arc};

use chrono::{Duration, Utc};
use settings::{Settings, Rewrite};
use entity::{
    email_status::EmailStatus::*,
    mail,
    prelude::{MailActive, MailEntity, MailRecipientActive, RecipientActive, RecipientModel},
    recipient,
};
use indymilter::{
    Actions, Callbacks, Context, EomContext, MacroStage, NegotiateContext, SocketInfo, Status,
};
use ipnet::IpNet;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectOptions, Database,
    DatabaseConnection, DbErr, EntityTrait, Insert, QueryFilter, Set, TransactionError,
    TransactionTrait,
};
use tokio::{net::TcpListener, signal};
use tracing::{debug, error, info, warn};

pub mod settings;

#[cfg(test)]
pub mod tests;

#[derive(Clone, Debug)]
struct SessionData {
    pub mail: MailActive,
    pub recipients: Vec<(RecipientModel, RecipientStatus)>,
}

#[derive(Clone, Debug)]
enum RecipientStatus {
    Add(Vec<String>),
    Change(Vec<String>),
    Remove,
    Keep,
}

#[tokio::main]
async fn main() {
    // Set up logging
    tracing_subscriber::fmt::init();

    let config = Settings::new()
        .unwrap_or_else(|e| {
            panic!(
                "Unable to read configuration from /etc/{}.toml: {}",
                env!("CARGO_PKG_NAME"), e
            )
        });

    let allowed_networks = Arc::new(config.get_allow_from_networks());
    let blocked_senders = Arc::new(config.get_blocked_senders());
    let rewrite_addresses = Arc::new(config.get_rewrites());

    info!(
        "Starting {} version {} on {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        config.get_listen_address()
    );
    let listener = TcpListener::bind(config.get_listen_address())
        .await
        .expect("Unable to open milter socket");

    let mut db_options = ConnectOptions::new(config.get_db_url());
    db_options
        .max_connections(100)
        .min_connections(1)
        .connect_timeout(Duration::seconds(2).to_std().unwrap())
        .idle_timeout(Duration::seconds(5).to_std().unwrap());
    let db = Arc::new(
        Database::connect(db_options)
            .await
            .expect("Unable to connect to database"),
    );

    Migrator::down(&*db, None)
        .await
        .expect("Unable to undo migrations");
    Migrator::up(&*db, None)
        .await
        .expect("Unable to run migrations");

    let db_1 = db.clone();
    let db_2 = db.clone();

    let callbacks = Callbacks::new()
        .on_negotiate(|context, _, _| Box::pin(negotiate(context)))
        .on_connect(|context, hostname, socket_info| {
            Box::pin(handle_connect(context, hostname, socket_info))
        })
        .on_mail(|context, args| Box::pin(handle_mail(context, args)))
        .on_rcpt(move |context, args| {
            Box::pin(handle_rcpt(
                context,
                args,
                db_1.clone(),
                blocked_senders.clone(),
                rewrite_addresses.clone(),
            ))
        })
        .on_header(|context, name, value| Box::pin(handle_header(context, name, value)))
        .on_eoh(move |context| {
            Box::pin(handle_eoh(
                context,
                allowed_networks.clone(),
                db_2.clone(),
                config.get_greylist_time_seconds(),
            ))
        })
        .on_eom(move |context| Box::pin(handle_eom(context)));

    indymilter::run(listener, callbacks, Default::default(), signal::ctrl_c())
        .await
        .expect("milter execution failed");
}

async fn negotiate(context: &mut NegotiateContext<SessionData>) -> Status {
    context.requested_actions |= Actions::DELETE_RCPT | Actions::ADD_RCPT;
    context
        .requested_macros
        .insert(MacroStage::Eom, CString::new("{auth_type}").unwrap());

    Status::Continue
}

async fn handle_connect(
    session: &mut Context<SessionData>,
    hostname: CString,
    socket_info: SocketInfo,
) -> Status {
    let mut session_data = MailActive {
        time_received: Set(Utc::now().into()),
        ..Default::default()
    };

    if let SocketInfo::Inet(addr) = socket_info {
        debug!("Connect from {}", addr.ip());
        session_data.sending_ip = Set(addr.ip().to_string());
        if !hostname.is_empty() {
            session_data.sending_host_name = match hostname.into_string() {
                Ok(string) => Set(Some(string)),
                Err(err) => {
                    warn!("Unable to read host name: {}", err);
                    Set(None)
                }
            }
        } else {
            session_data.sending_host_name = Set(None);
        }
    }

    session.data = Some(SessionData {
        mail: session_data,
        recipients: vec![],
    });

    Status::Continue
}

async fn handle_mail(session: &mut Context<SessionData>, args: Vec<CString>) -> Status {
    debug!("MAIL FROM {:?}", args);
    let session_data = session.data.as_mut().expect("No session?");

    if !args.is_empty() {
        if let Ok(sender) = args[0].clone().into_string() {
            if sender.len() > 2 {
                // Assume the first and last characters are < and >
                let sender = &sender[1..sender.len() - 1];
                let mut sender_parts = sender.split('@');
                if let Some(sender_local_part) = sender_parts.next() {
                    session_data.mail.sender_local_part = Set(sender_local_part.to_string());
                } else {
                    warn!("No sender_local_part? (args from MAIL FROM: {:?})", args);
                    return Status::Reject;
                }
                if let Some(sender_domain) = sender_parts.next() {
                    session_data.mail.sender_domain = Set(sender_domain.to_string());
                    Status::Continue
                } else {
                    warn!("No sender_domain? (args from MAIL FROM: {:?})", args);
                    Status::Reject
                }
            } else {
                warn!("Sender length is < 2? (args from MAIL FROM: {:?})", args);
                Status::Reject
            }
        } else {
            warn!(
                "Sender is not a valid String? (args from MAIL FROM: {:?})",
                args
            );
            Status::Reject
        }
    } else {
        warn!("Null sender? (args from MAIL FROM: {:?})", args);
        Status::Reject
    }
}

async fn handle_rcpt(
    session: &mut Context<SessionData>,
    args: Vec<CString>,
    db: Arc<DatabaseConnection>,
    spam_addresses: Arc<Vec<String>>,
    rewrite_addresses: Arc<Vec<Rewrite>>,
) -> Status {
    debug!("RCPT TO {:?}", args);
    let session_data = session.data.as_mut().expect("No session?");

    if !args.is_empty() {
        if let Ok(recipient) = args[0].clone().into_string() {
            if recipient.len() > 2 {
                // Assume the first and last characters are < and >
                let recipient = &recipient[1..recipient.len() - 1];
                let recipient_active = RecipientActive {
                    recipient: Set(recipient.to_owned()),
                    ..Default::default()
                };

                session_data.recipients.push(
                    match Insert::one(recipient_active)
                        .on_conflict(
                            OnConflict::column(recipient::Column::Recipient)
                                .update_column(recipient::Column::Recipient)
                                .to_owned(),
                        )
                        .exec_with_returning(db.as_ref())
                        .await
                    {
                        Ok(model) => (
                            model,
                            match is_spam_address((*spam_addresses).clone(), recipient.to_owned()) {
                                true => RecipientStatus::Remove,
                                false => {
                                    change_address((*rewrite_addresses).clone(), recipient.to_owned())
                                }
                            },
                        ),
                        Err(e) => {
                            error!("Unable to insert recipient: {}", e);
                            return Status::Tempfail;
                        }
                    },
                );
                Status::Continue
            } else {
                warn!("Recipient length is < 2? (args from RCPT TO: {:?})", args);
                Status::Reject
            }
        } else {
            warn!(
                "Recipient is not a valid String? (args from RCPT TO: {:?})",
                args
            );
            Status::Reject
        }
    } else {
        warn!("Null recipient? (args from RCPT TO: {:?})", args);
        Status::Reject
    }
}

async fn handle_header(
    session: &mut Context<SessionData>,
    name: CString,
    value: CString,
) -> Status {
    debug!("Header {:?}: {:?}", name, value);
    let session_data = session.data.as_mut().expect("No session?");

    // Shortcut if we already have the message-id
    if session_data.mail.message_id.is_set() {
        return Status::Continue;
    }

    if let Ok(name) = name.to_str() {
        if name.eq_ignore_ascii_case("message-id") {
            if let Ok(value) = value.to_str() {
                session_data.mail.message_id = Set(value.to_string());
            }
        }
    } else {
        warn!("Header name is not a valid String? (name: {:?})", name);
    }

    Status::Continue
}

async fn handle_eoh(
    session: &mut Context<SessionData>,
    allowed_networks: Arc<Vec<IpNet>>,
    db: Arc<DatabaseConnection>,
    greylist_time_seconds: i64,
) -> Status {
    debug!(
        "EOH, {{auth_type}}: {:?}",
        session.macros.get(&CString::new("{auth_type}").unwrap())
    );
    let session_data = session.data.as_mut().expect("No session?");

    // Check we have enough information in the session now
    if session_data.mail.sending_ip.is_not_set()
        || session_data.mail.sender_local_part.is_not_set()
        || session_data.mail.sender_domain.is_not_set()
        || session_data.mail.message_id.is_not_set()
        || session_data.recipients.is_empty()
    {
        warn!(
            "End of headers but we don't have all the information we need? {:?}",
            session_data
        );
        return Status::Tempfail;
    }

    if let Ok(from_ip) = IpAddr::from_str(session_data.mail.sending_ip.clone().unwrap().as_str()) {
        // Locally-generated email
        if from_ip.is_loopback() {
            session_data.mail.status = Set(LocallyAccepted);
            session_data.mail.time_accepted = Set(Some(Utc::now().into()));
            insert_mail(session_data.to_owned(), db)
                .await
                .expect("Unable to connect to database");
            Status::Accept
        // Authenticated users
        } else if session
            .macros
            .get(&CString::new("{auth_type}").unwrap())
            .is_some()
        {
            session_data.mail.status = Set(AuthenticatedAccepted);
            session_data.mail.time_accepted = Set(Some(Utc::now().into()));
            insert_mail(session_data.to_owned(), db)
                .await
                .expect("Unable to connect to database");
            Status::Accept
        // Whitelisted networks
        } else if is_allowed_ip(allowed_networks, from_ip) {
            session_data.mail.status = Set(IpAccepted);
            session_data.mail.time_accepted = Set(Some(Utc::now().into()));
            insert_mail(session_data.to_owned(), db)
                .await
                .expect("Unable to connect to database");
            Status::Accept
        } else {
            // Does the message already exist in the database?
            if let Ok(Some(existing_message)) = MailEntity::find()
                .filter(mail::Column::MessageId.eq(session_data.mail.message_id.clone().unwrap()))
                .one(db.as_ref())
                .await
            {
                match existing_message.status {
                    Greylisted => {
                        // If the message was greylisted but we've waited long enough
                        if existing_message
                            .time_received
                            .checked_add_signed(Duration::seconds(greylist_time_seconds))
                            .unwrap()
                            < Utc::now()
                        {
                            let mut active_existing_message: mail::ActiveModel =
                                existing_message.into();
                            active_existing_message.status = Set(PassedGreylistAccepted);
                            active_existing_message.time_accepted = Set(Some(Utc::now().into()));
                            active_existing_message
                                .update(db.as_ref())
                                .await
                                .expect("Unable to connect to database");
                            Status::Accept
                        } else {
                            // We know there's already a record for this message in the database; reject this one
                            Status::Tempfail
                        }
                    }
                    Denied => Status::Discard,
                    AuthenticatedAccepted
                    | IpAccepted
                    | KnownGoodAccepted
                    | LocallyAccepted
                    | OtherAccepted
                    | PassedGreylistAccepted => Status::Accept,
                }
            } else {
                // Ok, no existing message, what about previous ones from the same server?
                if let Ok(Some(_)) = MailEntity::find()
                    .filter(
                        mail::Column::SendingIp
                            .eq(session_data.mail.sending_ip.clone().unwrap())
                            .and(mail::Column::Status.is_in([
                                PassedGreylistAccepted,
                                KnownGoodAccepted,
                                OtherAccepted,
                            ])),
                    )
                    .one(db.as_ref())
                    .await
                {
                    session_data.mail.status = Set(KnownGoodAccepted);
                    session_data.mail.time_accepted = Set(Some(Utc::now().into()));
                    insert_mail(session_data.to_owned(), db)
                        .await
                        .expect("Unable to connect to database");
                    Status::Accept
                // Nope? Ok, then we'll have to greylist
                } else if greylist_time_seconds > 0 {
                    session_data.mail.status = Set(Greylisted);
                    insert_mail(session_data.to_owned(), db)
                        .await
                        .expect("Unable to connect to database");
                    Status::Tempfail
                // Greylisting is disabled
                } else {
                    session_data.mail.status = Set(OtherAccepted);
                    session_data.mail.time_accepted = Set(Some(Utc::now().into()));
                    insert_mail(session_data.to_owned(), db)
                        .await
                        .expect("Unable to connect to database");
                    Status::Accept
                }
            }
        }
    } else {
        warn!(
            "Unable to parse IP address {}",
            session_data.mail.sending_ip.clone().unwrap()
        );
        Status::Tempfail
    }
}

async fn handle_eom(context: &mut EomContext<SessionData>) -> Status {
    Status::Continue
}

fn is_allowed_ip(allowed_networks: Arc<Vec<IpNet>>, address: IpAddr) -> bool {
    for allowed_network in allowed_networks.as_ref() {
        if allowed_network.contains(&address) {
            return true;
        }
    }
    false
}

fn is_spam_address(spam_addresses: Vec<String>, address: String) -> bool {
    for spam_address in spam_addresses {
        if spam_address.eq_ignore_ascii_case(&address) {
            return true;
        }
    }
    false
}

fn change_address(rewrite_addresses: Vec<Rewrite>, address: String) -> RecipientStatus {
    for rewrite_address in rewrite_addresses {
        if rewrite_address.old_to.eq_ignore_ascii_case(&address) {
            return match rewrite_address.action {
                settings::ChangeRecipientAction::Add => RecipientStatus::Add(rewrite_address.new_to),
                settings::ChangeRecipientAction::Replace => RecipientStatus::Change(rewrite_address.new_to)
            };
        }
    }
    RecipientStatus::Keep
}

async fn insert_mail(
    session: SessionData,
    db: Arc<DatabaseConnection>,
) -> Result<(), TransactionError<DbErr>> {
    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            let mail = session.mail.save(txn).await?;
            let mail_id = mail.id.unwrap();

            for recipient in session.recipients {
                MailRecipientActive {
                    mail_id: Set(mail_id),
                    recipient_id: Set(recipient.0.id),
                    ..Default::default()
                }
                .save(txn)
                .await?;
            }

            Ok(())
        })
    })
    .await
}
