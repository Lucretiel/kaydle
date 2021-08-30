use arrayvec::ArrayString;
use memchr::memchr3;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, digit1, hex_digit1, oct_digit1},
    error::{FromExternalError, ParseError},
    IResult, Parser,
};
use nom_supreme::{
    multi::parse_separated_terminated_res,
    tag::{complete::tag, TagError},
    ParserExt,
};
use serde::{de, Deserialize, Serialize};

use crate::util::at_least_one;

/// Helper trait for building or recognizing integers
pub trait IntBuilder: Sized {
    fn add_digit(self, digit: u32, radix: u32) -> Result<Self, BoundsError>;
    fn start(sign: Sign) -> Self;
}

#[derive(Debug, Clone, Copy, Hash)]
pub enum KdlInt {
    Signed(i64),
    Unsigned(u64),
}

pub struct BoundsError;

impl IntBuilder for KdlInt {
    #[inline]
    fn add_digit(self, digit: u32, radix: u32) -> Result<Self, BoundsError> {
        match self {
            KdlInt::Signed(value) => Ok(KdlInt::Signed(
                value
                    .checked_mul(radix as i64)
                    .ok_or(BoundsError)?
                    .checked_sub(digit as i64)
                    .ok_or(BoundsError)?,
            )),
            KdlInt::Unsigned(value) => Ok(KdlInt::Unsigned(
                value
                    .checked_mul(radix as u64)
                    .ok_or(BoundsError)?
                    .checked_add(digit as u64)
                    .ok_or(BoundsError)?,
            )),
        }
    }

    fn start(sign: Sign) -> Self {
        match sign {
            Sign::Positive => KdlInt::Unsigned(0),
            Sign::Negative => KdlInt::Signed(0),
        }
    }
}

impl IntBuilder for () {
    fn add_digit(self, _digit: u32, _radix: u32) -> Result<Self, BoundsError> {
        Ok(())
    }

    fn start(_sign: Sign) -> Self {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Positive,
    Negative,
}

/// Parse a `+` or `-`
fn parse_sign<'i, E>(input: &'i str) -> IResult<&'i str, Sign, E>
where
    E: ParseError<&'i str>,
{
    alt((
        char('+').value(Sign::Positive),
        char('-').value(Sign::Negative),
    ))
    .parse(input)
}

/// Parse an optional `+` or `-`. Returns `Sign::Positive` if there was no sign.
fn parse_optional_sign<'i, E>(input: &'i str) -> IResult<&'i str, Sign, E>
where
    E: ParseError<&'i str>,
{
    parse_sign
        .opt()
        .map(|sign| sign.unwrap_or(Sign::Positive))
        .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Binary,
    Octal,
    Hex,
}

/// Parse `0x`, `0o`, or `0b`.
fn parse_base<'i, E>(input: &'i str) -> IResult<&'i str, Base, E>
where
    E: ParseError<&'i str> + TagError<&'i str, &'static str>,
{
    alt((
        tag("0x").value(Base::Hex),
        tag("0o").value(Base::Octal),
        tag("0b").value(Base::Binary),
    ))
    .parse(input)
}

/// Parse an integer as a series of integer components (recognized by
/// number_parser), separated by underscores. Build a number by feeding the
/// recognized digits into `T`, using the given base and sign.
fn parse_integer_part<'i, E, T, O>(
    number_parser: impl Parser<&'i str, O, E>,
    radix: u32,
    sign: Sign,
) -> impl Parser<&'i str, T, E>
where
    E: ParseError<&'i str>,
    E: FromExternalError<&'i str, BoundsError>,
    T: IntBuilder,
{
    parse_separated_terminated_res(
        number_parser.recognize(),
        take_while1(|c| c == '_'),
        char('_').not(),
        move || T::start(sign),
        move |value, digits| {
            digits
                .as_bytes()
                .iter()
                .map(|&c| (c as char).to_digit(radix).unwrap())
                .try_fold(value, |value, digit| value.add_digit(digit, radix))
        },
    )
}

/// Parse a binary, hex, or octal number
fn parse_weird_number<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: IntBuilder,
    E: ParseError<&'i str>,
    E: TagError<&'i str, &'static str>,
    E: FromExternalError<&'i str, BoundsError>,
{
    let (input, sign) = parse_optional_sign(input)?;
    let (input, base) = parse_base(input)?;

    match base {
        Base::Binary => {
            parse_integer_part(at_least_one(char('0').or(char('1'))), 2, sign).parse(input)
        }
        Base::Octal => parse_integer_part(oct_digit1, 8, sign).parse(input),
        Base::Hex => parse_integer_part(hex_digit1, 16, sign).parse(input),
    }
}

fn recognize_decimal_component<'i, E>(input: &'i str) -> IResult<&'i str, (), E>
where
    E: ParseError<&'i str>,
    E: FromExternalError<&'i str, BoundsError>,
{
    parse_integer_part(digit1, 10, Sign::Positive).parse(input)
}

/// Parse a decimal number, which may be an integer or a float.
fn parse_decimal_number<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: NumberBuilder,
    E: ParseError<&'i str>,
    E: FromExternalError<&'i str, BoundsError>,
{
    parse_optional_sign
        .terminated(recognize_decimal_component)
        .terminated(char('.').terminated(recognize_decimal_component).opt())
        .terminated(
            char('e')
                .or(char('E'))
                .terminated(parse_optional_sign)
                .terminated(recognize_decimal_component)
                .opt(),
        )
        .recognize()
        .map_res(T::from_str)
        .parse(input)
}

