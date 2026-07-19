use chrono::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::Result as SqlResult;
use uuid::Uuid;

/// Wrapper so we can implement `ToSql`/`FromSql` for `Option<DateTime<Utc>>`
/// via RFC3339 text without conflicting with chrono's own (feature-gated)
/// impls used elsewhere.
pub fn dt_to_sql(dt: &Option<DateTime<Utc>>) -> Option<String> {
    dt.map(|d| d.to_rfc3339())
}

pub fn dt_from_sql(s: Option<String>) -> SqlResult<Option<DateTime<Utc>>> {
    match s {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|d| Some(d.with_timezone(&Utc)))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))),
    }
}

pub fn uuid_to_sql(id: &Uuid) -> String {
    id.to_string()
}

pub fn uuid_from_sql(s: String) -> SqlResult<Uuid> {
    Uuid::parse_str(&s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

pub fn json_to_sql<T: serde::Serialize>(value: &T) -> SqlResult<String> {
    serde_json::to_string(value)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
}

pub fn json_from_sql<T: serde::de::DeserializeOwned>(s: &str) -> SqlResult<T> {
    serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

#[allow(dead_code)]
pub struct EnumText(pub String);

impl ToSql for EnumText {
    fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.clone()))
    }
}

impl FromSql for EnumText {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_str().map(|s| EnumText(s.to_string()))
    }
}

pub fn parse_err(field: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unrecognized {field} value: {value}"),
        )),
    )
}
