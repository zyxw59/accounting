use std::fmt;

use serde::{
    de::{Error as _, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};
use time::serde::format_description;

/// A wrapper around [`time::Date`] which implements `serde`.
///
/// It uses RFC 3339 format for human-readable formats, and the [Julian Day Number] for
/// non-human-readable formats.
///
/// [Julian Day Number]: https://en.wikipedia.org/wiki/Julian_day
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date(pub time::Date);

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

format_description!(rfc3339_date, Date, "[year]-[month]-[day]");

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error> {
        if deserializer.is_human_readable() {
            rfc3339_date::deserialize(deserializer).map(Date)
        } else {
            let jdn = i32::deserialize(deserializer)?;
            match time::Date::from_julian_day(jdn) {
                // wow, if only there were a function like `ComponentRange::into_de_error` that I
                // could use here...
                Err(err) => Err(D::Error::invalid_value(Unexpected::Signed(jdn as _), &err)),
                Ok(date) => Ok(Date(date)),
            }
        }
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            rfc3339_date::serialize(&self.0, serializer)
        } else {
            self.0.to_julian_day().serialize(serializer)
        }
    }
}
