use nom::{
    branch::alt,
    character::complete::char,
    error::{make_error, ParseError},
    multi::fold_many1,
    Err as NomErr, IResult, Parser,
};
use nom_supreme::{
    tag::{complete::tag, TagError},
    ParserExt,
};

enum BlockCommentTag {
    Start,
    End,
}

// TODO: a non-recursive version

/// Parse the end of a block comment (everything after the /*) with potential
/// nested block comments in it
fn finish_block_comment<'i, E>(mut input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    loop {
        let event = input
            .as_bytes()
            .windows(2)
            .enumerate()
            .find_map(|(i, tag)| match tag {
                b"*/" => Some((i, BlockCommentTag::End)),
                b"/*" => Some((i, BlockCommentTag::Start)),
                _ => None,
            });

        match event {
            None => return Err(NomErr::Error(make_error("", nom::error::ErrorKind::Eof))),
            Some((i, BlockCommentTag::End)) => return Ok((&input[i + 2..], ())),
            Some((i, BlockCommentTag::Start)) => {
                let (tail, ()) = finish_block_comment(&input[i + 2..])?;
                input = tail;
            }
        }
    }
}

// Parse and discard a (potentially nested) block comment
pub fn parse_block_comment<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    tag("/*").precedes(finish_block_comment).parse(input)
}

#[cfg(test)]
mod test_block_comment {
    use super::*;

    fn typed_block_comment(input: &str) -> IResult<&str, (), ()> {
        parse_block_comment(input)
    }

    #[test]
    fn basic() {
        assert_eq!(typed_block_comment("/* abcd */123"), Ok(("123", ())))
    }

    #[test]
    fn nested() {
        assert_eq!(
            typed_block_comment("/* abc /* def */ ghi */123"),
            Ok(("123", ()))
        )
    }

    #[test]
    fn failure() {
        assert_eq!(typed_block_comment("/*abc"), Err(NomErr::Error(())))
    }

    #[test]
    fn nested_failure() {
        assert_eq!(
            typed_block_comment("/* abc /* def */ ghi"),
            Err(NomErr::Error(()))
        )
    }
}

pub fn parse_line_comment<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    let (input, _) = tag("//").parse(input)?;
    match input.split_once('\n') {
        Some((_, tail)) => Ok((tail, ())),
        None => Ok(("", ())),
    }
}

/// Parse some non-newline whitespace: bom, unicode whitespace, and block
/// comments. Requires at least 1. Parse as much as we can find. Specifically
/// excludes newlines and things like newlines
pub fn parse_whitespace<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    fold_many1(
        alt((
            // BOM
            char('\u{FFEF}'),
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
        ))
        .value(())
        .or(parse_block_comment),
        (),
        |(), ()| (),
    )
    .parse(input)
}
