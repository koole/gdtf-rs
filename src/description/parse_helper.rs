//! Provides parsing helpers to work around internally tagged enums that produce wrong results
//! because of <https://github.com/serde-rs/serde/issues/1183>.
//!
//! This workaround involves annotating all "primitive" fields of an enum with a custom
//! `deserialize_with` function. The fields must implement [FromStr] or be an Option which contains
//! a value that implements FromStr.
//!
//! For example the following enum:
//! ```ignore
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! #[serde(tag = "@ComponentType")]
//! enum OutputType {
//!     Simple,
//!     Extended {
//!         #[serde(rename = "@Voltage")]
//!         voltage: Option<f64>,
//!     }
//! }
//! ```
//!
//! Should be adjusted to this:
//! ```ignore
//! use serde::Deserialize;
//! use gdtf::description::parse_helper::Parse;
//!
//! #[derive(Deserialize)]
//! #[serde(tag = "@ComponentType")]
//! enum OutputType {
//!     Simple,
//!     Extended {
//!         #[serde(rename = "@Voltage", deserialize_with = "Parse::deserialize")]
//!         voltage: Option<f64>,
//!     }
//! }
//! ```
//!
//! Note: support for each data-type must be opted into due to Rust rules. That can be done by
//! implementing the [CanParse] trait for each type, in this file. For example:
//!
//! ```ignore
//! use gdtf::description::parse_helper::CanParse;
//!
//! impl CanParse for f64 {}
//! ```

use serde::Deserializer;
use serde::de::{Error, IgnoredAny, MapAccess, Unexpected, Visitor};
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::str::FromStr;

pub trait CanParse: FromStr {}
impl CanParse for f64 {}

pub trait Parse<'de>: Sized {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, T> Parse<'de> for T
where
    T: CanParse,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ParseVisitor {
            marker: PhantomData,
        })
    }
}

impl<'de, T> Parse<'de> for Option<T>
where
    T: Parse<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(ParseOptVisitor {
            marker: PhantomData,
        })
    }
}

struct ParseVisitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for ParseVisitor<T>
where
    T: CanParse,
{
    type Value = T;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(std::any::type_name::<T>())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        T::from_str(v).map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
    }
}

struct ParseOptVisitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for ParseOptVisitor<T>
where
    T: Parse<'de>,
{
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("option")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Some)
    }
}

/// Generates the `with` module for the `Other` variant of an internally tagged enum, which holds
/// a tag value the GDTF specification does not define.
///
/// The variant is marked `#[serde(untagged)]`, so serde tries it only once every defined variant
/// has failed. Deserialization then reads the tag attribute verbatim, but refuses a defined value:
/// a defined variant whose own attributes are malformed stays an error rather than being recast
/// as `Other`. Serialization writes the value back as the tag attribute.
///
/// ```ignore
/// define_undefined_tag_helper!(mod other_support_type "@SupportType" ["Rope", "GroundSupport"]);
/// ```
macro_rules! define_undefined_tag_helper {
    (mod $modname:ident $tag:literal [$($defined:literal),+ $(,)?]) => {
        mod $modname {
            const DEFINED: &[&str] = &[$($defined),+];

            pub fn serialize<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                use ::serde::ser::SerializeMap;

                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry($tag, value)?;
                map.end()
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                $crate::description::parse_helper::deserialize_undefined_tag(
                    deserializer,
                    $tag,
                    DEFINED,
                )
            }
        }
    };
}

/// Reads the value of the `tag` attribute from a map, failing if it is one of `defined`.
pub fn deserialize_undefined_tag<'de, D>(
    deserializer: D,
    tag: &'static str,
    defined: &'static [&'static str],
) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(UndefinedTagVisitor { tag, defined })
}

struct UndefinedTagVisitor {
    tag: &'static str,
    defined: &'static [&'static str],
}

impl<'de> Visitor<'de> for UndefinedTagVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a `{}` attribute", self.tag)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut value = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == self.tag {
                value = Some(map.next_value::<String>()?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        let value = value.ok_or_else(|| A::Error::missing_field(self.tag))?;
        if self.defined.contains(&value.as_str()) {
            return Err(A::Error::invalid_value(
                Unexpected::Str(&value),
                &"a value the GDTF specification does not define",
            ));
        }
        Ok(value)
    }
}
