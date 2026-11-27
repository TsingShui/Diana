//! Locate JVM class files without giving them DEX index identities.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    str::FromStr,
    sync::Arc,
};

use diana_decomp::FailureKind;
use diana_decomp::input::jvm::{ClassFile, ClassScratch};

use super::archive::{ArchiveDirectory, EntryDecoder, EntryRecord, checked_stored_entry};
use super::input::{EntryIndex, InputError, InputKind, InputLease};
use super::query::{QueryError, QueryIssue};
use super::request::RequestScope;

pub use super::jvm_release::MANIFEST_NAME;

const CLASS_BYTES: usize = 64 * 1024 * 1024;
const NESTED_BYTES: usize = 512 * 1024 * 1024;
pub(super) const METADATA_BYTES: usize = 1024 * 1024;
const MAX_NESTING: usize = 8;

/// Physical archive-entry indexes. The final component names the class entry.
/// A standalone class is its own entry zero.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    artifact: usize,
    parents: Option<Arc<[usize]>>,
    entry: usize,
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("expected a JVM location such as j4, j6/j4 or a1/j4")]
pub struct InvalidLocation;

impl Location {
    pub fn artifact(&self) -> usize {
        self.artifact
    }

    pub fn entry(&self) -> usize {
        self.entry
    }

    pub(super) fn sibling(&self, entry: usize) -> Self {
        Self {
            artifact: self.artifact,
            parents: self.parents.clone(),
            entry,
        }
    }

    pub fn child(&self, entry: usize) -> Result<Self, InvalidLocation> {
        let original = self.parents.as_deref().unwrap_or_default();
        let mut parents = [0; MAX_NESTING];
        let length = original.len() + 1;
        let parents = parents.get_mut(..length).ok_or(InvalidLocation)?;
        let (last, prefix) = parents.split_last_mut().ok_or(InvalidLocation)?;
        prefix.copy_from_slice(original);
        *last = self.entry;
        Ok(Self {
            artifact: self.artifact,
            parents: Some(Arc::from(&*parents)),
            entry,
        })
    }

    fn path(&self) -> impl Iterator<Item = usize> + '_ {
        self.parents
            .as_deref()
            .unwrap_or_default()
            .iter()
            .copied()
            .chain([self.entry])
    }

    pub(super) fn flat(&self) -> String {
        self.to_string().replace('/', "-")
    }

    pub fn write(&self, output: &mut Vec<u8>) {
        if self.artifact != 0 {
            output.push(b'a');
            super::at::push_decimal(output, self.artifact as u64);
            output.push(b'/');
        }
        for (index, entry) in self.path().enumerate() {
            if index != 0 {
                output.push(b'/');
            }
            output.push(b'j');
            super::at::push_decimal(output, entry as u64);
        }
    }
}

pub(super) fn archive_coordinate(path: &[usize]) -> String {
    use fmt::Write;
    let mut text = String::new();
    for &entry in path {
        if !text.is_empty() {
            text.push('-');
        }
        write!(text, "j{entry}").expect("write to String");
    }
    text
}

impl fmt::Display for Location {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.artifact != 0 {
            write!(formatter, "a{}/", self.artifact)?;
        }
        for (index, entry) in self.path().enumerate() {
            if index != 0 {
                formatter.write_str("/")?;
            }
            write!(formatter, "j{entry}")?;
        }
        Ok(())
    }
}

impl FromStr for Location {
    type Err = InvalidLocation;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (artifact, text) = if let Some(prefixed) = text.strip_prefix('a') {
            let (index, class) = prefixed.split_once('/').ok_or(InvalidLocation)?;
            (location_number(index)?, class)
        } else {
            (0, text)
        };
        let mut parts = [0usize; MAX_NESTING + 1];
        let mut len = 0;
        for part in text.split('/') {
            let digits = part.strip_prefix('j').ok_or(InvalidLocation)?;
            *parts.get_mut(len).ok_or(InvalidLocation)? = location_number(digits)?;
            len += 1;
        }
        let (entry, parents) = parts[..len].split_last().ok_or(InvalidLocation)?;
        Ok(Self {
            artifact,
            parents: (!parents.is_empty()).then(|| Arc::from(parents)),
            entry: *entry,
        })
    }
}

fn location_number(digits: &str) -> Result<usize, InvalidLocation> {
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(InvalidLocation);
    }
    digits.parse().map_err(|_| InvalidLocation)
}

