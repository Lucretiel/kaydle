/*!
Parsers and types related to annotations, which are optional string tags that
can precede nodes and values. Usually used for type hinting, especially in
dynamic languages.
*/

use std::char::CharTryFromError;

use nom::{
    branch::alt,
    character::complete::char,
    combinator::success,
    error::{ContextError, FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{tag::TagError, ParserExt};

use crate::{
    string::{parse_identifier, KdlString, StringBuilder},
    value::KdlValue,
};

/// Parse an annotation, which is an identifier enclosed in parentheses.
pub fn parse_annotation<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
    T: StringBuilder<'i>,
{
    parse_identifier
        .terminated(char(')'))
        .cut()
        .preceded_by(char('('))
        .parse(input)
}

/// Trait for annotations. An annotation is essentially an optional string;
/// this trait allows for abstracting over cases where the caller doesn't care
/// about the annotation, or only cares about the *presence* of an annotation.
/// Used as the return type for [`parse_maybe_annotation`].
pub trait AnnotationBuilder<'i> {
    /// String type for the annotation
    type String: StringBuilder<'i>;

    /// There was no annotation
    #[must_use]
    fn absent() -> Self;

    /// There was an annotation
    #[must_use]
    fn annotated(annotation: Self::String) -> Self;
}

/// The unit type can be used as an annotation type in cases where the caller
/// doesn't care about the presence or value of an annotation.
impl<'i> AnnotationBuilder<'i> for () {
    type String = ();

    #[must_use]
    #[inline]
    fn absent() -> Self {}

    #[must_use]
    #[inline]
    fn annotated(_annotation: Self::String) -> Self {}
}

impl<'i, S: StringBuilder<'i>> AnnotationBuilder<'i> for Option<S> {
    type String = S;

    #[must_use]
    #[inline]
    fn absent() -> Self {
        None
    }

    #[must_use]
    #[inline]
    fn annotated(annotation: Self::String) -> Self {
        Some(annotation)
    }
}

/// Try to parse an annotation, but succeed if there is none present. Uses
/// [`AnnotationBuilder`] as a return type. Returns an error if the opening
/// parenthesis exists but an error occurred inside.
pub fn parse_maybe_annotation<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
    T: AnnotationBuilder<'i>,
{
    alt((
        parse_annotation.map(T::annotated),
        success(()).map(|()| T::absent()),
    ))
    .parse(input)
}

/// An annotated object of some kind. Contains some `item` as well as an
/// associated annotation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GenericAnnotated<A, T> {
    /// The annotation
    pub annotation: A,

    /// The thing being annotated
    pub item: T,
}

impl<A, T> GenericAnnotated<A, T> {
    /// Apply a function to the annotated item [`item`][Self::item]
    pub fn map_item<U>(self, op: impl FnOnce(T) -> U) -> GenericAnnotated<A, U> {
        GenericAnnotated {
            item: op(self.item),
            annotation: self.annotation,
        }
    }
}

/// A recognized annotated doesn't contain any data; it's used in cases where
/// the caller wants to parse and discard something with an annotation.
pub type RecognizedAnnotated = GenericAnnotated<(), ()>;

/// A normal annotated value uses an `Option<KdlString>` as its annotation
/// type.
pub type Annotated<'i, T> = GenericAnnotated<Option<KdlString<'i>>, T>;

/// An annotated [`KdlValue`].
pub type AnnotatedValue<'i> = Annotated<'i, KdlValue<'i>>;

/// A recognized annotation only contains the annotated item; the annotation
/// itself is ignored. Used in cases where the caller wants to parse and
/// discard the annotation before an item.
pub type RecognizedAnnotation<T> = GenericAnnotated<(), T>;

/// A recognized annotation value is a normal [`KdlValue`] with an ignored
/// annotation.
pub type RecognizedAnnotationValue<'i> = RecognizedAnnotation<KdlValue<'i>>;

/// Modify a parser to include an optional preceding annotation, parsing it
/// and the value itself into a [`GenericAnnotated`].
pub fn with_annotation<'i, P, T, A, E>(parser: P) -> impl Parser<&'i str, GenericAnnotated<A, T>, E>
where
    A: AnnotationBuilder<'i>,
    P: Parser<&'i str, T, E>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    parse_maybe_annotation
        .context("annotation")
        .and(parser)
        .map(|(annotation, item)| GenericAnnotated { annotation, item })
}

#[cfg(test)]
mod tests {
    use nom::error::Error;
    use nom_supreme::tag::complete::tag;

    use super::*;

    macro_rules! test {
        (
            name: $name:ident,
            type: $anno:ty,
            absent: $absent:expr,
            present: $present:expr,
        ) => {
            mod $name {
                use super::*;

                fn annotated_hello(
                    input: &'static str,
                ) -> IResult<&'static str, GenericAnnotated<$anno, &'static str>, Error<&'static str>>
                {
                    with_annotation(tag("hello")).parse(input)
                }

                #[test]
                fn absent() {
                    let (tail, value) =
                        annotated_hello.parse("hello world").expect("parse failure");

                    assert_eq!(value.item, "hello");
                    assert_eq!(tail, " world");

                    assert_eq!(value.annotation, $absent);
                }

                #[test]
                fn present() {
                    let (tail, value) = annotated_hello
                        .parse("(type)hello world")
                        .expect("parse failure");

                    assert_eq!(value.item, "hello");
                    assert_eq!(tail, " world");

                    assert_eq!(value.annotation, $present)
                }

                #[test]
                fn present_quoted() {
                    let (tail, value) = annotated_hello
                        .parse("(\"type\")hello world")
                        .expect("parse failure");

                    assert_eq!(value.item, "hello");
                    assert_eq!(tail, " world");

                    assert_eq!(value.annotation, $present)
                }

                #[test]
                fn bad() {
                    let _err = annotated_hello
                        .parse("(123)hello")
                        .expect_err("parse success");
                }

                #[test]
                fn bad_item() {
                    let _err = annotated_hello
                        .parse("(type)goodbye")
                        .expect_err("parse_success");
                }
            }
        };
    }

    test! {
        name: empty,
        type: (),
        absent: (),
        present: (),
    }

    test! {
        name: empty_opt,
        type: Option<()>,
        absent: None,
        present: Some(()),
    }

    test! {
        name: string,
        type: Option<String>,
        absent: None,
        present: Some("type".to_owned()),
    }
}
