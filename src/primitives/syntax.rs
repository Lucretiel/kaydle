use nom::{error::ParseError, IResult, Parser};
use nom_supreme::{
    tag::{complete::tag, TagError},
    ParserExt,
};

use super::whitespace;

/// Parse /-, which comments out a single node, prop, value, or children.
/// Parses trailing whitespace.
pub fn parse_commented_node<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
{
    whitespace::parse_whitespace
        .opt()
        .preceded_by(tag("/-"))
        .value(())
        .parse(input)
}