#[derive(Clone, Copy)]
pub struct JvmRoot<'a> {
    bytes: &'a [u8],
    entry: Option<&'a EntryRecord>,
    location: &'a Location,
}

impl<'a> JvmRoot<'a> {
    pub fn location(self) -> &'a Location {
        self.location
    }

    pub(super) fn entry_name(self) -> Option<&'a str> {
        self.entry.map(EntryRecord::name)
    }
}

struct Container<'a> {
    artifact: usize,
    bytes: Cow<'a, [u8]>,
    directory: Option<Cow<'a, ArchiveDirectory>>,
    parents: Option<Arc<[usize]>>,
    classes: Vec<Location>,
    catalogs: Vec<EntryIndex>,
    libraries: Vec<EntryIndex>,
    end: usize,
}

impl<'a> Container<'a> {
    fn new(
        artifact: usize,
        bytes: Cow<'a, [u8]>,
        directory: Option<Cow<'a, ArchiveDirectory>>,
        parents: Option<Arc<[usize]>>,
        inventory: bool,
    ) -> Result<Self, QueryError> {
        let mut container = Self {
            artifact,
            bytes,
            directory,
            parents,
            classes: Vec::new(),
            catalogs: Vec::new(),
            libraries: Vec::new(),
            end: 0,
        };
        if inventory {
            if let Some(directory) = &container.directory {
                for entry in directory.entries() {
                    let name = entry.name();
                    if name.ends_with(".class") {
                        container
                            .classes
                            .try_reserve(1)
                            .map_err(InputError::Allocation)?;
                        container.classes.push(Location {
                            artifact,
                            parents: container.parents.clone(),
                            entry: entry.index().0,
                        });
                    } else if is_nested_catalog(name) {
                        container
                            .catalogs
                            .try_reserve(1)
                            .map_err(InputError::Allocation)?;
                        container.catalogs.push(entry.index());
                    } else if is_layout_library(name) {
                        container
                            .libraries
                            .try_reserve(1)
                            .map_err(InputError::Allocation)?;
                        container.libraries.push(entry.index());
                    }
                }
            } else {
                container.classes.push(Location {
                    artifact,
                    parents: None,
                    entry: 0,
                });
            }
        }
        Ok(container)
    }

    fn archive_view(&self) -> Option<JvmArchive<'_>> {
        Some(JvmArchive {
            artifact: self.artifact,
            bytes: &self.bytes,
            entries: self.directory.as_deref()?.entries(),
            parents: self.parents.as_ref(),
        })
    }
}

pub(super) struct JvmArchive<'a> {
    pub(super) artifact: usize,
    pub(super) bytes: &'a [u8],
    pub(super) entries: &'a [EntryRecord],
    parents: Option<&'a Arc<[usize]>>,
}

impl JvmArchive<'_> {
    pub(super) fn path(&self) -> &[usize] {
        self.parents.map_or(&[], |parents| parents.as_ref())
    }

    pub(super) fn entry_location(&self, entry: &EntryRecord) -> Location {
        Location {
            artifact: self.artifact,
            parents: self.parents.cloned(),
            entry: entry.index().0,
        }
    }
}

pub struct JvmRoots<'a> {
    containers: Vec<Container<'a>>,
    issues: Vec<QueryIssue>,
}

fn project_input(scope: &RequestScope, artifact: usize) -> Result<&InputLease, QueryError> {
    scope
        .jvm_artifact(artifact)
        .ok_or_else(|| unknown_artifact(artifact))?
        .map_err(|error| QueryError::Local {
            kind: error.kind(),
            reason: error.to_string(),
        })
}

fn unknown_artifact(artifact: usize) -> QueryError {
    QueryError::Local {
        kind: FailureKind::Malformed,
        reason: format!("no JVM project artifact a{artifact}"),
    }
}

fn unsupported_source() -> QueryError {
    QueryError::Local {
        kind: FailureKind::Unsupported,
        reason: "JVM locations require a JAR or standalone class".to_owned(),
    }
}

