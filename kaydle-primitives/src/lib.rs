/*!
This crate contains parsers and other low-level helper types for handling
[KDL](https://kdl.dev/) documents. It is a complete but highly skeletal KDL
implementation, suitable for use by KDL crate authors to provide higher level
access to KDL data. Unless you're working on a KDL implementation, you should
probably not be using this crate.
*/

#![deny(missing_docs)]

use nom::{branch::alt, error::ParseError, IResult, Parser};
use nom_supreme::{
    tag::{complete::tag, TagError},
    ParserExt,
};

pub mod annotation;
pub mod node;
pub mod number;
pub mod property;
pub mod string;
mod util;
pub mod value;
pub mod whitespace;

/// Parse the string `null`
pub fn parse_null<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
{
    tag("null").value(()).parse(input)
}

/// Parse a `true` or `false`
pub fn parse_bool<'i, E>(input: &'i str) -> IResult<&'i str, bool, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
{
    alt((tag("true").value(true), tag("false").value(false))).parse(input)
}
