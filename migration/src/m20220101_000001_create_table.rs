use sea_orm_migration::prelude::*;

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
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Mail::SenderDomain)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Mail::Recipients).string_len(4000).not_null())
                    .col(ColumnDef::new(Mail::MessageId).string_len(100).not_null())
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
                    .to_owned(),
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
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mail_senderip")
                    .if_not_exists()
                    .table(Mail::Table)
                    .col(Mail::SendingIp)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Mail::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Mail {
    Table,
    Id,
    SenderLocalPart,
    SenderDomain,
    Recipients,
    MessageId,
    SendingHostName,
    SendingIp,
    TimeReceived,
    TimeAccepted,
    Status,
}
