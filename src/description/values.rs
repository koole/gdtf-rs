//! Basic attribute types used throughout device descriptions.

use super::util::IterUtil;
use serde::de::value::StrDeserializer;
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::num::NonZeroU8;
use std::ops::{Deref, RangeInclusive};

fn take_three<I: Iterator<Item = Result<T, E>>, T, E>(
    mut iter: I,
    on_missing: impl FnOnce() -> E,
) -> Result<[T; 3], E> {
    let val0 = match iter.next() {
        Some(val0) => val0?,
        None => return Err(on_missing()),
    };
    let val1 = match iter.next() {
        Some(val1) => val1?,
        None => return Err(on_missing()),
    };
    let val2 = match iter.next() {
        Some(val2) => val2?,
        None => return Err(on_missing()),
    };
    if iter.next().is_some() {
        Err(on_missing())
    } else {
        Ok([val0, val1, val2])
    }
}

fn take_four<I: Iterator<Item = Result<T, E>>, T, E>(
    mut iter: I,
    on_missing: impl FnOnce() -> E,
) -> Result<[T; 4], E> {
    let val0 = match iter.next() {
        Some(val0) => val0?,
        None => return Err(on_missing()),
    };
    let val1 = match iter.next() {
        Some(val1) => val1?,
        None => return Err(on_missing()),
    };
    let val2 = match iter.next() {
        Some(val2) => val2?,
        None => return Err(on_missing()),
    };
    let val3 = match iter.next() {
        Some(val3) => val3?,
        None => return Err(on_missing()),
    };
    if iter.next().is_some() {
        Err(on_missing())
    } else {
        Ok([val0, val1, val2, val3])
    }
}

const ALLOWED_NAME_RANGES: &[RangeInclusive<char>] = &[
    ' '..=' ',
    '"'..='#',
    '%'..='%',
    '\''..='+',
    '-'..='-',
    '/'..='>',
    '@'..='Z',
    '_'..='z',
];

fn is_invalid_name_char(ch: char) -> bool {
    let is_valid = ALLOWED_NAME_RANGES.iter().any(|range| range.contains(&ch));
    !is_valid
}

/// A unique object name.
///
/// The allowed characters are listed in [Annex C](https://gdtf.eu/gdtf/annex/annex-c/) of the GDTF
/// reference.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    /// Attempts to construct a [Name] from a string value.
    ///
    /// This function will fail if the string is not entirely made up of the
    /// [allowed characters](https://gdtf.eu/gdtf/annex/annex-c/). On failure the original string
    /// value is returned.
    pub fn new<T>(value: T) -> Result<Self, String>
    where
        T: Into<String>,
    {
        let value = value.into();
        let is_ok = !value.contains(is_invalid_name_char);
        if is_ok { Ok(Name(value)) } else { Err(value) }
    }

    /// Constructs a [Name] from a string value, replacing
    /// [invalid characters](https://gdtf.eu/gdtf/annex/annex-c/) with `_`.
    pub fn new_lossy<T>(value: T) -> Self
    where
        T: Into<String>,
    {
        let mut value = value.into();
        while let Some((index, ch)) = value
            .char_indices()
            .find(|(_, ch)| is_invalid_name_char(*ch))
        {
            value.replace_range(index..(index + ch.len_utf8()), "_");
        }
        Name(value)
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Name> for String {
    fn from(value: Name) -> Self {
        value.0
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl TryFrom<String> for Name {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Name::new(value).map_err(|_| ())
    }
}

impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NameVisitor;

        impl<'de> Visitor<'de> for NameVisitor {
            type Value = Name;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a non-empty name string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_string(v.to_string())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Name::new_lossy(v))
            }
        }

        deserializer.deserialize_string(NameVisitor)
    }
}

/// A link to an element.
///
/// Node links are made of a series of [Name]s which form a path of object names. THe path starting
/// point depends on where the node link is used.
///
/// Serialized format:
/// ```text
/// Name.Name.Name...
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node(Vec<Name>);

impl Node {
    /// Construct a [Node](super::values::Node) from a list of [Name]s.
    pub fn new(names: impl Into<Vec<Name>>) -> Self {
        Node(names.into())
    }
}

