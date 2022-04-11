use std::{
    char::CharTryFromError,
    fmt::{self, Formatter},
};

use nom::{
    branch::alt,
    error::{ContextError, FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{tag::TagError, ParserExt};
use serde::{de, Deserialize, Serialize};

use crate::{
    annotation::{with_annotation, Annotated, AnnotationBuilder},
    number::{parse_number, BoundsError, KdlNumber, NumberBuilder},
    parse_bool, parse_null,
    string::{parse_string, KdlString, StringBuilder},
};

/// An arbitrary KDL Value
#[derive(Debug, Clone)]
pub enum GenericValue<N, S> {
    Null,
    Bool(bool),
    Number(N),
    String(S),
}

pub type KdlValue<'a> = GenericValue<KdlNumber, KdlString<'a>>;

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

pub trait ValueBuilder<'a> {
    type Number: NumberBuilder;
    type String: StringBuilder<'a>;

    fn from_null() -> Self;
    fn from_bool(value: bool) -> Self;
    fn from_number(value: Self::Number) -> Self;
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

impl ValueBuilder<'_> for () {
    type Number = ();
    type String = ();

    fn from_null() {}
    fn from_bool(_value: bool) {}
    fn from_number(_value: Self::Number) {}
    fn from_string(_value: Self::String) {}
}

pub fn parse_value<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: ValueBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: FromExternalError<&'i str, BoundsError>,
    E: ContextError<&'i str>,
{
    alt((
        parse_null.map(|()| T::from_null()).context("null"),
        parse_bool.map(T::from_bool).context("bool"),
        parse_string.map(T::from_string).context("string"),
        parse_number.map(T::from_number).context("number"),
    ))
    .parse(input)
}

pub fn parse_annotated_value<'i, V, A, E>(input: &'i str) -> IResult<&'i str, Annotated<A, V>, E>
where
    V: ValueBuilder<'i>,
    A: AnnotationBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: FromExternalError<&'i str, BoundsError>,
    E: ContextError<&'i str>,
{
    with_annotation(parse_value).parse(input)
}
