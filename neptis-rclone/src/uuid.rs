use diesel::deserialize::FromSql;
use diesel::expression::AsExpression;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use diesel::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsExpression)]
#[diesel(sql_type = Text)]
pub struct SqliteUuid(pub Uuid);

impl From<Uuid> for SqliteUuid {
    fn from(id: Uuid) -> Self {
        SqliteUuid(id)
    }
}

impl From<SqliteUuid> for Uuid {
    fn from(id: SqliteUuid) -> Self {
        id.0
    }
}

impl ToSql<Text, Sqlite> for SqliteUuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.0.to_string());
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for SqliteUuid {
    fn from_sql(mut bytes: diesel::sqlite::SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.read_blob())?;
        Ok(SqliteUuid(Uuid::parse_str(s)?))
    }
}