impl Deref for Node {
    type Target = [Name];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[Name]> for Node {
    fn as_ref(&self) -> &[Name] {
        &self.0
    }
}

impl From<Node> for Vec<Name> {
    fn from(value: Node) -> Self {
        value.0
    }
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.iter().join("."))
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NodeVisitor;

        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = Node;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a node reference in the format Name.Name.Name...")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let names: Result<Vec<_>, _> = v
                    .split('.')
                    .map(|item| Name::deserialize(StrDeserializer::new(item)))
                    .collect();
                names.map(Node)
            }
        }

        deserializer.deserialize_str(NodeVisitor)
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

pub(crate) trait NodeExt {
    fn single(&self) -> Option<&Name>;
}

impl NodeExt for [Name] {
    fn single(&self) -> Option<&Name> {
        if self.len() > 1 {
            return None;
        }
        self.first()
    }
}

/// CIE color representation xyY 1931.
///
/// Serialized format:
/// ```text
/// x,y,Y
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ColorCie {
    /// CIE x value
    pub x: f64,

    /// CIE y value
    pub y: f64,

    /// CIE Y value
    pub z: f64,

    /// Set when the source text did not match the ColorCIE grammar and this
    /// value is the substituted white point rather than authored data. Read it
    /// through [`ColorCie::is_substituted`].
    substituted: bool,
}

/// Provenance is metadata about where a value came from, not part of what the
/// value means, so a substituted white point compares equal to an authored one.
/// [`ColorCie::is_substituted`] is the only thing that separates them.
impl PartialEq for ColorCie {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl Serialize for ColorCie {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{},{},{}", self.x, self.y, self.z))
    }
}

impl<'de> Deserialize<'de> for ColorCie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorCIEVisitor;

        impl<'de> Visitor<'de> for ColorCIEVisitor {
            type Value = ColorCie;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a color string in the format x,y,Y")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // A value that does not match the grammar degrades this one
                // element; it never fails the archive. One malformed `Color=`
                // costs a whole fixture otherwise, and the attribute is
                // usually one the reader never looks at.
                //
                // The substitute is the white point, matching both reference
                // implementations: libMVRgdtf falls back to a
                // default-constructed `CCieColor` (its "pure white") and
                // pygdtf's `ColorCIE` except-branch falls back to the same.
                let fields = v
                    .split(',')
                    .map(|field_str| field_str.trim().parse::<f64>().ok());
                let Ok([Some(x), Some(y), Some(z)]) = take_three(fields.map(Ok::<_, ()>), || ())
                else {
                    return Ok(ColorCie::substitute());
                };
                // GDTF types each component as a Float and the published XSD
                // pins it to `-?[0-9]+(\.[0-9]+)?`, so NaN and the infinities
                // are not ColorCIE however they are spelled. Rust, C and
                // Python all parse a bare `nan` as a float, so without this a
                // NaN authored by a broken builder reaches the colour path
                // silently.
                if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                    return Ok(ColorCie::substitute());
                }
                Ok(ColorCie::new(x, y, z))
            }
        }

        deserializer.deserialize_str(ColorCIEVisitor)
    }
}

impl ColorCie {
    /// Constructs a color from its CIE xyY components.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        ColorCie {
            x,
            y,
            z,
            substituted: false,
        }
    }

    /// Constructs a pure white color.
    ///
    /// Pure white is represented with the values x=0.3127, y=0.3290, Y=100.
    pub const fn white() -> Self {
        ColorCie::new(0.3127, 0.3290, 100.)
    }

    /// The white point, marked as standing in for source text that did not
    /// match the ColorCIE grammar.
    const fn substitute() -> Self {
        ColorCie {
            substituted: true,
            ..ColorCie::white()
        }
    }

    /// Whether this value was substituted by the parser because the source
    /// text did not match the ColorCIE grammar.
    ///
    /// A substituted color is the white point of [`ColorCie::white`]; the
    /// source text it replaced is not retained. The corresponding
    /// [`ValidationError`](crate::ValidationError) names the element it sits
    /// on.
    pub const fn is_substituted(self) -> bool {
        self.substituted
    }
}

