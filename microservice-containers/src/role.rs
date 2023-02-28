#[cfg(feature = "diesel")]
use diesel::{
    sql_types::SmallInt,
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, ToSql, Output},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "diesel", derive(diesel::AsExpression, diesel::FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = SmallInt))]
#[cfg_attr(feature = "paperclip", derive(paperclip::actix::Apiv2Schema))]
pub enum Role {
    User = 0,
    Admin = 1,
    SuperAdmin = 2,
    Tester = 3,
}

#[cfg(feature = "diesel")]
impl<DB: Backend> ToSql<SmallInt, DB> for Role
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            Role::User => 0.to_sql(out),
            Role::Admin => 1.to_sql(out),
            Role::SuperAdmin => 2.to_sql(out),
            Role::Tester => 3.to_sql(out),
        }
    }
}

#[cfg(feature = "diesel")]
impl<DB: Backend> FromSql<SmallInt, DB> for Role
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: diesel::backend::RawValue<'_, DB>) -> deserialize::Result<Self> {
        let v = i16::from_sql(bytes)?;
        Ok(match v {
            0 => Role::User,
            1 => Role::Admin,
            2 => Role::SuperAdmin,
            3 => Role::Tester,
            v => return Err(format!("Unknown identity role {v:?}").into()),
        })
    }
}
