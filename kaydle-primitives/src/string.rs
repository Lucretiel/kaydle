use std::{
    borrow::Cow,
    char::CharTryFromError,
    convert::TryInto,
    fmt::{self, Formatter},
    iter::FromIterator,
    ops::{Deref, DerefMut, Index, RangeFrom, RangeTo},
};

use memchr::{memchr, memchr2};
use nom::{
    branch::alt,
    bytes::complete::take_while_m_n,
    character::complete::char,
    combinator::success,
    error::{make_error, ContextError, ErrorKind, FromExternalError, ParseError},
    Err as NomErr, IResult, Parser,
};
use nom_supreme::{
    multi::parse_separated_terminated,
    tag::{complete::tag, TagError},
    ParserExt,
};
use serde::{de, Deserialize, Serialize};

use crate::util::back;

/// A KDL string, parsed from an identifier, escaped string, or raw string.
/// Exists in either Owned or Borrowed form, depending on whether there were
/// excapes in the string
#[derive(Debug, Clone, Eq)]
pub struct KdlString<'a> {
    inner: Cow<'a, str>,
}

impl<'a> KdlString<'a> {
    pub fn new() -> Self {
        Self::from_borrowed("")
    }

    pub fn from_cow(cow: Cow<'a, str>) -> Self {
        Self { inner: cow }
    }

    pub fn from_borrowed(s: &'a str) -> Self {
        Self::from_cow(Cow::Borrowed(s))
    }

    pub fn from_string(s: String) -> Self {
        Self::from_cow(Cow::Owned(s))
    }

    pub fn into_string(self) -> String {
        self.inner.into_owned()
    }

    pub fn as_str(&self) -> &str {
        self
    }

    /// Apply a KDL string to a visitor
    pub fn visit_to<V, E>(self, visitor: V) -> Result<V::Value, E>
    where
        V: de::Visitor<'a>,
        E: de::Error,
    {
        match self.inner {
            Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
            Cow::Owned(value) => visitor.visit_string(value),
        }
    }
}

impl<T: AsRef<str>> PartialEq<T> for KdlString<'_> {
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Default for KdlString<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Deref for KdlString<'a> {
    type Target = Cow<'a, str>;

    fn deref(&self) -> &Cow<'a, str> {
        &self.inner
    }
}

impl DerefMut for KdlString<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsRef<str> for KdlString<'_> {
    fn as_ref(&self) -> &str {
        self
    }
}

impl<'a> Extend<&'a str> for KdlString<'a> {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        iter.into_iter().for_each(|s| self.push_str(s))
    }
}

impl Extend<char> for KdlString<'_> {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        iter.into_iter().for_each(|c| self.push_char(c))
    }
}

impl<'a, 'b> Extend<&'b char> for KdlString<'a> {
    fn extend<T: IntoIterator<Item = &'b char>>(&mut self, iter: T) {
        self.extend(iter.into_iter().copied())
    }
}

impl<T> FromIterator<T> for KdlString<'_>
where
    Self: Extend<T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut string = KdlString::new();
        string.extend(iter);
        string
    }
}

impl Serialize for KdlString<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl<'de> Deserialize<'de> for KdlString<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct KdlStringVisitor;

        impl<'de> de::Visitor<'de> for KdlStringVisitor {
            type Value = KdlString<'de>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                write!(formatter, "a KDL string")
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlString::from_borrowed(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_string(value.to_owned())
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlString::from_string(value))
            }
        }

        deserializer.deserialize_string(KdlStringVisitor)
    }
}

/// Helper trait for parsing strings with escape sequences. Allows for returning
/// borrowed strings without any allocation if there are no escape sequences,
/// or for recognizing strings without doing actual parsing / allocation.
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

impl<'a> StringBuilder<'a> for KdlString<'a> {
    fn push_str(&mut self, s: &'a str) {
        if self.is_empty() {
            **self = Cow::Borrowed(s)
        } else {
            self.to_mut().push_str(s);
        }
    }

    fn push_char(&mut self, c: char) {
        self.to_mut().push(c)
    }

    fn from_str(s: &'a str) -> Self {
        Self::from_borrowed(s)
    }
}

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

/// Parse a raw string, resembling `r##"abc"##`
pub fn parse_raw_string<'i, E: ParseError<&'i str>>(input: &'i str) -> IResult<&'i str, &'i str, E>
where
    E: ParseError<&'i str>,
{
    // The very end the input (an empty string), used for certain error
    // reports
    let eof_location = back(input);

    let (input, hash_count) =
        parse_separated_terminated(char('#'), success(()), char('"'), || 0, |n, _c| n + 1)
            .or(char('"').value(0))
            .preceded_by(char('r'))
            .parse(input)?;

    let mut shifter = SliceShifter::new(input);

    'outer: loop {
        match memchr(b'"', shifter.tail().as_bytes()) {
            // Couldn't find any quotes; need more input
            None => {
                return Err(NomErr::Error(E::or(
                    make_error(eof_location, ErrorKind::Eof),
                    E::from_char(eof_location, '"'),
                )))
            }

            // Found a quote; search the successor bytes for hashes
            Some(quote_idx) => {
                shifter.shift(quote_idx);
                let payload = shifter.head();
                shifter.shift(1);

                for _ in 0..hash_count {
                    match shifter.tail().as_bytes().get(0) {
                        None => {
                            return Err(NomErr::Error(E::or(
                                make_error(eof_location, ErrorKind::Eof),
                                E::from_char(eof_location, '#'),
                            )))
                        }
                        Some(b'#') => shifter.shift(1),
                        Some(..) => continue 'outer,
                    }
                }

                return Ok((shifter.tail(), payload));
            }
        }
    }
}