/// Vector with 3 float components.
///
/// Serialized format:
/// ```text
/// {x,y,z}
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3([f64; 3]);

impl Vector3 {
    /// Constructs a zero-sized vector.
    pub const fn zero() -> Self {
        Vector3([0., 0., 0.])
    }
}

impl Serialize for Vector3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{{{},{},{}}}", self.0[0], self.0[1], self.0[2]))
    }
}

impl<'de> Deserialize<'de> for Vector3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Vector3Visitor;

        impl<'de> Visitor<'de> for Vector3Visitor {
            type Value = Vector3;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a vector3 string in the format {float,float,float}")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let Some(without_tails) = v.strip_prefix('{').and_then(|v| v.strip_suffix('}'))
                else {
                    return Err(E::invalid_value(Unexpected::Str(v), &self));
                };

                let fields = without_tails.split(',').map(|field_str| {
                    field_str.trim().parse::<f64>().map_err(|_| {
                        E::invalid_value(Unexpected::Str(field_str), &"a floating point number")
                    })
                });
                let values = take_three(fields, || E::invalid_value(Unexpected::Str(v), &self))?;
                Ok(Vector3(values))
            }
        }

        deserializer.deserialize_str(Vector3Visitor)
    }
}

/// A transform matrix consisting of 4 x 4 floats.
///
/// The metric system consists of the right-handed cartesian coordinates XYZ:
///  - X - from left (-X) to right (+X)
///  - Y - from the outside of the monitor (-Y) to the inside of the monitor (+Y)
///  - Z - from bottom (-Z) to top (+Z)
///  - 0,0,0 - center base.
///
/// Serialized format:
/// ```text
/// {float,float,float,float}
/// {float,float,float,float}
/// {float,float,float,float}
/// {float,float,float,float}
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix([[f64; 4]; 4]);

impl Matrix {
    /// Constructs a matrix representing an identity transform.
    pub const fn identity() -> Self {
        Matrix([
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ])
    }
}

impl Serialize for Matrix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!(
            "{{{},{},{},{}}}{{{},{},{},{}}}{{{},{},{},{}}}{{{},{},{},{}}}",
            self.0[0][0],
            self.0[0][1],
            self.0[0][2],
            self.0[0][3],
            self.0[1][0],
            self.0[1][1],
            self.0[1][2],
            self.0[1][3],
            self.0[2][0],
            self.0[2][1],
            self.0[2][2],
            self.0[2][3],
            self.0[3][0],
            self.0[3][1],
            self.0[3][2],
            self.0[3][3]
        ))
    }
}

impl<'de> Deserialize<'de> for Matrix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MatrixVisitor;

        impl<'de> Visitor<'de> for MatrixVisitor {
            type Value = Matrix;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a matrix string in the format {float,float,float,float}{float,float,float,float}{float,float,float,float}{float,float,float,float}")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let Some(without_start) = v.strip_prefix('{') else {
                    return Err(E::invalid_value(Unexpected::Str(v), &self));
                };

                let rows = without_start.split('{').map(|row_str| {
                    let Some(without_end) = row_str.strip_suffix('}') else {
                        return Err(E::invalid_value(Unexpected::Str(v), &self));
                    };

                    let columns = without_end.split(',').map(|column_str| {
                        column_str.trim().parse::<f64>().map_err(|_| {
                            E::invalid_value(
                                Unexpected::Str(column_str),
                                &"a floating point number",
                            )
                        })
                    });
                    take_four(columns, || E::invalid_value(Unexpected::Str(v), &self))
                });
                let row_values = take_four(rows, || E::invalid_value(Unexpected::Str(v), &self))?;
                Ok(Matrix(row_values))
            }
        }

        deserializer.deserialize_str(MatrixVisitor)
    }
}

/// A rotation matrix consisting of 3 x 3 floats.
///
/// The metric system consists of the right-handed cartesian coordinates XYZ:
///  - X - from left (-X) to right (+X)
///  - Y - from the outside of the monitor (-Y) to the inside of the monitor (+Y)
///  - Z - from bottom (-Z) to top (+Z)
///
/// Serialized format:
/// ```text
/// {float,float,float}
/// {float,float,float}
/// {float,float,float}
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation([[f64; 3]; 3]);

