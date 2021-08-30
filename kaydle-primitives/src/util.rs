use nom::Err as NomErr;
use nom::Parser;

#[inline(always)]
pub fn back(input: &str) -> &str {
    &input[input.len()..]
}

/// Run a parser as many times as it will succeed, at least once. Discard the
/// parser results.
pub fn at_least_one<I, O, E>(mut parser: impl Parser<I, O, E>) -> impl Parser<I, (), E>
where
    I: Clone,
{
    move |input| {
        let (mut input, _) = parser.parse(input)?;
        loop {
            match parser.parse(input.clone()) {
                Ok((tail, _)) => input = tail,
                Err(NomErr::Error(..)) => return Ok((input, ())),
                Err(err) => return Err(err),
            }
        }
    }
}