/// Parse a KDL number
pub fn parse_number<'i, T, E>(input: &'i str) -> IResult<&'i str, T, E>
where
    T: NumberBuilder,
    E: ParseError<&'i str>,
    E: FromExternalError<&'i str, BoundsError>,
    E: TagError<&'i str, &'static str>,
{
    alt((
        // Important: Given the number "-0xFF", the "-0" is a valid decimal
        // integer. It is therefore important that we try the weird numbers
        // *first*, then fall back to the decimal version.
        parse_weird_number.map(T::from_int),
        parse_decimal_number,
    ))
    .parse(input)
}

pub trait NumberBuilder: Sized {
    type IntForm: IntBuilder;

    fn from_str(input: &str) -> Result<Self, BoundsError>;
    fn from_int(input: Self::IntForm) -> Self;
}

impl NumberBuilder for () {
    type IntForm = ();
    fn from_str(_input: &str) -> Result<Self, BoundsError> {
        Ok(())
    }

    fn from_int(_input: ()) -> Self {}
}

impl NumberBuilder for KdlNumber {
    type IntForm = KdlInt;

    fn from_str(input: &str) -> Result<Self, BoundsError> {
        let mut buffer: ArrayString<64>;

        let input = if input.contains('_') {
            buffer = ArrayString::new();

            input
                .split('_')
                .filter(|s| !s.is_empty())
                .try_for_each(|s| buffer.try_push_str(s).ok())
                .ok_or(BoundsError)?;

            &buffer
        } else {
            input
        };

        if memchr3(b'.', b'e', b'E', input.as_bytes()).is_some() {
            input.parse().map(KdlNumber::Float).map_err(|_| BoundsError)
        } else if input.starts_with('-') {
            input
                .parse()
                .map(KdlNumber::Signed)
                .map_err(|_| BoundsError)
        } else {
            input
                .parse()
                .map(KdlNumber::Unsigned)
                .map_err(|_| BoundsError)
        }
    }

    fn from_int(input: KdlInt) -> Self {
        input.into()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum KdlNumber {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

impl From<u64> for KdlNumber {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<i64> for KdlNumber {
    fn from(value: i64) -> Self {
        Self::Signed(value)
    }
}

impl From<f64> for KdlNumber {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<KdlInt> for KdlNumber {
    fn from(value: KdlInt) -> Self {
        match value {
            KdlInt::Signed(value) => KdlNumber::Signed(value),
            KdlInt::Unsigned(value) => KdlNumber::Unsigned(value),
        }
    }
}

impl Default for KdlNumber {
    fn default() -> Self {
        KdlNumber::Signed(0)
    }
}

impl KdlNumber {
    /// Apply a KDL number to a visitor
    pub fn visit_to<'de, V, E>(self, visitor: V) -> Result<V::Value, E>
    where
        V: de::Visitor<'de>,
        E: de::Error,
    {
        match self {
            KdlNumber::Signed(value) => visitor.visit_i64(value),
            KdlNumber::Unsigned(value) => visitor.visit_u64(value),
            KdlNumber::Float(value) => visitor.visit_f64(value),
        }
    }
}

impl Serialize for KdlNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            KdlNumber::Signed(value) => serializer.serialize_i64(value),
            KdlNumber::Unsigned(value) => serializer.serialize_u64(value),
            KdlNumber::Float(value) => serializer.serialize_f64(value),
        }
    }
}

impl<'de> Deserialize<'de> for KdlNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = KdlNumber;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a KDL number")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlNumber::Signed(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlNumber::Unsigned(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(KdlNumber::Float(value))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test {
        ($test:ident: $parser:ident($input:literal) ok $variant:ident($value:literal), $tail:literal) => {
            #[test]
            fn $test() {
                let res: IResult<&str, KdlNumber, ()> = $parser($input);
                let value = cool_asserts::assert_matches!(res, Ok(($tail, KdlNumber::$variant(value))) => value);
                assert_eq!(value, $value);
            }
        };

        ($test:ident: $parser:ident($input:literal) err) => {
            #[test]
            fn $test() {
                let res: IResult<&str, (), ()> = $parser($input);
                res.expect_err("parser succeeded");
            }
        };
    }

    macro_rules! tests {
        ($parser:ident: $(
            $test:ident: $input:literal $state:ident $($variant:ident($value:literal), $tail:literal)?;
        )*) => {
            $(
                test!{ $test: $parser($input) $state $($variant($value), $tail)? }
            )*
        };
    }

    tests! {
        parse_number:

        decimal: "10 " ok Unsigned(10), " ";
        negative: "-10 " ok Signed(-10), " ";
        underscores: "1_000_000 " ok Unsigned(1000000), " ";

        float: "-10.5 " ok Float(-10.5), " ";
        exponent: "10.5e3 " ok Float(10500.0), " ";

        hex: "0xFF " ok Unsigned(0xFF), " ";
        neg_hex: "-0x0A " ok Signed(-0x0A), " ";
        hex_underscore: "0xFF_FF " ok Unsigned(0xFFFF), " ";

        binary: "0b00001111 " ok Unsigned(15), " ";
        octal: "-0o777_7 " ok Signed(-0o7777), " ";

    }
}
