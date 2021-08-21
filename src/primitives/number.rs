use nom::{
    character::complete::satisfy,
    error::{make_error, ErrorKind, ParseError},
    Err as NomErr, IResult, Parser,
};

fn parse_int<'i, E: ParseError<&'i str>>(input: &str) -> IResult<&str, i64, E> {
    todo!()
}

fn parse_float<'i, E: ParseError<&'i str>>(input: &str) -> IResult<&str, i64, E> {
    todo!()
}
