/*!
Parsers and utility types for parsing KDL numbers. While KDL doesn't distinguish
between floats and integers, or signed and unsigned numbers, this module uses
some simple logic to detect different integer types, to aid with deserialization.

# Number type logic

Currently, we use a simple set of rules to decide if a KDL number should be
parsed as an [`i64`], [`u64`], or [`f64`]:

- If the number contains a fractional part or exponent, it's unconditionally
parsed as an `f64`.
- Otherwise, if it's negative, it's parsed as an `i64`.
- Otherwise, it's parsed as a `u64`.

These rules may change in the future. Possible improvements:

- If it includes an exponent, it might be an integer, even if it includes a
fractional parts.
- If it's an integer but it overflows an `i64` or `u64`, it could be stored in
an `f64` instead.
- It's also possible we could use type hints (from serde or KDL annotations) to
guide the parse as well.
*/

use arrayvec::ArrayString;
use memchr::memchr3;
use nom::{
    branch::alt,
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

/// Helper trait for building or recognizing integers.
pub trait IntBuilder: Sized {
    /// Create a new int. The digits appended by [`add_digit`][Self::add_digit]
    /// will increase the magnitude of the number positively or negatively,
    /// depending on the `sign`.
    fn start(sign: Sign) -> Self;

    /// Append a digit to this number.
    fn add_digit(self, digit: u32, radix: u32) -> Result<Self, BoundsError>;
}

/// A KDL integer, which is either signed or unsigned. In practice, we always
/// parse positive integers as unsigned and negative integers as signed.
#[derive(Debug, Clone, Copy, Hash)]
pub enum KdlInt {
    /// A signed integer. Returned by the parser if the number was negative.
    Signed(i64),

    /// A signed integer. Returned by the parser if the number was positive.
    Unsigned(u64),
}

/// A Bounds error occurred during number parsing. This type is incomplete and
/// will grow in the future.
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

    #[inline]
    fn start(sign: Sign) -> Self {
        match sign {
            Sign::Positive => KdlInt::Unsigned(0),
            Sign::Negative => KdlInt::Signed(0),
        }
    }
}

/// The empty tuple can be used as an integer builder in cases where it's only
/// necessary to recognize the presence of a number and not to parse it.
impl IntBuilder for () {
    fn add_digit(self, _digit: u32, _radix: u32) -> Result<Self, BoundsError> {
        Ok(())
    }

    fn start(_sign: Sign) -> Self {}
}

/// The parsed sign of a number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// Positive, `+`. No sign character is assumed positive.
    Positive,

    /// Negative, `-`.
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
enum Base {
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
        at_least_one(char('_')),
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
        Base::Binary => parse_integer_part(at_least_one(alt((char('0'), char('1')))), 2, sign)
            .cut()
            .parse(input),
        Base::Octal => parse_integer_part(oct_digit1, 8, sign).cut().parse(input),
        Base::Hex => parse_integer_part(hex_digit1, 16, sign).cut().parse(input),
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
        .terminated(
            recognize_decimal_component
                .cut()
                .preceded_by(char('.'))
                .opt(),
        )
        .terminated(
            parse_optional_sign
                .terminated(recognize_decimal_component)
                .cut()
                .preceded_by(alt((char('e'), char('E'))))
                .opt(),
        )
        .recognize()
        .map_res_cut(T::from_str)
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

/// Trait for building KDL numbers
pub trait NumberBuilder: Sized {
    /// Inner type for building integers specifically. Used for binary, hex,
    /// and octal numbers, because we parse them manually (because the `0x`
    /// prefix isn't recognized by [`from_radix`][i32::from_str_radix])
    type IntForm: IntBuilder;

    /// Parse a decimal number from a string. This may have a fractional or
    /// exponent component, and may have a sign. Digits may be separated by `_`.
    fn from_str(input: &str) -> Result<Self, BoundsError>;

    /// Receive a parsed integer
    fn from_int(input: Self::IntForm) -> Self;
}

/// The empty tuple can be used as an number builder in cases where it's only
/// necessary to recognize the presence of a number and not to parse it.
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

/// A KDL Number. The KDL spec doesn't distinguish between integers and floats,
/// or between signed an unsigned numbers, but kaydle uses a heuristic to pick
/// a type for deserialization purposes.
#[derive(Debug, Copy, Clone)]
pub enum KdlNumber {
    /// A signed integer; returned by the parser if the number was negative and
    /// had no fractional component or exponent.
    Signed(i64),

    /// An unsigned integer; returned by the parser if the number was positive
    /// or unsigned and had no fractional component or exponent.
    Unsigned(u64),

    /// A floating point number; used unconditionally if the number contained
    /// a fractional component or exponent. Right now this applies even if the
    /// final parser ends up being an integer (eg, 1.12e5); this may change in
    /// the future.
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