impl Rotation {
    /// Constructs a matrix representing an identity rotation.
    pub const fn identity() -> Self {
        Rotation([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
    }
}

impl Serialize for Rotation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!(
            "{{{},{},{}}}{{{},{},{}}}{{{},{},{}}}",
            self.0[0][0],
            self.0[0][1],
            self.0[0][2],
            self.0[1][0],
            self.0[1][1],
            self.0[1][2],
            self.0[2][0],
            self.0[2][1],
            self.0[2][2]
        ))
    }
}

impl<'de> Deserialize<'de> for Rotation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RotationVisitor;

        impl<'de> Visitor<'de> for RotationVisitor {
            type Value = Rotation;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a rotation string in the format {float,float,float}{float,float,float}{float,float,float}")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let Some(without_start) = v.strip_prefix('{') else {
                    return Err(E::invalid_value(Unexpected::Str(v), &self));
                };

                let rows = without_start.split('{').map(|row_str| {
                    let Some(without_end) = row_str.strip_suffix('}') else {
                        return Err(E::invalid_value(Unexpected::Str(v), &self));
                    };

                    let columns = without_end.split(',').map(|column_str| {
                        column_str.trim().parse::<f64>().map_err(|_| {
                            E::invalid_value(
                                Unexpected::Str(column_str),
                                &"a floating point number",
                            )
                        })
                    });
                    take_three(columns, || E::invalid_value(Unexpected::Str(v), &self))
                });
                let row_values = take_three(rows, || E::invalid_value(Unexpected::Str(v), &self))?;
                Ok(Rotation(row_values))
            }
        }

        deserializer.deserialize_str(RotationVisitor)
    }
}

/// A DMX address value consisting of a universe and an address.
///
/// There are 2^23 universes and 2^9 (512) addresses, combined making 32 bits of information.
/// Universe and address numbers are conventionally ordered starting from 1.
///
/// Serialized format:
/// ```text
/// Absolute format:  <int>
/// Alternate format: Universe.Address
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DmxAddress(u32);

impl Serialize for DmxAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}.{}", self.universe(), self.address()))
    }
}

impl<'de> Deserialize<'de> for DmxAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DMXAddressVisitor;

        impl<'de> Visitor<'de> for DMXAddressVisitor {
            type Value = DmxAddress;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "an integer between 0 and 2^32 or an address in the format Universe.Address",
                )
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DmxAddress(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v.split_once('.') {
                    Some((universe_str, address_str)) => {
                        let universe = universe_str.trim().parse::<u32>().map_err(|_| {
                            E::invalid_value(
                                Unexpected::Str(universe_str),
                                &"an integer between 0 and 2^32",
                            )
                        })?;
                        let address = address_str.trim().parse::<u32>().map_err(|_| {
                            E::invalid_value(
                                Unexpected::Str(address_str),
                                &"an integer between 0 and 2^32",
                            )
                        })?;
                        Ok(DmxAddress::from_universe_address(universe, address))
                    }
                    None => v
                        .trim()
                        .parse::<u32>()
                        .map(DmxAddress::from_absolute)
                        .map_err(|_| E::invalid_value(Unexpected::Str(v), &self)),
                }
            }
        }

        deserializer.deserialize_str(DMXAddressVisitor)
    }
}

impl DmxAddress {
    /// Constructs a [DmxAddress] from a single absolute value.
    ///
    /// Absolute values contain both the universe and address. The 23 highest bits are the universe,
    /// and the 9 lowest bits are the address.
    pub fn from_absolute(value: u32) -> Self {
        DmxAddress(value)
    }

    /// Constructs a [DmxAddress] from a separate universe and address.
    ///
    /// The universe is expected to be in the range 1..2^23, and the address is expected to be in
    /// the range 1..2^9.
    pub fn from_universe_address(universe: u32, address: u32) -> Self {
        DmxAddress(((universe.saturating_sub(1)) << 9) + address.saturating_sub(1).max((2 ^ 9) - 1))
    }

    /// Returns the address as an absolute value.
    ///
    /// Absolute values contain both the universe and address. The 23 highest bits are the universe,
    /// and the 9 lowest bits are the address.
    pub fn absolute(self) -> u32 {
        self.0
    }

