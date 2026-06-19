use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, one_of},
    combinator::{map, recognize},
    error::{Error, ErrorKind, ParseError},
    multi::many1,
    sequence::delimited,
    IResult, Input, Parser,
};

use crate::{util::ws, KconfigInput};

pub fn parse_string(input: KconfigInput) -> IResult<KconfigInput, String> {
    map(
        alt((
            delimited(tag("'"), take_until_unbalanced('\''), tag("'")),
            delimited(tag("\""), take_until_unbalanced('"'), tag("\"")),
        )),
        |d| d.fragment().to_string(),
    )
    .parse(input)
}

/// Consumes the content of a quoted string up to (but not including) its closing
/// `delimiter`, mirroring Kconfig's lexer. The string ends at the first occurrence
/// of `delimiter` that is:
///   * not escaped with a backslash, and
///   * not nested inside a `$(...)` macro expansion (whose own quotes are part of
///     the macro argument, e.g. `"$(shell, ... "x" "y")"`).
///
/// A newline or end of input before the closing delimiter is an error, since a
/// string delimiter cannot span multiple lines. This correctly leaves a following
/// expression untouched, e.g. for `'y'||(...)` only `y` is consumed.
pub fn take_until_unbalanced(
    delimiter: char,
) -> impl Fn(KconfigInput) -> IResult<KconfigInput, KconfigInput> {
    move |i: KconfigInput| {
        let mut macro_depth: usize = 0;
        let mut chars = i.fragment().char_indices().peekable();

        while let Some((index, c)) = chars.next() {
            match c {
                // An escaped character never terminates the string; skip the next one.
                '\\' => {
                    chars.next();
                }
                // Enter a `$(...)` macro expansion; quotes inside do not terminate.
                '$' if matches!(chars.peek(), Some((_, '('))) => {
                    macro_depth += 1;
                    chars.next();
                }
                ')' if macro_depth > 0 => {
                    macro_depth -= 1;
                }
                // A string delimiter is never allowed to span multiple lines.
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

        // Reached end of input without finding the closing delimiter.
        Err(nom::Err::Error(Error::from_error_kind(
            i,
            ErrorKind::TakeUntil,
        )))
    }
}

/// A first word is `'something here'` or `"something here"` or just a normal word without spaces. It is used in places where Kconfig allows either a string or a symbol, such as in `default` attributes.
pub fn parse_first_word(input: KconfigInput) -> IResult<KconfigInput, KconfigInput> {
    alt((
        recognize((tag("'"), take_until_unbalanced('\''), tag("'"))),
        recognize((tag("\""), take_until_unbalanced('"'), tag("\""))),
        recognize(ws(many1(alt((alphanumeric1, recognize(one_of("-._'\""))))))),
    ))
    .parse(input)
}
