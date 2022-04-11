/*!
Parser and container type for KDL properties. A property is a key-value pair,
where a key is a KDL identifier and a value is a (possibly annotated) KDL value,
separated by `=`.
 */

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

/// A property, containing a key and potentially annotated value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericProperty<K, A, V> {
    /// The key
    pub key: K,

    /// The value, with its annotation
    pub value: GenericAnnotated<A, V>,
}

/// A normal property, where the key is a [`KdlString`], the value is a
/// [`KdlValue`], and the value's annotation is an `Option<KdlString>`.
pub type Property<'a> = GenericProperty<KdlString<'a>, Option<KdlString<'a>>, KdlValue<'a>>;

/// A Recognized Property is a property that retains no data. It's useful in
/// cases where you want to note that a property has successfully been parsed,
/// but not do any extra work or allocation actually parsing the underlying
/// strings or values.
pub type RecognizedProperty = GenericProperty<(), (), ()>;

/// Parse any KDL property, which is a key-value pair, separated by `=`.
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
