use sea_orm::{EnumIter, DeriveActiveEnum};

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum EmailStatus {
    #[sea_orm(num_value = 0)]
    New,
    #[sea_orm(num_value = 1)]
    LocallyAcepted,
    #[sea_orm(num_value = 2)]
    IpAccepted,
    #[sea_orm(num_value = 3)]
    AuthenticatedAccepted,
    #[sea_orm(num_value = 4)]
    PassedGreylistAccepted,
    #[sea_orm(num_value = 5)]
    KnownGoodAccepted,
    #[sea_orm(num_value = 6)]
    OtherAccepted,
    #[sea_orm(num_value = 10)]
    Greylisted,
    #[sea_orm(num_value = 20)]
    Denied,
}