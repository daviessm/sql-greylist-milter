use sea_orm::{EnumIter, DeriveActiveEnum};

#[derive(Clone, PartialEq, Eq, Debug, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum EmailStatus {
    New = 0,
    LocallyAcepted = 1,
    IpAccepted = 2,
    AuthenticatedAccepted = 3,
    PassedGreylistAccepted = 4,
    KnownGoodAccepted = 5,
    OtherAccepted = 6,
    Greylisted = 10,
    Denied = 20,
}