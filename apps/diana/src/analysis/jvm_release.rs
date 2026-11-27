//! Selection of logical JVM class paths for a configured Java release.

use super::archive::{ArchiveDirectory, EntryDecoder};
use super::input::{EntryIndex, InputError};
use super::jvm::Location;
use super::query::{QueryError, QueryIssue};
use diana_decomp::FailureKind;
use std::collections::HashMap;

const VERSIONS: &str = "META-INF/versions/";
pub const MANIFEST_NAME: &str = "META-INF/MANIFEST.MF";

pub(super) fn select(
    classes: &mut Vec<Location>,
    release: u16,
    bytes: &[u8],
    directory: &ArchiveDirectory,
    decoder: &mut EntryDecoder,
    issues: &mut Vec<QueryIssue>,
) -> Result<(), QueryError> {
    let name = |location: &Location| {
        directory
            .entry(EntryIndex(location.entry()))
            .expect("inventoried physical class entry")
            .name()
    };
    if !classes
        .iter()
        .any(|location| name(location).starts_with(VERSIONS))
    {
        return Ok(());
    }

    let enabled = release >= 9
        && match multi_release(directory, decoder, bytes) {
            Ok(enabled) => enabled,
            Err((index, error)) => {
                issues.push(QueryIssue::new(
                    classes[0].sibling(index).to_string(),
                    &error,
                ));
                false
            }
        };
    let mut latest = HashMap::<&str, u16>::new();
    if enabled {
        for location in &*classes {
            let Some((version, path)) = versioned_path(name(location)) else {
                continue;
            };
            if version <= release {
                latest.try_reserve(1).map_err(InputError::Allocation)?;
                latest
                    .entry(path)
                    .and_modify(|value| *value = (*value).max(version))
                    .or_insert(version);
            }
        }
    }
    classes.retain(|location| {
        let name = name(location);
        if !name.starts_with(VERSIONS) {
            return !latest.contains_key(name);
        }
        versioned_path(name).is_some_and(|(version, path)| latest.get(path) == Some(&version))
    });
    Ok(())
}

fn versioned_path(name: &str) -> Option<(u16, &str)> {
    let (number, path) = name.strip_prefix(VERSIONS)?.split_once('/')?;
    if number.starts_with('0')
        || !number.bytes().all(|byte| byte.is_ascii_digit())
        || path.starts_with("META-INF/")
    {
        return None;
    }
    let number: u16 = number.parse().ok()?;
    (number >= 9).then_some((number, path))
}

fn multi_release(
    directory: &ArchiveDirectory,
    decoder: &mut EntryDecoder,
    bytes: &[u8],
) -> Result<bool, (usize, QueryError)> {
    let mut manifests = directory
        .entries()
        .iter()
        .filter(|entry| entry.name() == MANIFEST_NAME);
    let Some(manifest) = manifests.next() else {
        return Ok(false);
    };
    let index = manifest.index().0;
    if manifests.next().is_some() {
        return Err((
            index,
            malformed("multiple JAR manifests make target-release selection ambiguous"),
        ));
    }
    let body = decoder
        .read(bytes, manifest, super::jvm::METADATA_BYTES)
        .map_err(|error| (index, error.into()))?;
    manifest_flag(body).map_err(|error| (index, error))
}

fn manifest_flag(mut bytes: &[u8]) -> Result<bool, QueryError> {
    let mut declared = None;
    let mut partial = None;
    while let Some(line) = consume_line(&mut bytes) {
        if let Some(continuation) = line.strip_prefix(b" ") {
            if let Some(progress) = partial.as_mut() {
                *progress = continue_flag(*progress, continuation);
            }
            continue;
        }
        if let Some(progress) = partial.take() {
            declared = Some(progress == b"true".len());
        }
        if line.is_empty() {
            break;
        }
        let Some(separator) = line.iter().position(|&byte| byte == b':') else {
            continue;
        };
        if line[..separator].eq_ignore_ascii_case(b"Multi-Release") {
            if declared.is_some() {
                return Err(malformed(
                    "JAR manifest repeats the Multi-Release main attribute",
                ));
            }
            let value = line[separator + 1..]
                .strip_prefix(b" ")
                .ok_or_else(|| malformed("invalid Multi-Release manifest value"))?;
            partial = Some(continue_flag(0, value));
        }
    }
    Ok(partial
        .map(|progress| progress == b"true".len())
        .or(declared)
        .unwrap_or(false))
}

fn consume_line<'a>(bytes: &mut &'a [u8]) -> Option<&'a [u8]> {
    if bytes.is_empty() {
        return None;
    }
    let Some(end) = bytes.iter().position(|&byte| matches!(byte, b'\r' | b'\n')) else {
        return Some(std::mem::take(bytes));
    };
    let (line, tail) = bytes.split_at(end);
    let ending = if tail.starts_with(b"\r\n") { 2 } else { 1 };
    *bytes = &tail[ending..];
    Some(line)
}

fn continue_flag(progress: usize, value: &[u8]) -> usize {
    progress
        .checked_add(value.len())
        .filter(|&end| {
            b"true"
                .get(progress..end)
                .is_some_and(|expected| expected.eq_ignore_ascii_case(value))
        })
        .unwrap_or(usize::MAX)
}

fn malformed(reason: &str) -> QueryError {
    QueryError::Local {
        kind: FailureKind::Malformed,
        reason: reason.to_owned(),
    }
}
