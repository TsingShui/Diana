//! Resource metadata scoped to the original JVM archive and entry.

use std::collections::HashMap;

use super::archive::{EntryDecoder, EntryRecord};
use super::filter::{FilterMatch, Relevance, TextFilter, best_first};
use super::input::InputError;
use super::jvm::{JvmRoots, Location, MANIFEST_NAME, METADATA_BYTES};
use super::query::QueryError;
use super::request::RequestScope;
use diana_decomp::FailureKind;

pub struct Resource<'a> {
    pub at: Location,
    pub entry: &'a EntryRecord,
    relevance: Option<Relevance>,
}

pub fn select<'a>(
    roots: &'a JvmRoots<'_>,
    filter: Option<&TextFilter>,
    resources_only: bool,
) -> Result<Vec<Resource<'a>>, QueryError> {
    let mut resources = Vec::new();
    for archive in roots.archives() {
        for entry in archive.entries {
            if resources_only && !is_resource(entry) {
                continue;
            }
            let matched = match filter {
                Some(filter) => filter.evaluate_text(entry.name())?,
                None => Some(FilterMatch::default()),
            };
            if let Some(matched) = matched {
                resources.try_reserve(1).map_err(InputError::Allocation)?;
                resources.push(Resource {
                    at: archive.entry_location(entry),
                    entry,
                    relevance: matched.relevance,
                });
            }
        }
    }
    if filter.is_some_and(TextFilter::is_fuzzy) {
        resources.sort_by(|left, right| {
            best_first(left.relevance, right.relevance).then_with(|| left.at.cmp(&right.at))
        });
    }
    Ok(resources)
}

fn is_resource(entry: &EntryRecord) -> bool {
    !entry.name().ends_with(".class")
        && (!entry.name().ends_with('/') || entry.uncompressed_size().0 != 0)
}

/// Reads the same physical manifest retained by export. This request opens
/// just the primary archive, including for multi-archive project inputs.
pub fn manifest(scope: &RequestScope) -> Result<Option<(Location, Vec<u8>)>, QueryError> {
    let first = "j0"
        .parse::<Location>()
        .expect("primary archive root location");
    let roots = JvmRoots::focus_archive(scope, &first)?;
    let Some(archive) = roots.archive_for(&first) else {
        return Ok(None);
    };
    let Some(entry) = archive
        .entries
        .iter()
        .rfind(|entry| entry.name().eq_ignore_ascii_case(MANIFEST_NAME))
    else {
        return Ok(None);
    };
    let mut decoder = EntryDecoder::new();
    let bytes = decoder.read(archive.bytes, entry, METADATA_BYTES)?;
    let mut manifest = Vec::new();
    manifest
        .try_reserve_exact(bytes.len())
        .map_err(InputError::Allocation)?;
    manifest.extend_from_slice(bytes);
    Ok(Some((archive.entry_location(entry), manifest)))
}

/// Literal path candidates from original entries. The matched literal is
/// evidence about a path spelling, independently of runtime lookup.
#[derive(Clone, Debug, Default)]
pub struct PathTargets {
    names: HashMap<String, FilterMatch>,
}

impl PathTargets {
    pub(crate) fn named(roots: &JvmRoots<'_>, filter: &TextFilter) -> Result<Self, QueryError> {
        let selected = select(roots, Some(filter), true)?;
        let mut names = HashMap::new();
        names
            .try_reserve(selected.len())
            .map_err(InputError::Allocation)?;
        for resource in selected {
            names.insert(
                resource.entry.name().to_owned(),
                FilterMatch {
                    relevance: resource.relevance,
                },
            );
        }
        Ok(Self { names })
    }

    pub(crate) fn at(roots: &JvmRoots<'_>, at: &Location) -> Result<Self, QueryError> {
        let entry = roots
            .archive_for(at)
            .and_then(|archive| archive.entries.get(at.entry()))
            .filter(|entry| is_resource(entry))
            .ok_or_else(|| QueryError::Local {
                kind: FailureKind::Malformed,
                reason: format!("no original JVM resource at {at}"),
            })?;
        Ok(Self {
            names: HashMap::from([(entry.name().to_owned(), FilterMatch::default())]),
        })
    }

    pub(crate) fn is_ranked(&self) -> bool {
        self.names
            .values()
            .any(|candidate| candidate.relevance.is_some())
    }

    pub(crate) fn matches(
        &self,
        value: diana_decomp::input::jvm::JvmString<'_>,
    ) -> Option<FilterMatch> {
        if self.names.is_empty() {
            return None;
        }
        let lookup = |text: &str| {
            self.names
                .get(text)
                .or_else(|| text.strip_prefix('/').and_then(|path| self.names.get(path)))
                .copied()
        };
        if let Some(text) = value.utf8() {
            lookup(text)
        } else {
            let text = char::decode_utf16(value.units())
                .collect::<Result<String, _>>()
                .ok()?;
            lookup(&text)
        }
    }
}
