use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "mail_recipient")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub mail_id: i32,
    pub recipient_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::mail::Entity",
        from = "Column::MailId",
        to = "super::mail::Column::Id"
    )]
    Mail,
    #[sea_orm(
        belongs_to = "super::recipient::Entity",
        from = "Column::RecipientId",
        to = "super::recipient::Column::Id"
    )]
    Recipient,
}

impl ActiveModelBehavior for ActiveModel {}
