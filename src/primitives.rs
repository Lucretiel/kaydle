pub mod number;
pub mod string;
pub mod whitespace;
mod syntax;

use nom::{
    branch::alt,
    error::{ParseError}, IResult, Parser,
};
use nom_supreme::{
    tag::{complete::tag, TagError},
    ParserExt,
};

pub fn parse_bool<'i, E>(input: &'i str) -> IResult<&'i str, bool, E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    alt((tag("true").value(true), tag("false").value(false))).parse(input)
}

pub fn parse_null<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    tag("null").value(()).parse(input)
}