    /// Returns the universe part of the [DmxAddress].
    ///
    /// The universe is an integer in the range 1..2^23.
    pub fn universe(self) -> u32 {
        (self.0 >> 9) + 1
    }

    /// Returns the address part of the [DmxAddress].
    ///
    /// The address is an integer in the range 1..2^9.
    pub fn address(self) -> u32 {
        (self.0 & ((2 ^ 9) - 1)) + 1
    }
}

/// The value of a DMX parameter.
///
/// This type defines both a value and a byte count, without depending on the resolution of the
/// channel the value is applied to.
///
/// When converting a [DmxValue] to the value for a channel, one of two conventions are used:
///  - If the value has the `shifting` flag, bytes in the value are shifted to the new size.
///    For example `255/1s` in a 16 bit channel will result in `65280`.
///  - Otherwise, bytes in the value are mirrored to the new size. For example `255/1` in a 16 bit
///    channel will result in `65535`.
///
/// Serialized format:
/// ```text
/// Mirroring format: value/n
/// Shifting format: value/ns
/// ```
#[derive(Debug, Clone, Copy)]
pub struct DmxValue {
    bytes: NonZeroU8,
    value: u64,
    shifting: bool,
    substituted: bool,
}

/// Provenance is metadata about where a value came from, not part of what the
/// value means, so a substituted `0/1` addresses the same DMX slot as an
/// authored `0/1` and compares and hashes equal to it.
/// [`DmxValue::is_substituted`] is the only thing that separates them.
impl PartialEq for DmxValue {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes && self.value == other.value && self.shifting == other.shifting
    }
}

impl Eq for DmxValue {}

impl std::hash::Hash for DmxValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
        self.value.hash(state);
        self.shifting.hash(state);
    }
}

impl DmxValue {
    /// Whether this value was substituted by the parser because the source
    /// text did not match the `Uint/n` DMXValue grammar.
    ///
    /// A substituted value is `0/1`, the default the GDTF specification
    /// declares for `ChannelSet@DMXFrom`; the source text it replaced is not
    /// retained. The corresponding [`ValidationError`](crate::ValidationError)
    /// names the element it sits on.
    pub const fn is_substituted(self) -> bool {
        self.substituted
    }

    /// The number of bytes of the associated `value`.
    pub const fn bytes(self) -> NonZeroU8 {
        self.bytes
    }

    /// The concrete value.
    ///
    /// It should be assumed this value is actually of the size defined by `bytes`.
    pub const fn value(self) -> u64 {
        self.value
    }

    /// Whether to shift the `value` when converting to a larger byte size.
    pub const fn shifting(self) -> bool {
        self.shifting
    }

    /// Attempts to construct a DmxValue.
    ///
    /// Construction will fail if the value is larger than the maximum value that can fit into
    /// the provided number of bytes.
    pub const fn new(value: u64, bytes: NonZeroU8, shifting: bool) -> Option<Self> {
        let bits = bytes.get() as u32 * 8;
        let max_value = 2u64.saturating_pow(bits);
        if value > max_value {
            None
        } else {
            Some(DmxValue {
                value,
                bytes,
                shifting,
                substituted: false,
            })
        }
    }

    /// `0/1`, marked as standing in for source text that did not match the
    /// `Uint/n` DMXValue grammar.
    const fn substitute() -> Self {
        DmxValue {
            substituted: true,
            ..DmxValue::new_u8(0, false)
        }
    }

    /// Construct a DmxValue from an 8-bit integer.
    pub const fn new_u8(value: u8, shifting: bool) -> Self {
        DmxValue {
            value: value as u64,
            bytes: unsafe { NonZeroU8::new_unchecked(1) },
            shifting,
            substituted: false,
        }
    }

    /// Construct a DmxValue from a 16-bit integer.
    pub const fn new_u16(value: u16, shifting: bool) -> Self {
        DmxValue {
            value: value as u64,
            bytes: unsafe { NonZeroU8::new_unchecked(2) },
            shifting,
            substituted: false,
        }
    }

