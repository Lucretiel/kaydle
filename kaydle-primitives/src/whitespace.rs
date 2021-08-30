/*!
Various whitespace parsers. We never care about the contents of whitespace so
they always return `()`
*/

use nom::{
    branch::alt,
    character::complete::char,
    combinator::eof,
    error::{make_error, ErrorKind, ParseError},
    Err as NomErr, IResult, Parser,
};
use nom_supreme::{
    tag::{complete::tag, TagError},
    ParserExt,
};

use crate::util::{at_least_one, back};

enum BlockCommentTag {
    Start,
    End,
}

/// Parse the part of a multi line comment that comes after the /*. Operates
/// recursively.
fn finish_block_comment<'i, E>(mut input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
{
    let back = back(input);

    loop {
        let event = input
            .as_bytes()
            .windows(2)
            .enumerate()
            .find_map(|(i, tag)| match tag {
                b"/*" => Some((i, BlockCommentTag::Start)),
                b"*/" => Some((i, BlockCommentTag::End)),
                _ => None,
            });

        match event {
            None => {
                return Err(NomErr::Error(E::or(
                    make_error(back, ErrorKind::Eof),
                    E::from_tag(back, "*/"),
                )))
            }
            Some((i, BlockCommentTag::End)) => return Ok((&input[i + 2..], ())),
            Some((i, BlockCommentTag::Start)) => {
                let (tail, ()) = finish_block_comment(&input[i + 2..])?;
                input = tail;
            }
        }
    }
}

/// Parse a multi line comment, which may be nested.
pub fn parse_block_comment<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
{
    tag("/*").precedes(finish_block_comment).parse(input)
}

/// Parse any amount (1 or more) of plain non-newline whitespace. Includes
/// "real" whitespace, bom, and multiline comments
pub fn parse_plain_whitespace<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    at_least_one(
        alt((
            // Whitespace
            char('\u{0009}'),
            char('\u{0020}'),
            char('\u{00A0}'),
            char('\u{1680}'),
            char('\u{2000}'),
            char('\u{2001}'),
            char('\u{2002}'),
            char('\u{2003}'),
            char('\u{2004}'),
            char('\u{2005}'),
            char('\u{2006}'),
            char('\u{2007}'),
            char('\u{2008}'),
            char('\u{2009}'),
            char('\u{200A}'),
            char('\u{202F}'),
            char('\u{205F}'),
            char('\u{3000}'),
            // BOM
            char('\u{FFEF}'),
        ))
        .value(())
        .or(parse_block_comment),
    )
    .parse(input)
}

/// Parse a single newline
pub fn parse_newline<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
{
    alt((
        tag("\r\n").value('\n'),
        char('\r'),
        char('\n'),
        char('\u{85}'),
        char('\u{0C}'),
        char('\u{2028}'),
        char('\u{2029}'),
    ))
    .value(())
    .parse(input)
}

fn is_newline(c: char) -> bool {
    ['\r', '\n', '\u{85}', '\u{0C}', '\u{2028}', '\u{2029}'].contains(&c)
}

/// Parse a single `//` style comment, terminated by a newline.
pub fn parse_single_line_comment<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
    E: ParseError<&'i str>,
{
    let (input, _) = tag("//").parse(input)?;
    match input
        .char_indices()
        .find_map(|(i, c)| is_newline(c).then(|| i))
    {
        None => Ok((back(input), ())),
        Some(i) => {
            let input = &input[i..];
            parse_newline(input)
        }
    }
}

/// Parse a normal line terminator (newline or single line comment)
pub fn parse_endline<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
    E: ParseError<&'i str>,
{
    alt((parse_newline, parse_single_line_comment)).parse(input)
}

/// Parse 0 or more linespace. Linespace is any endline or plain whitespace
pub fn parse_linespace<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
    E: ParseError<&'i str>,
{
    at_least_one(alt((parse_endline, parse_plain_whitespace)))
        .opt()
        .value(())
        .parse(input)
}

/// Parse a single escline. An escline is an endline that doesn't count as a
/// line terminator (because it's preceeded by an escape), preceeded by 0 or
/// more plain
pub fn parse_escaped_endline<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
    E: ParseError<&'i str>,
{
    char('\\')
        .precedes(parse_plain_whitespace.opt())
        .precedes(parse_endline)
        .parse(input)
}

/// Parse 1 or more nodespace. A nodespace is the whitespace that exists between
/// components of a node; conceptually it's all kinds of non-newline whitespace
pub fn parse_node_space<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
    E: ParseError<&'i str>,
{
    at_least_one(alt((parse_plain_whitespace, parse_escaped_endline))).parse(input)
}

/// Parse a node terminator, which is an endline, eof, or semicolon
pub fn parse_node_terminator<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: TagError<&'i str, &'static str>,
    E: ParseError<&'i str>,
{
    alt((parse_endline, eof.value(()), char(';').value(()))).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test {
        ($test:ident: $parser:ident($input:literal) ok $tail:literal) => {
            #[test]
            fn $test() {
                let res: IResult<&str, (), ()> = $parser($input);
                let (tail, ()) = res.expect("parse failed");
                assert_eq!(tail, $tail);
            }
        };

        ($test:ident: $parser:ident($input:literal) err $location:literal) => {
            #[test]
            fn $test() {
                let res: IResult<&str, (), (&str, nom::error::ErrorKind)> = $parser($input);
                cool_asserts::assert_matches!(res, Err(nom::Err::Error(($location, _))));
            }
        };
    }

    macro_rules! tests {
        ($parser:ident: $(
            $test:ident: $input:literal $state:ident $tail:literal;
        )*) => {
            mod $parser {
                use super::*;

                $(
                    test!{ $test: $parser($input) $state $tail }
                )*
            }
        };
    }

    tests! {
        parse_block_comment:

        basic: "/* abc */ def" ok " def";
        newlines: "/*\nabc\n123*/ def" ok " def";
        nested: "/* abc /* 123 */ def */ 456" ok " 456";

        missing_terminator: "/* 123" err "";

        missing_nested_terminator: "/* 123 /* abc */ def" err "";

        adjacent: "/* 123 */ abc /* 456 */ def" ok " abc /* 456 */ def";
    }
}