#[cfg(test)]
mod test_parse_raw {
    use super::*;
    use cool_asserts::assert_matches;
    use nom::error::Error;

    fn typed_parse_raw(input: &str) -> IResult<&str, &str, Error<&str>> {
        parse_raw_string(input)
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
        assert_matches!(
            typed_parse_raw(r####"r###"abc"####),
            Err(NomErr::Error(Error { input: "", .. }))
        )
    }

    #[test]
    fn partially_finished() {
        assert_matches!(
            typed_parse_raw(r####"r###"abc"#"####),
            Err(NomErr::Error(Error { input: "", .. }))
        )
    }
}

/// Parse a KDL bare identifier.
///
/// # Compatibility note:
///
/// Currently this parses only a subset of KDL identifiers: alphabetics followed
/// by alphanumerics.
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
}

// Parse a string matching u{00F1} as an escaped unicode code point
fn parse_unicode_escape<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
{
    take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit())
        .map(|s| u32::from_str_radix(s, 16).expect("failed to parse 1-6 hex digits to a u32?"))
        .map_res(|c: u32| c.try_into())
        .terminated(char('}'))
        .cut()
        .preceded_by(tag("u{"))
        .parse(input)
}

fn parse_escape<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
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

/// Parse a chunk of an escaped string. Must be at least 1 character.
fn parse_unescaped_chunk<'i, E>(input: &'i str) -> IResult<&'i str, &'i str, E>
where
    E: ParseError<&'i str>,
{
    // The very end the input (an empty string), used for certain error
    // reports
    let eof_location = back(input);

    match memchr2(b'"', b'\\', input.as_bytes()) {
        None => Err(NomErr::Error(E::or(
            make_error(eof_location, ErrorKind::Eof),
            E::from_char(eof_location, '"'),
        ))),

        Some(0) => Err(NomErr::Error(make_error(input, ErrorKind::TakeWhile1))),
        Some(n) => {
            let (head, tail) = input.split_at(n);
            Ok((tail, head))
        }
    }
}

enum StringChunk<'a> {
    Chunk(&'a str),
    Char(char),
}

fn parse_chunk<'i, E>(input: &'i str) -> IResult<&'i str, StringChunk<'i>, E>
where
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
{
    alt((
        parse_unescaped_chunk.map(StringChunk::Chunk),
        parse_escape.map(StringChunk::Char),
    ))
    .parse(input)
}

/// Parse a regular, quoted string (with escape sequences)
///
/// "This" -> &str
/// "This\nvalue" -> String
pub fn parse_escaped_string<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
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
mod test_parse_escaped_string {
    use super::*;
    use cool_asserts::assert_matches;
    use nom::error::Error;

    fn typed_parse_identifier(input: &str) -> IResult<&str, KdlString<'_>, Error<&str>> {
        parse_escaped_string(input)
    }

    #[test]
    fn basic() {
        assert_matches!(
            typed_parse_identifier("\"hello\" abc"),
            Ok((
                " abc",
                KdlString {
                    inner: Cow::Borrowed("hello")
                }
            ))
        )
    }

    #[test]
    fn with_escape() {
        assert_matches!(
            typed_parse_identifier("\"hello \\t world\" abc"),
            Ok((
                " abc",
                KdlString { inner: Cow::Owned(s) }
            )) => assert_eq!(s, "hello \t world")
        );
    }

    #[test]
    fn with_escaped_unicode() {
        assert_matches!(
            typed_parse_identifier("\"hello\\u{0A}world\" abc"),
            Ok((
                " abc",
                KdlString {
                    inner: Cow::Owned(s)
                }
            )) => assert_eq!(s, "hello\nworld")
        );
    }
}

/// Parse a KDL string, which is either a raw or escaped string
pub fn parse_string<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    alt((
        parse_escaped_string.context("escaped string"),
        parse_raw_string.context("raw string").map(T::from_str),
    ))
    .parse(input)
}

/// Parse a KDL identifier, which is either a bare identifer or a string
pub fn parse_identifier<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: StringBuilder<'i>,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, CharTryFromError>,
    E: ContextError<&'i str>,
{
    alt((
        parse_bare_identifier
            .map(T::from_str)
            .context("bare identifier"),
        parse_string.context("string"),
    ))
    .parse(input)
}