    /// Construct a DmxValue from a 32-bit integer.
    pub const fn new_u32(value: u32, shifting: bool) -> Self {
        DmxValue {
            value: value as u64,
            bytes: unsafe { NonZeroU8::new_unchecked(4) },
            shifting,
            substituted: false,
        }
    }

    /// Construct a DmxValue from a 64-bit integer.
    pub const fn new_u64(value: u64, shifting: bool) -> Self {
        DmxValue {
            value,
            bytes: unsafe { NonZeroU8::new_unchecked(8) },
            shifting,
            substituted: false,
        }
    }

    /// Convert a DmxValue to a number with the provided number of bytes.
    pub fn to(self, bytes: u8) -> u64 {
        let my_bytes = self.bytes.get();
        if bytes == 0 {
            0
        } else if my_bytes == bytes {
            self.value
        } else if bytes < my_bytes {
            // shrinking case: always take most significant bytes
            let byte_diff = my_bytes - bytes;
            let bit_diff = byte_diff as u32 * 8;
            self.value >> bit_diff
        } else {
            // expanding case
            let byte_diff = bytes - my_bytes;
            let bit_diff = byte_diff as u32 * 8;
            let mut value = self.value << bit_diff;

            // if not shifting: copy the value multiple times
            // e.g. (with decimal)
            // value now = 1234000000
            // becomes     1234123412
            if !self.shifting {
                let repeat_count = byte_diff.div_ceil(my_bytes);
                for repeat in 0..repeat_count {
                    let shift_bytes_inv = repeat * my_bytes;
                    let shift_bits_inv = shift_bytes_inv as u32 * 8;

                    if shift_bits_inv > bit_diff {
                        value |= self.value >> (shift_bits_inv - bit_diff);
                    } else {
                        value |= self.value << (bit_diff - shift_bits_inv);
                    }
                }
            }

            value
        }
    }

    /// Convert a DmxValue to an 8-bit integer.
    pub fn to_u8(self) -> u8 {
        self.to(1) as u8
    }

    /// Convert a DmxValue to a 16-bit integer.
    pub fn to_u16(self) -> u16 {
        self.to(2) as u16
    }

    /// Convert a DmxValue to a 32-bit integer.
    pub fn to_u32(self) -> u32 {
        self.to(4) as u32
    }

    /// Convert a DmxValue to a 64-bit integer.
    pub fn to_u64(self) -> u64 {
        self.to(8)
    }
}

impl Serialize for DmxValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let postfix = if self.shifting { "s" } else { "" };
        serializer.serialize_str(&format!("{}/{}{}", self.value, self.bytes, postfix))
    }
}

impl<'de> Deserialize<'de> for DmxValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DMXValueVisitor;

        impl<'de> Visitor<'de> for DMXValueVisitor {
            type Value = DmxValue;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a DMX value in the format uint/n or uint/ns")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // A value that does not match the `Uint/n` grammar degrades
                // this one element; it never fails the archive. One malformed
                // `DMXFrom=` costs a whole fixture otherwise, and it is
                // usually on a channel set the reader never looks at.
                //
                // The substitute is `0/1`, which is both the default the GDTF
                // specification declares for `ChannelSet@DMXFrom` and what
                // libMVRgdtf writes when its own conversion rejects the text
                // (`dmxValueRaw = 0; byteSpecifier = 1;` in
                // GdtfConverter::ConvertDMXValue, which records an
                // `eValueError_DmxValueHasWrongValue` and parses on).
                let Some((value_str, bytes_str)) = v.split_once('/') else {
                    return Ok(DmxValue::substitute());
                };

                let (bytes_str, shifting) = match bytes_str.strip_suffix('s') {
                    Some(stripped) => (stripped, true),
                    None => (bytes_str, false),
                };

                let (Ok(value), Ok(bytes)) = (value_str.parse::<u64>(), bytes_str.parse::<u8>())
                else {
                    return Ok(DmxValue::substitute());
                };

                let bytes = bytes.max(1);
                let bits = bytes as u32 * 8;
                let max_value = 2u64.saturating_pow(bits);
                let value = value.min(max_value);

                Ok(DmxValue {
                    value,
                    bytes: NonZeroU8::new(bytes).unwrap(),
                    shifting,
                    substituted: false,
                })
            }
        }

        deserializer.deserialize_str(DMXValueVisitor)
    }
}