fn source_directory(input: &InputLease, explicit: bool) -> Option<Option<&ArchiveDirectory>> {
    match input.kind() {
        InputKind::BareClass => Some(None),
        InputKind::Archive => {
            let directory = input.archive()?;
            let entries = directory.entries();
            if !explicit
                && entries
                    .iter()
                    .any(|entry| super::apk::is_root_name(entry.name()))
            {
                return None;
            }
            let named_jvm = input
                .identity()
                .path()
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("jar") || extension.eq_ignore_ascii_case("war")
                });
            if explicit || named_jvm {
                return Some(Some(directory));
            }
            let mut manifest = false;
            let mut android = false;
            for entry in entries {
                let name = entry.name();
                if is_class(entry) || is_nested_catalog(name) || is_layout_library(name) {
                    return Some(Some(directory));
                }
                manifest |= name == MANIFEST_NAME;
                android |= matches!(name, "AndroidManifest.xml" | "resources.arsc");
            }
            // APK v1 signatures also use the JAR manifest path.
            (manifest && !android).then_some(Some(directory))
        }
        InputKind::BareDex | InputKind::Opaque => None,
    }
}

impl<'a> JvmRoots<'a> {
    pub fn supports(scope: &RequestScope) -> bool {
        scope.jvm_artifacts().is_some() || source_directory(scope.input(), false).is_some()
    }

    pub fn select(scope: &'a RequestScope) -> Result<Option<Self>, QueryError> {
        Self::select_with_jobs(scope, 1)
    }

    pub fn select_with_jobs(
        scope: &'a RequestScope,
        jobs: usize,
    ) -> Result<Option<Self>, QueryError> {
        let mut roots = Self {
            containers: Vec::new(),
            issues: Vec::new(),
        };
        if let Some(artifacts) = scope.jvm_artifacts() {
            roots
                .containers
                .try_reserve_exact(artifacts.len())
                .map_err(InputError::Allocation)?;
            for artifact in 0..artifacts.len() {
                match project_input(scope, artifact) {
                    Ok(input) => {
                        if let Some(directory) = source_directory(input, true) {
                            roots.containers.push(Container::new(
                                artifact,
                                Cow::Borrowed(input.bytes()),
                                directory.map(Cow::Borrowed),
                                None,
                                true,
                            )?);
                        } else {
                            roots.issues.push(QueryIssue::new(
                                format!("a{artifact}"),
                                &unsupported_source(),
                            ));
                        }
                    }
                    Err(error) => roots
                        .issues
                        .push(QueryIssue::new(format!("a{artifact}"), &error)),
                }
            }
        } else {
            let input = scope.input();
            let Some(directory) = source_directory(input, false) else {
                return Ok(None);
            };
            roots.containers.push(Container::new(
                0,
                Cow::Borrowed(input.bytes()),
                directory.map(Cow::Borrowed),
                None,
                true,
            )?);
        }
        let mut decoder = EntryDecoder::new();
        let mut expanded = 0;
        let mut ordinal = 0;
        while ordinal < roots.containers.len() {
            roots.collect(ordinal, &mut decoder, &mut expanded, jobs)?;
            ordinal += 1;
        }
        let mut end = 0usize;
        for container in &mut roots.containers {
            if let Some((release, directory)) =
                scope.java_version().zip(container.directory.as_deref())
            {
                super::jvm_release::select(
                    &mut container.classes,
                    release,
                    &container.bytes,
                    directory,
                    &mut decoder,
                    &mut roots.issues,
                )?;
            }
            end = end
                .checked_add(container.classes.len())
                .ok_or(InputError::PlatformCapacity)?;
            container.end = end;
        }
        Ok(Some(roots))
    }

    /// Read a class at its explicit physical location.
    pub fn focused(scope: &'a RequestScope, location: &Location) -> Result<Self, QueryError> {
        let mut roots = Self::focus_archive(scope, location)?;
        if roots.by_location(location).is_none() {
            return Err(QueryError::Local {
                kind: FailureKind::Malformed,
                reason: format!("no JVM class at {location}"),
            });
        }
        let container = roots
            .containers
            .last_mut()
            .expect("focused input container");
        container.classes.push(location.clone());
        container.end = 1;
        Ok(roots)
    }

