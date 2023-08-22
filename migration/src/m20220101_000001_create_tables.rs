use sea_orm_migration::{prelude::*, sea_orm::DatabaseBackend};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Mail::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Mail::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(Mail::SenderLocalPart)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Mail::SenderDomain)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Mail::MessageId).string_len(200).not_null())
                    .col(ColumnDef::new(Mail::SendingHostName).string_len(200))
                    .col(ColumnDef::new(Mail::SendingIp).string_len(39).not_null())
                    .col(
                        ColumnDef::new(Mail::TimeReceived)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Mail::TimeAccepted).timestamp_with_time_zone())
                    .col(ColumnDef::new(Mail::Status).tiny_integer().not_null())
                    .clone(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mail_messageid")
                    .if_not_exists()
                    .table(Mail::Table)
                    .col(Mail::MessageId)
                    .unique()
                    .clone(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mail_senderip")
                    .if_not_exists()
                    .table(Mail::Table)
                    .col(Mail::SendingIp)
                    .col(Mail::Status)
                    .clone(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Recipient::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Recipient::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(Recipient::Recipient)
                            .string_len(100)
                            .not_null(),
                    )
                    .clone(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recipient_recipient")
                    .if_not_exists()
                    .table(Recipient::Table)
                    .col(Recipient::Recipient)
                    .unique()
                    .clone(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MailRecipient::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MailRecipient::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(MailRecipient::MailId).integer().not_null())
                    .col(
                        ColumnDef::new(MailRecipient::RecipientId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(MailRecipient::Table, MailRecipient::MailId)
                            .to(Mail::Table, Mail::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(MailRecipient::Table, MailRecipient::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .clone(),
            )
            .await?;

        // Now copy any old data over
        if Ok(true) == manager.has_table("incoming_mail").await
            && manager.get_database_backend() == DatabaseBackend::Postgres
        {
            let db = manager.get_connection();
            db.execute_unprepared(
                "INSERT INTO recipient (recipient)
                 SELECT DISTINCT LOWER(TRIM(UNNEST(STRING_TO_ARRAY(recipients, ', '))))
                 FROM incoming_mail",
            )
            .await?;

            db.execute_unprepared("ALTER TABLE mail ADD COLUMN old_id integer")
                .await?;

            db.execute_unprepared(
                "INSERT INTO mail (sender_local_part, sender_domain, message_id, sending_host_name, sending_ip, time_received, time_accepted, status, old_id)
                 SELECT sender_local_part, sender_domain, message_id, sending_host_name, sending_ip, time_received, time_accepted, status, incoming_mail_id
                 FROM incoming_mail
                 ON CONFLICT DO NOTHING",
            )
            .await?;

            db.execute_unprepared(
                "INSERT INTO mail_recipient (mail_id, recipient_id)
                 SELECT
                     a.id AS mail_id,
                     r.id AS recipient_id
                 FROM
                     recipient r
                 JOIN
                 (
                     SELECT
                         id,
                         incoming_mail_id,
                         LOWER(TRIM(UNNEST(STRING_TO_ARRAY(recipients, ', ')))) AS recipients
                     FROM
                         incoming_mail im
                     JOIN mail m ON
                         im.incoming_mail_id = m.old_id) AS a ON
                     r.recipient = a.recipients",
            )
            .await?;

            db.execute_unprepared("ALTER TABLE mail DROP COLUMN old_id")
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MailRecipient::Table).clone())
            .await?;

        manager
            .drop_table(Table::drop().table(Recipient::Table).clone())
            .await?;

        manager
            .drop_table(Table::drop().table(Mail::Table).clone())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Mail {
    Table,
    Id,
    SenderLocalPart,
    SenderDomain,
    MessageId,
    SendingHostName,
    SendingIp,
    TimeReceived,
    TimeAccepted,
    Status,
}

#[derive(DeriveIden)]
enum Recipient {
    Table,
    Id,
    Recipient,
}

#[derive(DeriveIden)]
enum MailRecipient {
    Table,
    Id,
    MailId,
    RecipientId,
}
