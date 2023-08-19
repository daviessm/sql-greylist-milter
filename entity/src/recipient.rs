use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "recipient")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub recipient: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::mail::Entity")]
    Mail,
}

impl Related<super::mail::Entity> for Entity {
    fn to() -> RelationDef {
        super::mail_recipient::Relation::Mail.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::mail_recipient::Relation::Recipient.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