    /// Lease the archive containing the final physical entry. This follows the
    /// declared archive chain without reading unrelated children or validating
    /// the kind of the last entry.
    pub fn focus_archive(scope: &'a RequestScope, location: &Location) -> Result<Self, QueryError> {
        let explicit = scope.jvm_artifacts().is_some();
        let input = if explicit {
            project_input(scope, location.artifact)?
        } else {
            if location.artifact != 0 {
                return Err(unknown_artifact(location.artifact));
            }
            scope.input()
        };
        let directory = source_directory(input, explicit)
            .ok_or_else(unsupported_source)?
            .map(Cow::Borrowed);
        let mut roots = Self {
            containers: vec![Container::new(
                location.artifact,
                Cow::Borrowed(input.bytes()),
                directory,
                None,
                false,
            )?],
            issues: Vec::new(),
        };
        let mut decoder = EntryDecoder::new();
        let mut available = NESTED_BYTES;
        for &index in location.parents.as_deref().unwrap_or_default() {
            let container = roots.containers.last().expect("initial input container");
            let entry = container
                .directory
                .as_deref()
                .and_then(|directory| directory.entry(EntryIndex(index)))
                .ok_or_else(|| QueryError::Local {
                    kind: FailureKind::Malformed,
                    reason: format!("archive chain of {location} has no entry j{index}"),
                })?;
            let declared = is_layout_library(entry.name())
                || container.directory.as_ref().is_some_and(|directory| {
                    directory
                        .entries()
                        .iter()
                        .filter(|item| is_nested_catalog(item.name()))
                        .any(|catalog| {
                            nested_names(&mut decoder, &container.bytes, catalog).is_ok_and(
                                |names| {
                                    names
                                        .into_iter()
                                        .any(|name| name.is_ok_and(|name| name == entry.name()))
                                },
                            )
                        })
                });
            if !declared {
                return Err(QueryError::Local {
                    kind: FailureKind::Malformed,
                    reason: format!("archive chain of {location} has no declared JAR at j{index}"),
                });
            }
            let parent = Location {
                artifact: container.artifact,
                parents: container.parents.clone(),
                entry: index,
            };
            let child = open_nested(
                &mut decoder,
                &container.bytes,
                entry,
                &parent,
                available,
                false,
            )?;
            available -= child.bytes.len();
            roots.containers.push(child);
        }
        Ok(roots)
    }

