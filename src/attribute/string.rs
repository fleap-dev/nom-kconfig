use nom::{
    bytes::complete::tag,
    combinator::map,
    error::{Error, ErrorKind, ParseError},
    sequence::delimited,
    IResult, Input, Parser,
};

use crate::KconfigInput;

pub fn parse_string(input: KconfigInput) -> IResult<KconfigInput, String> {
    map(
        delimited(tag("\""), take_until_unbalanced('"'), tag("\"")),
        |d| d.fragment().to_string(),
    )
    .parse(input)
}

/// See [`crate::string::take_until_unbalanced`]: the string ends at the first
/// unescaped `delimiter` that is not nested inside a `$(...)` macro expansion.
pub fn take_until_unbalanced(
    delimiter: char,
) -> impl Fn(KconfigInput) -> IResult<KconfigInput, KconfigInput> {
    move |i: KconfigInput| {
        let mut macro_depth: usize = 0;
        let mut chars = i.fragment().char_indices().peekable();

        while let Some((index, c)) = chars.next() {
            match c {
                '\\' => {
                    chars.next();
                }
                '$' if matches!(chars.peek(), Some((_, '('))) => {
                    macro_depth += 1;
                    chars.next();
                }
                ')' if macro_depth > 0 => {
                    macro_depth -= 1;
                }
                '\n' => {
                    return Err(nom::Err::Error(Error::from_error_kind(
                        i,
                        ErrorKind::TakeUntil,
                    )))
                }
                c if c == delimiter && macro_depth == 0 => {
                    return Ok(i.take_split(index));
                }
                _ => {}
            }
        }

        Err(nom::Err::Error(Error::from_error_kind(
            i,
            ErrorKind::TakeUntil,
        )))
    }
}
