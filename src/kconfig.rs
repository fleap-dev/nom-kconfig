use nom::{combinator::eof, IResult, Parser};
#[cfg(feature = "permissive")]
use nom::combinator::opt;
#[cfg(not(feature = "permissive"))]
use nom::{combinator::map, multi::many0, sequence::delimited};
#[cfg(feature = "deserialize")]
use serde::Deserialize;
#[cfg(feature = "serialize")]
use serde::Serialize;
#[cfg(feature = "debug")]
use tracing::debug;

#[cfg(feature = "permissive")]
use crate::util::parse_until_eol;
#[cfg(not(feature = "permissive"))]
use crate::util::ws;
use crate::{
    entry::{parse_entry, Entry},
    error::Error,
    util::ws_comment,
    KconfigInput,
};

/// A Kconfig file.
/// Field `file` is relative to the root directory defined in [KconfigFile](crate::KconfigFile).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "hash", derive(Hash))]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "deserialize", derive(Deserialize))]
pub struct Kconfig {
    pub file: String,
    pub entries: Vec<Entry>,
}

/// Parses a kconfig input.
/// # Example
/// ```
/// use std::path::PathBuf;
/// use nom_kconfig::{KconfigInput, KconfigFile, Entry, kconfig::parse_kconfig, Kconfig};
///
/// let kconfig_file = KconfigFile::new(PathBuf::from("path/to/root/dir"), PathBuf::from("Kconfig"));
/// let content = "";
/// let input = KconfigInput::new_extra(content, kconfig_file);
/// assert_eq!(parse_kconfig(input).unwrap().1, Kconfig {file: "Kconfig".to_string(), entries: vec!() })
/// ```
pub fn parse_kconfig(input: KconfigInput) -> Result<(KconfigInput, Kconfig), Error> {
    match private_parse_kconfig(input) {
        Ok((input, result)) => Ok((input, result)),
        Err(nom_error) => Err(Error::from(nom_error)),
    }
}

pub(crate) fn private_parse_kconfig(input: KconfigInput) -> IResult<KconfigInput, Kconfig> {
    #[cfg(feature = "debug")]
    debug!("parsing '{}'", input.extra.full_path().display());
    let file: std::path::PathBuf = input.extra.file.clone();

    #[cfg(feature = "permissive")]
    let (input, entries) = parse_entries_permissive(input)?;

    #[cfg(not(feature = "permissive"))]
    let (input, entries) =
        delimited(ws_comment, many0(parse_entry), ws(eof)).parse(input)?;

    Ok((
        input,
        Kconfig {
            file: file.display().to_string(),
            entries,
        },
    ))
}

/// Parse the top-level entries of a Kconfig file, recovering from malformed
/// input by skipping any line that does not parse as a known entry.
///
/// Several historical kernel trees contain genuinely broken Kconfig files —
/// e.g. stray `+` patch markers (v3.6 `sound/soc/ux500/Kconfig`), a lone `.`
/// line (v3.7–v3.11 `drivers/media/usb/stk1160/Kconfig`), or a trailing `\\`
/// line-continuation inside an expression (v3.19 `sound/soc/intel/Kconfig`).
/// The real kconfig tooling tolerates these, so rather than aborting the whole
/// parse we drop the offending fragment and resync at the next valid entry.
#[cfg(feature = "permissive")]
fn parse_entries_permissive(mut input: KconfigInput) -> IResult<KconfigInput, Vec<Entry>> {
    let mut entries = Vec::new();
    loop {
        // Skip leading whitespace/comments; stop once nothing is left.
        (input, _) = ws_comment(input)?;
        if opt(eof).parse(input.clone())?.1.is_some() {
            break;
        }

        match parse_entry(input.clone()) {
            Ok((rest, entry)) => {
                input = rest;
                entries.push(entry);
            }
            Err(nom::Err::Error(_)) => {
                // Unparseable line: discard it and continue. `ws_comment` above
                // guarantees we are positioned on a non-whitespace character, so
                // `parse_until_eol` consumes at least one character and the loop
                // always makes progress.
                (input, _) = parse_until_eol(input)?;
            }
            Err(err) => return Err(err),
        }
    }
    Ok((input, entries))
}