    fn collect(
        &mut self,
        ordinal: usize,
        decoder: &mut EntryDecoder,
        expanded: &mut usize,
        jobs: usize,
    ) -> Result<(), QueryError> {
        let container = &self.containers[ordinal];
        let Some(directory) = container.directory.as_deref() else {
            return Ok(());
        };
        if container.catalogs.is_empty() && container.libraries.is_empty() {
            return Ok(());
        }
        let mut declarations = Vec::new();
        for &index in &container.catalogs {
            let catalog = directory.entry(index).ok_or(QueryError::WorkerLost)?;
            let context = Location {
                artifact: container.artifact,
                parents: container.parents.clone(),
                entry: catalog.index().0,
            };
            match nested_names(decoder, &container.bytes, catalog) {
                Ok(names) => {
                    declarations
                        .try_reserve(names.len())
                        .map_err(InputError::Allocation)?;
                    for name in names {
                        match name {
                            Ok(name) => declarations.push((context.clone(), name)),
                            Err(error) => self
                                .issues
                                .push(QueryIssue::new(context.to_string(), &error)),
                        }
                    }
                }
                Err(error) => self
                    .issues
                    .push(QueryIssue::new(context.to_string(), &error)),
            }
        }
        let mut matches = HashMap::<&str, Vec<&EntryRecord>>::new();
        matches
            .try_reserve(declarations.len())
            .map_err(InputError::Allocation)?;
        for (_, name) in &declarations {
            matches.entry(name.as_str()).or_default();
        }
        if !matches.is_empty() {
            for entry in directory.entries() {
                if let Some(entries) = matches.get_mut(entry.name()) {
                    entries.try_reserve(1).map_err(InputError::Allocation)?;
                    entries.push(entry);
                }
            }
        }
        let mut seen = HashSet::new();
        let mut selected = Vec::new();
        selected
            .try_reserve_exact(container.libraries.len())
            .map_err(InputError::Allocation)?;
        seen.try_reserve(container.libraries.len())
            .map_err(InputError::Allocation)?;
        for &index in &container.libraries {
            let entry = directory.entry(index).ok_or(QueryError::WorkerLost)?;
            let location = Location {
                artifact: container.artifact,
                parents: container.parents.clone(),
                entry: index.0,
            };
            selected.push((location, entry));
            seen.insert(index.0);
        }
        for (context, name) in &declarations {
            let entries = &matches[name.as_str()];
            if entries.is_empty() {
                self.issues.push(QueryIssue {
                    context: context.to_string(),
                    kind: "Malformed".to_owned(),
                    reason: format!("declared nested JAR {name:?} has no archive entry"),
                });
            }
            for &entry in entries {
                seen.try_reserve(1).map_err(InputError::Allocation)?;
                if !seen.insert(entry.index().0) {
                    continue;
                }
                let location = Location {
                    artifact: container.artifact,
                    parents: container.parents.clone(),
                    entry: entry.index().0,
                };
                selected.try_reserve(1).map_err(InputError::Allocation)?;
                selected.push((location, entry));
            }
        }
        if selected.is_empty() {
            return Ok(());
        }
        let available = NESTED_BYTES.saturating_sub(*expanded);
        let declared = selected.iter().try_fold(0u64, |total, (_, entry)| {
            total.checked_add(entry.uncompressed_size().0)
        });
        let mut children = Vec::new();
        children
            .try_reserve_exact(selected.len())
            .map_err(InputError::Allocation)?;
        if jobs > 1 && declared.is_some_and(|size| size <= available as u64) {
            // The combined declared size fits before workers claim independent
            // entries. Each entry must still match both its length and CRC.
            let mut decoded = Vec::new();
            decoded
                .try_reserve_exact(selected.len())
                .map_err(InputError::Allocation)?;
            super::runner::run(
                selected.len(),
                jobs,
                EntryDecoder::new,
                |decoder, index, emit| {
                    let (location, entry) = &selected[index];
                    let result =
                        open_nested(decoder, &container.bytes, entry, location, available, true);
                    emit((index, result))
                },
                &mut |job| {
                    decoded.push(job);
                    Ok::<_, QueryError>(())
                },
            )?;
            decoded.sort_unstable_by_key(|(index, _)| *index);
            for (index, result) in decoded {
                match result {
                    Ok(child) => {
                        *expanded += child.bytes.len();
                        children.push(child);
                    }
                    Err(error) => self
                        .issues
                        .push(QueryIssue::new(selected[index].0.to_string(), &error)),
                }
            }
        } else {
            // Keep physical selection stable near the shared limit. An
            // unreadable entry does not consume capacity needed by its siblings.
            for (location, entry) in selected {
                match open_nested(
                    decoder,
                    &container.bytes,
                    entry,
                    &location,
                    NESTED_BYTES.saturating_sub(*expanded),
                    true,
                ) {
                    Ok(child) => {
                        *expanded += child.bytes.len();
                        children.push(child);
                    }
                    Err(error) => self
                        .issues
                        .push(QueryIssue::new(location.to_string(), &error)),
                }
            }
        }
        self.containers
            .try_reserve(children.len())
            .map_err(InputError::Allocation)?;
        self.containers.extend(children);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.containers.last().map_or(0, |container| container.end)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn issues(&self) -> &[QueryIssue] {
        &self.issues
    }

    pub fn get(&self, index: usize) -> Option<JvmRoot<'_>> {
        let container = self.containers.get(
            self.containers
                .partition_point(|container| container.end <= index),
        )?;
        let location = container
            .classes
            .get(index - (container.end - container.classes.len()))?;
        self.root_at(container, location)
    }

    pub fn by_location<'r>(&'r self, location: &'r Location) -> Option<JvmRoot<'r>> {
        let container = self.containers.iter().find(|container| {
            container.artifact == location.artifact && container.parents == location.parents
        })?;
        self.root_at(container, location)
    }

    fn root_at<'r>(
        &'r self,
        container: &'r Container<'a>,
        location: &'r Location,
    ) -> Option<JvmRoot<'r>> {
        let entry = match &container.directory {
            Some(directory) => Some(
                directory
                    .entry(EntryIndex(location.entry))
                    .filter(|entry| is_class(entry))?,
            ),
            None if location.entry == 0 => None,
            None => return None,
        };
        Some(JvmRoot {
            bytes: &container.bytes,
            entry,
            location,
        })
    }

    pub(super) fn archives(&self) -> impl Iterator<Item = JvmArchive<'_>> {
        self.containers.iter().filter_map(Container::archive_view)
    }

    pub(super) fn archive_for(&self, location: &Location) -> Option<JvmArchive<'_>> {
        self.containers
            .iter()
            .find(|container| {
                container.artifact == location.artifact && container.parents == location.parents
            })
            .and_then(Container::archive_view)
    }

    pub fn reader(&self) -> JvmReader {
        JvmReader {
            decoder: EntryDecoder::new(),
            scratch: ClassScratch::default(),
        }
    }
}

