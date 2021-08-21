use std::{
    borrow::Cow,
    char::CharTryFromError,
    convert::TryInto,
    num::ParseIntError,
    ops::{Index, RangeFrom, RangeTo},
};

use nom::{
    branch::alt,
    bytes::complete::{take_while, take_while1, take_while_m_n},
    character::complete::{char, satisfy},
    combinator::success,
    error::{make_error, ContextError, ErrorKind, FromExternalError, ParseError},
    Err as NomErr, IResult, Needed, Parser,
};
use nom_supreme::{
    multi::{collect_separated_terminated, parse_separated_terminated},
    tag::{complete::tag, TagError},
    ParserExt,
};

struct SliceShifter<'a, T: ?Sized> {
    base: &'a T,
    point: usize,
}

impl<'a, T: ?Sized, A: ?Sized, B: ?Sized> SliceShifter<'a, T>
where
    T: Index<RangeTo<usize>, Output = A>,
    T: Index<RangeFrom<usize>, Output = B>,
{
    fn new(base: &'a T) -> Self {
        Self { base, point: 0 }
    }

    fn head(&self) -> &'a A {
        &self.base[..self.point]
    }

    fn tail(&self) -> &'a B {
        &self.base[self.point..]
    }

    fn shift(&mut self, amount: usize) {
        self.point += amount
    }
}

