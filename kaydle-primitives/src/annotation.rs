use std::char::CharTryFromError;

use nom::{
    character::complete::char,
    error::{ContextError, FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{tag::TagError, ParserExt};

use crate::string::{parse_identifier, StringBuilder};

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

pub trait AnnotationBuilder<'i> {
    type String: StringBuilder<'i>;

    #[must_use]
    fn build(annotation: Option<Self::String>) -> Self;
}

impl<'i> AnnotationBuilder<'i> for () {
    type String = ();

    #[inline]
    #[must_use]
    fn build(_annotation: Option<()>) -> Self {}
}

impl<'i> AnnotationBuilder<'i> for bool {
    type String = ();

    #[inline]
    #[must_use]
    fn build(annotation: Option<()>) -> Self {
        annotation.is_some()
    }
}

impl<'i, S: StringBuilder<'i>> AnnotationBuilder<'i> for Option<S> {
    type String = S;

    #[inline]
    #[must_use]
    fn build(annotation: Option<S>) -> Self {
        annotation
    }
}

/// An annotated object of some kind
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Annotated<A, T> {
    pub annotation: A,
    pub item: T,
}

pub type RecognizedAnnotation = Annotated<(), ()>;

pub fn with_annotation<'i, P, T, A, E>(parser: P) -> impl Parser<&'i str, Annotated<A, T>, E>
where
    A: AnnotationBuilder<'i>,
    P: Parser<&'i str, T, E>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    parse_annotation
        .opt()
        .map(A::build)
        .context("annotation")
        .and(parser)
        .map(|(annotation, item)| Annotated { annotation, item })
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
                ) -> IResult<&'static str, Annotated<$anno, &'static str>, Error<&'static str>>
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
        name: boolean,
        type: bool,
        absent: false,
        present: true,
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