pub struct JvmReader {
    decoder: EntryDecoder,
    scratch: ClassScratch,
}

impl JvmReader {
    pub fn read<'r>(&'r mut self, root: JvmRoot<'r>) -> Result<ClassFile<'r>, QueryError> {
        let bytes = match root.entry {
            Some(entry) => self.decoder.read(root.bytes, entry, CLASS_BYTES)?,
            None => root.bytes,
        };
        Ok(ClassFile::parse_in(bytes, &mut self.scratch)?)
    }
}

fn is_class(entry: &EntryRecord) -> bool {
    entry.name().ends_with(".class")
}

fn is_nested_catalog(name: &str) -> bool {
    matches!(
        name,
        "META-INF/versions.list" | "META-INF/libraries.list" | "fabric.mod.json"
    )
}

/// Library locations in executable Spring Boot archives and standard web
/// applications. Select the physical records directly so duplicate names
/// retain independent archive coordinates.
fn is_layout_library(name: &str) -> bool {
    if !name.ends_with(".jar") {
        return false;
    }
    ["BOOT-INF/lib/", "WEB-INF/lib/", "WEB-INF/lib-provided/"]
        .iter()
        .filter_map(|prefix| name.strip_prefix(prefix))
        .any(|file| file.len() > ".jar".len() && !file.contains('/'))
}

fn nested_names(
    decoder: &mut EntryDecoder,
    bytes: &[u8],
    entry: &EntryRecord,
) -> Result<Vec<Result<String, QueryError>>, QueryError> {
    let bytes = decoder.read(bytes, entry, METADATA_BYTES)?;
    if entry.name() == "fabric.mod.json" {
        #[derive(serde::Deserialize)]
        struct Manifest {
            #[serde(default)]
            jars: Vec<serde_json::Value>,
        }
        let manifest: Manifest =
            serde_json::from_slice(bytes).map_err(|error| QueryError::Local {
                kind: FailureKind::Malformed,
                reason: format!("invalid fabric.mod.json: {error}"),
            })?;
        return Ok(manifest
            .jars
            .into_iter()
            .map(|jar| {
                jar.get("file")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| QueryError::Local {
                        kind: FailureKind::Malformed,
                        reason: "invalid nested JAR declaration in fabric.mod.json".to_owned(),
                    })
            })
            .collect());
    }
    let prefix = if entry.name() == "META-INF/versions.list" {
        "META-INF/versions/"
    } else {
        "META-INF/libraries/"
    };
    let text = std::str::from_utf8(bytes).map_err(|error| QueryError::Local {
        kind: FailureKind::Malformed,
        reason: format!("invalid nested JAR catalog: {error}"),
    })?;
    Ok(text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| {
            let mut fields = line.split('\t');
            match (fields.next(), fields.next(), fields.next(), fields.next()) {
                (Some(hash), Some(id), Some(name), None)
                    if !hash.is_empty() && !id.is_empty() && name.ends_with(".jar") =>
                {
                    Ok(format!("{prefix}{name}"))
                }
                _ => Err(QueryError::Local {
                    kind: FailureKind::Malformed,
                    reason: format!("invalid row {} in nested JAR catalog", index + 1),
                }),
            }
        })
        .collect())
}

fn open_nested<'a>(
    decoder: &mut EntryDecoder,
    bytes: &Cow<'a, [u8]>,
    entry: &EntryRecord,
    location: &Location,
    limit: usize,
    inventory: bool,
) -> Result<Container<'a>, QueryError> {
    let mut path = [0usize; MAX_NESTING];
    let mut len = 0;
    for component in location.path() {
        let slot = path.get_mut(len).ok_or(QueryError::Local {
            kind: FailureKind::ResourceExhausted,
            reason: "nested JAR depth limit exceeded".to_owned(),
        })?;
        *slot = component;
        len += 1;
    }
    let payload = match bytes {
        Cow::Borrowed(bytes) if entry.method() == 0 => {
            Cow::Borrowed(checked_stored_entry(bytes, entry, limit)?)
        }
        _ => Cow::Owned(decoder.take(bytes, entry, limit)?),
    };
    let directory = ArchiveDirectory::parse(&payload).map_err(InputError::from)?;
    Container::new(
        location.artifact,
        payload,
        Some(Cow::Owned(directory)),
        Some(Arc::from(&path[..len])),
        inventory,
    )
}