impl Default for DmxValue {
    fn default() -> Self {
        DmxValue {
            value: 0,
            bytes: NonZeroU8::new(1).unwrap(),
            shifting: false,
            substituted: false,
        }
    }
}

impl From<DmxValue> for u8 {
    fn from(value: DmxValue) -> Self {
        value.to_u8()
    }
}

impl From<DmxValue> for u16 {
    fn from(value: DmxValue) -> Self {
        value.to_u16()
    }
}

impl From<DmxValue> for u32 {
    fn from(value: DmxValue) -> Self {
        value.to_u32()
    }
}

impl From<DmxValue> for u64 {
    fn from(value: DmxValue) -> Self {
        value.to_u64()
    }
}

/// A version value consisting of a major and minor number.
///
/// Supported versions: `1.0`, `1.1`, `1.2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    /// Major version number.
    pub major: u8,

    /// Minor version number.
    pub minor: u8,
}

impl Version {
    /// Returns if this version is supported by the parser.
    pub fn is_supported(self) -> bool {
        self.major == 1 && self.minor <= 2
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}.{}", self.major, self.minor))
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionVisitor;

        impl<'de> Visitor<'de> for VersionVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a version value in the format uint.uint")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let Some((major_str, minor_str)) = v.split_once('.') else {
                    return Err(E::invalid_value(Unexpected::Str(v), &self));
                };

                let major = major_str.parse::<u8>().map_err(|_| {
                    E::invalid_value(Unexpected::Str(major_str), &"an integer between 0 and 2^8")
                })?;
                let minor = minor_str.parse::<u8>().map_err(|_| {
                    E::invalid_value(Unexpected::Str(minor_str), &"an integer between 0 and 2^8")
                })?;

                Ok(Version { major, minor })
            }
        }

        deserializer.deserialize_str(VersionVisitor)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

pub(crate) fn ok_or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(T::deserialize(deserializer).unwrap_or_default())
}

pub(crate) fn non_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;
    Ok(if str.is_empty() { None } else { Some(str) })
}

#[cfg(test)]
mod tests {
    use super::{ColorCie, DmxValue};
    use serde::Deserialize;
    use serde::de::value::{Error as ValueError, StrDeserializer};

    fn dmx(text: &str) -> DmxValue {
        DmxValue::deserialize(StrDeserializer::<ValueError>::new(text))
            .expect("a malformed DMXValue degrades to a substitute, it never fails the parse")
    }

    fn color(text: &str) -> ColorCie {
        ColorCie::deserialize(StrDeserializer::<ValueError>::new(text))
            .expect("a malformed ColorCIE degrades to a substitute, it never fails the parse")
    }

    // GDTF 1.2 (DIN SPEC 15800:2022) types `DMXValue` as `Uint/n`, and the
    // published XSD pins the grammar to `None|([0-9]+/[1-9][0-9]*s?)`. A
    // leading `-` is therefore not a DMXValue at all.
    //
    // The substituted value is `0/1`: that is both the spec's declared default
    // for `ChannelSet@DMXFrom` and what libMVRgdtf writes when its own
    // conversion rejects the text (`dmxValueRaw = 0; byteSpecifier = 1;` in
    // GdtfConverter::ConvertDMXValue).
    #[test]
    fn a_signed_dmx_value_is_substituted_with_the_spec_default() {
        let value = dmx("-255/4");
        assert_eq!(value.value(), 0, "substitute is the spec default 0/1");
        assert_eq!(value.bytes().get(), 1, "substitute is the spec default 0/1");
        assert!(!value.shifting(), "the spec default is byte-mirroring");
        assert!(
            value.is_substituted(),
            "the substitution must be recorded on the value, not swallowed"
        );
    }

    #[test]
    fn a_dmx_value_without_a_byte_count_is_substituted() {
        let value = dmx("255");
        assert_eq!((value.value(), value.bytes().get()), (0, 1));
        assert!(value.is_substituted());
    }

    #[test]
    fn a_dmx_value_with_a_non_numeric_byte_count_is_substituted() {
        let value = dmx("128/x");
        assert_eq!((value.value(), value.bytes().get()), (0, 1));
        assert!(value.is_substituted());
    }

