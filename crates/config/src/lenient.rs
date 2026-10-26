use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::Deserializer;
use serde::de::{self, Visitor};

struct Number<T>(PhantomData<T>);

impl<T: FromStr> Visitor<'_> for Number<T> {
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
        v.trim()
            .parse()
            .map_err(|_| E::custom("expected a number in range"))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<T, E> {
        self.visit_str(&v.to_string())
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<T, E> {
        self.visit_str(&v.to_string())
    }
}

pub fn number<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
{
    d.deserialize_any(Number(PhantomData))
}

struct Flag {
    accepts_one: bool,
}

impl Visitor<'_> for Flag {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a boolean")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> {
        Ok(v)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<bool, E> {
        Ok(v == "true" || (self.accepts_one && v == "1"))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
        Ok(self.accepts_one && v == 1)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E> {
        Ok(self.accepts_one && v == 1)
    }
}

pub fn true_or_one<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    d.deserialize_any(Flag { accepts_one: true })
}

pub fn true_only<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    d.deserialize_any(Flag { accepts_one: false })
}
