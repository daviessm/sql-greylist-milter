use sea_orm::entity::prelude::*;

use super::email_status::EmailStatus;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "mail")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub sender_local_part: String,
    pub sender_domain: String,
    pub recipients: String,
    pub message_id: String,
    pub sending_host_name: Option<String>,
    pub sending_ip: String,
    pub time_received: DateTimeWithTimeZone,
    pub time_accepted: Option<DateTimeWithTimeZone>,
    pub status: EmailStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
