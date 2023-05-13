//! Helper functions for serde.

/// Serialization for [`time::Date`] that uses BSON's datetime format for non-human-readable
/// formats, and RFC 3339 date format for human-readable formats.
pub mod date {
    use bson::DateTime;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::{serde::format_description, Date, OffsetDateTime};

    format_description!(rfc3339_date, Date, "[year]-[month]-[day]");

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error> {
        if deserializer.is_human_readable() {
            rfc3339_date::deserialize(deserializer)
        } else {
            DateTime::deserialize(deserializer).map(|dt| OffsetDateTime::from(dt).date())
        }
    }

    pub fn serialize<S: Serializer>(date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            rfc3339_date::serialize(date, serializer)
        } else {
            DateTime::from(date.midnight().assume_utc()).serialize(serializer)
        }
    }
}
