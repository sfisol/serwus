use diesel::{
    sql_types::SmallInt,
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, ToSql, Output},
};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow, PartialEq)]
#[sql_type = "SmallInt"]
pub enum Role {
    User = 0,
    Admin = 1,
    SuperAdmin = 2,
    Tester = 3,
}

impl<DB: Backend> ToSql<SmallInt, DB> for Role
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let v = match *self {
            Role::User => 0,
            Role::Admin => 1,
            Role::SuperAdmin => 2,
            Role::Tester => 3,
        };
        v.to_sql(out)
    }
}

impl<DB: Backend> FromSql<SmallInt, DB> for Role
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = i16::from_sql(bytes)?;
        Ok(match v {
            0 => Role::User,
            1 => Role::Admin,
            2 => Role::SuperAdmin,
            3 => Role::Tester,
            v => return Err(format!("Unknown identity role {:?}", v).into()),
        })
    }
}
