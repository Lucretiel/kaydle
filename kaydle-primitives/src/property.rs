use std::char::CharTryFromError;

use nom::{
    character::complete::char,
    error::{ContextError, FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{tag::TagError, ParserExt};

use crate::{
    annotation::{AnnotationBuilder, GenericAnnotated},
    number::BoundsError,
    string::{parse_identifier, KdlString, StringBuilder},
    value::{parse_value, KdlValue, ValueBuilder},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericProperty<K, A, V> {
    pub key: K,
    pub value: GenericAnnotated<A, V>,
}

pub type Property<'a> = GenericProperty<KdlString<'a>, KdlString<'a>, KdlValue<'a>>;

/// A Recognized Property is a property that retains no data. It's useful in
/// cases where you want to note that a property has successfully been parsed,
/// but not do any extra work or allocation actually parsing the underlying
/// strings or values.
pub type RecognizedProperty = GenericProperty<(), (), ()>;

pub fn parse_property<'i, K, A, V, E>(
    input: &'i str,
) -> IResult<&'i str, GenericProperty<K, A, V>, E>
where
    K: StringBuilder<'i>,
    A: AnnotationBuilder<'i>,
    V: ValueBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: FromExternalError<&'i str, BoundsError>,
    E: ContextError<&'i str>,
{
    parse_identifier
        .context("key")
        .terminated(char('='))
        .and(parse_value.context("value"))
        .map(|(key, value)| GenericProperty { key, value })
        .parse(input)
}