/// Parse a string resembling r##"abc"##
pub fn parse_raw<'i, E: ParseError<&'i str>>(input: &'i str) -> IResult<&'i str, &'i str, E> {
    let (input, hash_count) =
        parse_separated_terminated(char('#'), success(()), char('"'), || 0, |n, c| n + 1)
            .or(char('"').value(0))
            .preceded_by(char('r'))
            .parse(input)?;

    let mut shifter = SliceShifter::new(input);

    loop {
        match shifter.tail().find('"') {
            // Couldn't find any quotes; need more input
            None => return Err(NomErr::Error(make_error("", ErrorKind::Eof))),

            // Found a quote; search the successor bytes for hashes
            Some(quote_idx) => {
                shifter.shift(quote_idx);
                let payload = shifter.head();
                shifter.shift(1);

                match shifter.tail().as_bytes().get(..hash_count) {
                    None => return Err(NomErr::Error(make_error("", ErrorKind::Eof))),
                    Some(hash_zone) => {
                        if hash_zone.iter().all(|&b| b == b'#') {
                            shifter.shift(hash_count);
                            return Ok((shifter.tail(), payload));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test_parse_raw {
    use super::*;
    use nom::error::Error;

    fn typed_parse_raw(input: &str) -> IResult<&str, &str, Error<&str>> {
        parse_raw(input)
    }

    #[test]
    fn hashless() {
        assert_eq!(typed_parse_raw(r#"r"abc"def"#), Ok(("def", "abc")))
    }

    #[test]
    fn hashed() {
        assert_eq!(
            typed_parse_raw(r####"r##"abc"##def"####),
            Ok(("def", "abc"))
        )
    }

    #[test]
    fn inner_hashes() {
        assert_eq!(
            typed_parse_raw(r####"r##"abc"#abc"##def"####),
            Ok(("def", r##"abc"#abc"##))
        )
    }

    #[test]
    fn extra_hashes() {
        assert_eq!(typed_parse_raw(r####"r##"abc"###"####), Ok(("#", "abc")))
    }

    #[test]
    fn unfinished() {
        assert_eq!(
            typed_parse_raw(r####"r###"abc"####),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        )
    }

    #[test]
    fn partially_finished() {
        assert_eq!(
            typed_parse_raw(r####"r###"abc"#"####),
            Err(NomErr::Error(Error {
                input: "",
                code: ErrorKind::Eof
            }))
        )
    }
}

/// Parse a KDL bare identifier.
///
/// # Compatibility note:
///
/// Currently this parses only a subset of KDL identifiers; it doesn't currently
/// allow punctuation.
pub fn parse_bare_identifier<'i, E: ParseError<&'i str>>(
    input: &'i str,
) -> IResult<&'i str, &'i str, E> {
    match input.chars().next() {
        Some(c) if c.is_alphabetic() => {
            let split_point = input[1..].find(|c: char| !c.is_alphanumeric()).unwrap_or(0) + 1;
            let (ident, tail) = input.split_at(split_point);
            Ok((tail, ident))
        }
        _ => Err(NomErr::Error(make_error(input, ErrorKind::Alpha))),
    }
}

#[cfg(test)]
mod test_parse_identifier {
    use super::*;
    use nom::error::{Error, ErrorKind};

    fn typed_parse_identifier(input: &str) -> IResult<&str, &str, Error<&str>> {
        parse_bare_identifier(input)
    }

    #[test]
    fn basic() {
        assert_eq!(typed_parse_identifier("abc abc"), Ok((" abc", "abc")))
    }

    #[test]
    fn with_num() {
        assert_eq!(typed_parse_identifier("abc123 abc"), Ok((" abc", "abc123")))
    }

    #[test]
    fn start_with_letter() {
        assert_eq!(
            typed_parse_identifier("123"),
            Err(NomErr::Error(Error {
                input: "123",
                code: ErrorKind::Alpha
            }))
        )
    }

    #[test]
    fn need_letter() {
        assert_eq!(
            typed_parse_identifier(""),
            Err(NomErr::Incomplete(Needed::new(1)))
        )
    }
}

// Parse a string matching u{00F1} as an escaped unicode code point
fn parse_unicode_escape<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
{
    take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit())
        .preceded_by(tag("u{"))
        .terminated(char('}'))
        .map_res_cut(|s| u32::from_str_radix(s, 16))
        .map_res_cut(|c: u32| c.try_into())
        .parse(input)
}

fn parse_escape<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
{
    alt((
        char('n').value('\n'),
        char('r').value('\r'),
        char('t').value('\t'),
        char('\\').value('\\'),
        char('/').value('/'),
        char('"').value('"'),
        char('b').value('\u{08}'),
        char('f').value('\u{0C}'),
        parse_unicode_escape,
    ))
    .preceded_by(char('\\'))
    .parse(input)
}

fn parse_unescaped_chunk<'i, E>(input: &'i str) -> IResult<&'i str, &'i str, E>
where
    E: ParseError<&'i str>,
{
    take_while1(|c: char| c != '"' && c != '\\').parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringChunk<'a> {
    Chunk(&'a str),
    Char(char),
}

fn parse_chunk<'i, E>(input: &'i str) -> IResult<&'i str, StringChunk<'i>, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
{
    alt((
        parse_unescaped_chunk.map(StringChunk::Chunk),
        parse_escape.map(StringChunk::Char),
    ))
    .parse(input)
}

/// Helper trait for parsing strings with escape sequences. Allows for returning
/// borrowed strings without any allocation if there are no escape sequences.
pub trait StringBuilder<'a>: Sized {
    /// Add a borrowed string to the back of this string
    fn push_str(&mut self, s: &'a str);

    /// Add a char to the back of this string
    fn push_char(&mut self, c: char);

    /// Create a new instance from a borrowed string
    fn from_str(s: &'a str) -> Self;

    /// Create a new empty instance
    fn empty() -> Self {
        Self::from_str("")
    }
}

/// The empty tuple can be used as a string builder in cases where it's only
/// necessary to recognize a string and not to parse it
impl<'a> StringBuilder<'a> for () {
    fn push_str(&mut self, _s: &'a str) {}
    fn push_char(&mut self, _c: char) {}
    fn from_str(_s: &'a str) {}
}

/// Strings can, of course, be built
impl<'a> StringBuilder<'a> for String {
    fn push_str(&mut self, s: &'a str) {
        self.push_str(s)
    }

    fn push_char(&mut self, c: char) {
        self.push(c)
    }

    fn from_str(s: &'a str) -> Self {
        s.to_owned()
    }
}

/// Cow is the real winner; as long as it's empty, push_str can work without
/// allocating.
impl<'a> StringBuilder<'a> for Cow<'a, str> {
    fn push_str(&mut self, s: &'a str) {
        if self.is_empty() {
            *self = Cow::Borrowed(s)
        } else {
            self.to_mut().push_str(s);
        }
    }

    fn push_char(&mut self, c: char) {
        self.to_mut().push(c)
    }

    fn from_str(s: &'a str) -> Self {
        Cow::Borrowed(s)
    }
}

/// Parse a regular, quoted string (with escape sequences)
pub fn parse_escaped_string<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
{
    parse_separated_terminated(
        parse_chunk,
        success(()),
        char('"'),
        T::empty,
        |mut string, chunk| {
            match chunk {
                StringChunk::Chunk(chunk) => string.push_str(chunk),
                StringChunk::Char(c) => string.push_char(c),
            }
            string
        },
    )
    .or(char('"').map(|_| T::empty()))
    .preceded_by(char('"'))
    .parse(input)
}

#[cfg(test)]
mod test_parse_escaped_string {}

/// Parse a KDL string, which is either a raw or escaped string
pub fn parse_string<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    alt((
        parse_escaped_string.context("escaped string"),
        parse_raw.context("raw string").map(T::from_str),
    ))
    .parse(input)
}

/// Parse a KDL identifier, which is either a bare identifer or a string
pub fn parse_identifier<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, ParseIntError>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    alt((
        parse_bare_identifier
            .context("bare identifier")
            .map(T::from_str),
        parse_string,
    ))
    .parse(input)
}