    #[test]
    fn a_well_formed_dmx_value_is_untouched_and_unmarked() {
        let value = dmx("65535/2");
        assert_eq!((value.value(), value.bytes().get()), (65535, 2));
        assert!(!value.shifting());
        assert!(
            !value.is_substituted(),
            "a value that matched the grammar must not be reported as substituted"
        );
    }

    #[test]
    fn a_well_formed_shifting_dmx_value_is_untouched_and_unmarked() {
        let value = dmx("255/1s");
        assert_eq!((value.value(), value.bytes().get()), (255, 1));
        assert!(value.shifting());
        assert!(!value.is_substituted());
    }

    // Provenance is not part of the value. A substituted `0/1` addresses the
    // same DMX slot as an authored `0/1`, so equality and hashing must not
    // split them; only `is_substituted` distinguishes them.
    #[test]
    fn the_substitution_marker_is_not_part_of_dmx_value_equality() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let substituted = dmx("-255/4");
        let authored = dmx("0/1");
        assert_ne!(substituted.is_substituted(), authored.is_substituted());
        assert_eq!(
            substituted, authored,
            "provenance must not change what the value means"
        );

        let hash = |v: DmxValue| {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(substituted), hash(authored));
    }

    // `-nan(ind)` is MSVC's rendering of an indefinite NaN (documented in
    // Microsoft's printf format specification), emitted when a fixture builder
    // formats a NaN it computed. GDTF types `ColorCIE` as three floats and the
    // XSD pins each to `-?[0-9]+(\.[0-9]+)?`, so it is not a ColorCIE.
    //
    // The substitute is the spec's own white point, `{0.3127, 0.3290, 100.0}`.
    // Both reference parsers agree: libMVRgdtf falls back to a
    // default-constructed `CCieColor` (its stated "pure white") and pygdtf's
    // `ColorCIE` except-branch falls back to the same white.
    #[test]
    fn an_msvc_indefinite_nan_color_is_substituted_with_the_white_point() {
        let value = color("-nan(ind),-nan(ind),-nan(ind)");
        assert_eq!((value.x, value.y, value.z), (0.3127, 0.3290, 100.0));
        assert!(
            value.is_substituted(),
            "the substitution must be recorded on the value, not swallowed"
        );
    }

    // The bare `nan` spelling parses as a float in Rust, C and Python, so every
    // parser accepts it silently and a NaN reaches the colour path. It is no
    // more a ColorCIE than `-nan(ind)` is, and it comes from the same authoring
    // bug, so it degrades the same way rather than propagating a NaN.
    #[test]
    fn a_non_finite_color_component_is_substituted_with_the_white_point() {
        for text in ["nan,nan,nan", "0.3,nan,100", "inf,0.3,100", "0.3,0.3,-inf"] {
            let value = color(text);
            assert_eq!(
                (value.x, value.y, value.z),
                (0.3127, 0.3290, 100.0),
                "{text} carries a non-finite component and is not a ColorCIE"
            );
            assert!(value.is_substituted(), "{text} must be recorded");
        }
    }

    #[test]
    fn a_color_with_the_wrong_arity_is_substituted_with_the_white_point() {
        for text in ["0.3,0.3", "", "garbage"] {
            let value = color(text);
            assert_eq!((value.x, value.y, value.z), (0.3127, 0.3290, 100.0));
            assert!(value.is_substituted(), "{text} must be recorded");
        }
    }

    #[test]
    fn a_well_formed_color_is_untouched_and_unmarked() {
        let value = color("0.64,0.33,21.262754");
        assert_eq!((value.x, value.y, value.z), (0.64, 0.33, 21.262754));
        assert!(
            !value.is_substituted(),
            "a value that matched the grammar must not be reported as substituted"
        );
    }

    #[test]
    fn the_substitution_marker_is_not_part_of_color_equality() {
        let substituted = color("-nan(ind),-nan(ind),-nan(ind)");
        let authored = color("0.3127,0.3290,100.0");
        assert_ne!(substituted.is_substituted(), authored.is_substituted());
        assert_eq!(
            substituted, authored,
            "provenance must not change what the value means"
        );
    }
}
