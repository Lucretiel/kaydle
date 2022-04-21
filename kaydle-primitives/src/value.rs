/*!
Parsers and utility types related to parsing primitive values. Values can
be `null`, `true`, `false`, a number, or a string.
*/

use std::{
    char::CharTryFromError,
    fmt::{self, Formatter},
};

use nom::{
    branch::alt,
    error::{FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{context::ContextError, tag::TagError, ParserExt};
use serde::{de, Deserialize, Serialize};

use crate::{
    annotation::{with_annotation, AnnotationBuilder, GenericAnnotated},
    number::{parse_number, BoundsError, KdlNumber, NumberBuilder},
    parse_bool, parse_null,
    string::{parse_string, KdlString, StringBuilder},
};

/// An arbitrary KDL Value. See also [`AnnotatedValue`][crate::annotation::AnnotatedValue]
/// for a value that includes an optional annotation.
#[derive(Debug, Clone, Copy)]
pub enum GenericValue<N, S> {
    /// `null`
    Null,

    /// `true` or `false`
    Bool(bool),

    /// A number
    Number(N),

    /// A string
    String(S),
}

/// A normal KDL value, containing a [`KdlNumber`] and [`KdlString`].
pub type KdlValue<'a> = GenericValue<KdlNumber, KdlString<'a>>;

/// A recognized KDL value. Used in cases where the caller cares about the type
/// of the value, but not its content; in particular it's intended to allow
/// parsers to avoid the complex runtime costs & allocations of parsing a
/// string or a number in cases where we don't need the value.
pub type RecognizedValue = GenericValue<(), ()>;

impl<'a> KdlValue<'a> {
    /// Apply a KDL value to a visitor
    pub fn visit_to<V, E>(self, visitor: V) -> Result<V::Value, E>
    where
        V: de::Visitor<'a>,
        E: de::Error,
    {
        match self {
            GenericValue::Null => visitor.visit_unit(),
            GenericValue::Bool(value) => visitor.visit_bool(value),
            GenericValue::Number(value) => value.visit_to(visitor),
            GenericValue::String(value) => value.visit_to(visitor),
        }
    }
}

impl Serialize for KdlValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            GenericValue::Null => serializer.serialize_unit(),
            GenericValue::Bool(value) => value.serialize(serializer),
            GenericValue::Number(value) => value.serialize(serializer),
            GenericValue::String(value) => value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for KdlValue<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> de::Visitor<'de> for ValueVisitor {
            type Value = KdlValue<'de>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                write!(formatter, "a KDL value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::Null)
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::Number(KdlNumber::Signed(v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::Number(KdlNumber::Unsigned(v)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::Number(KdlNumber::Float(v)))
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::String(KdlString::from_str(v)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_string(v.to_owned())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlValue::String(KdlString::from_string(v)))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Trait for building primitive KDL values. Used to abstract over cases where
/// the caller might not care about the actual content of the value. Used as
/// the return value for [`parse_bare_value`].
pub trait ValueBuilder<'a> {
    /// The number type used in this value.
    type Number: NumberBuilder;

    /// The string type used in this value.
    type String: StringBuilder<'a>;

    /// Build a KDL value from `null`.
    fn from_null() -> Self;

    /// Build a KDL value from `true` or `false`.
    fn from_bool(value: bool) -> Self;

    /// Build a KDL value from a number.
    fn from_number(value: Self::Number) -> Self;

    /// Build a KDL value from a string.
    fn from_string(value: Self::String) -> Self;
}

impl<'a, N, S> ValueBuilder<'a> for GenericValue<N, S>
where
    N: NumberBuilder,
    S: StringBuilder<'a>,
{
    type Number = N;
    type String = S;

    fn from_null() -> Self {
        Self::Null
    }

    fn from_bool(value: bool) -> Self {
        Self::Bool(value)
    }

    fn from_number(value: N) -> Self {
        Self::Number(value)
    }

    fn from_string(value: S) -> Self {
        Self::String(value)
    }
}

/// The unit type can be used as an annotation type in cases where the caller
/// doesn't care about the actual content of the value.
impl ValueBuilder<'_> for () {
    type Number = ();
    type String = ();

    fn from_null() {}
    fn from_bool(_value: bool) {}
    fn from_number(_value: Self::Number) {}
    fn from_string(_value: Self::String) {}
}

/// Parse any one KDL value. See also [`parse_value`], which includes an
/// annotation.
pub fn parse_bare_value<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: ValueBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: FromExternalError<&'i str, BoundsError>,
    E: ContextError<&'i str, &'static str>,
{
    alt((
        parse_null.map(|()| T::from_null()).context("null"),
        parse_bool.map(T::from_bool).context("bool"),
        parse_string.map(T::from_string).context("string"),
        parse_number.map(T::from_number).context("number"),
    ))
    .parse(input)
}

/// Parse any one KDL value with an optional preceding annotation.
pub fn parse_value<'i, T, A, E>(input: &'i str) -> IResult<&'i str, GenericAnnotated<A, T>, E>
where
    T: ValueBuilder<'i>,
    A: AnnotationBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: FromExternalError<&'i str, BoundsError>,
    E: ContextError<&'i str, &'static str>,
{
    with_annotation(parse_bare_value).parse(input)
}
