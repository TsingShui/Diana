use std::sync::OnceLock;

use super::archive::{self, EntryPayload, EntryRecord, read_entry};
use super::input::{ArchiveProblem, EntryIndex, InputError, InputKind, InputLease};

/// One root DEX image borrowed from a lease. `entry` is `None` only for a bare
/// DEX input; `label` is display metadata and never decides identity.
///
/// The payload is inflated on first use and kept for the rest of the request, so
/// the prepared roots cost one directory walk and nothing else. `OnceLock` also
/// makes a repeated scan (an `xref` frontier level, a second query) reuse the
/// bytes instead of inflating the same entry again.
#[derive(Debug)]
pub struct DexRoot<'a> {
    label: String,
    entry: Option<&'a EntryRecord>,
    /// Original leased bytes; offsets remain container-relative for DEX 041.
    source: &'a [u8],
    payload: OnceLock<Result<EntryPayload<'a>, ArchiveProblem>>,
}

impl<'a> DexRoot<'a> {
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn entry(&self) -> Option<&'a EntryRecord> {
        self.entry
    }
    /// The container bytes, inflating the entry on first use.
    ///
    /// A damaged or undescribable payload fails here rather than during root
    /// selection, so one unreadable entry cannot stop a scan that never reads it.
    pub fn bytes(&self) -> Result<&[u8], InputError> {
        let Some(entry) = self.entry else {
            return Ok(self.source);
        };
        match self.payload.get_or_init(|| read_entry(self.source, entry)) {
            Ok(payload) => Ok(payload.bytes()),
            Err(problem) => Err(InputError::MalformedArchive(problem.clone())),
        }
    }

    /// The declared size of this container, or the source length for a bare DEX.
    pub fn declared_len(&self) -> usize {
        self.entry.map_or(self.source.len(), |entry| {
            usize::try_from(entry.uncompressed_size().0).unwrap_or(usize::MAX)
        })
    }

    /// A prefix cursor bound to this root and one worker's scratch. Compressed
    /// growth resumes the stream; stored/bare input simply lends its exact bytes.
    pub(super) fn prefix_reader<'s>(
        &'s self,
        scratch: &'s mut Vec<u8>,
    ) -> Result<RootPrefix<'s>, InputError> {
        let Some(entry) = self.entry else {
            return Ok(RootPrefix::Borrowed(self.source));
        };
        if entry.method() == 0 {
            return Ok(RootPrefix::Borrowed(
                archive::entry_payload(self.source, entry).map_err(InputError::MalformedArchive)?,
            ));
        }
        Ok(RootPrefix::Deflate(
            archive::PrefixReader::new(self.source, entry, scratch)
                .map_err(InputError::MalformedArchive)?,
        ))
    }

    /// The container bytes, inflating into `scratch` instead of retaining them.
    ///
    /// A single-pass scan reads every root once and needs none of them again, so
    /// keeping one payload per root would cost the size of the whole DEX set. The
    /// scratch buffer belongs to the worker and is reused across roots; the bytes
    /// borrow from it and are dropped when the root's scan ends.
    pub fn bytes_into<'s>(&'s self, scratch: &'s mut Vec<u8>) -> Result<&'s [u8], InputError> {
        let Some(entry) = self.entry else {
            return Ok(self.source);
        };
        if entry.method() == 0 {
            return archive::entry_payload(self.source, entry)
                .map_err(InputError::MalformedArchive);
        }
        let written = archive::inflate_into(self.source, entry, scratch)
            .map_err(InputError::MalformedArchive)?;
        Ok(&scratch[..written])
    }
}

pub(super) enum RootPrefix<'s> {
    Borrowed(&'s [u8]),
    Deflate(archive::PrefixReader<'s, 's>),
}

impl RootPrefix<'_> {
    pub(super) fn read(&mut self, aim: usize) -> Result<&[u8], InputError> {
        match self {
            Self::Borrowed(bytes) => Ok(&bytes[..aim.min(bytes.len())]),
            Self::Deflate(reader) => reader.read(aim).map_err(InputError::MalformedArchive),
        }
    }
}

/// Selects the root DEX images in physical directory order.
///
/// A bare DEX is one root labelled with the input file name. An archive
/// contributes `classes.dex` and `classes<index>.dex` records; a duplicate root
/// name still yields one root per physical record, in directory order, rather than
/// being deduplicated by name. Payloads are not inflated here: an explicit
/// physical entry restricts later reads, and an unrelated damaged entry is not a
/// dependency of a scan that never touches it.
pub(super) fn roots(
    lease: &InputLease,
    selected: Option<EntryIndex>,
) -> Result<Vec<DexRoot<'_>>, InputError> {
    match lease.kind() {
        InputKind::BareDex => {
            if selected.is_some_and(|entry| entry.0 != 0) {
                return Ok(Vec::new());
            }
            let label = lease.identity().path().file_name().map_or_else(
                || "input.dex".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            Ok(vec![DexRoot {
                label,
                entry: None,
                source: lease.bytes(),
                payload: OnceLock::new(),
            }])
        }
        InputKind::Archive => {
            let directory = lease.archive().ok_or(InputError::MalformedArchive(
                ArchiveProblem::InconsistentDirectory,
            ))?;
            let mut roots = Vec::new();
            for entry in directory.entries() {
                if selected.is_some_and(|index| index != entry.index())
                    || !is_root_name(entry.name())
                {
                    continue;
                }
                let payload = OnceLock::new();
                roots.try_reserve(1).map_err(InputError::Allocation)?;
                roots.push(DexRoot {
                    label: entry.name().to_owned(),
                    entry: Some(entry),
                    source: lease.bytes(),
                    payload,
                });
            }
            Ok(roots)
        }
        InputKind::Opaque => Err(InputError::NotDex),
    }
}

/// `classes.dex` or `classes<N>.dex` with `N >= 2` and no leading zero, matching
/// the root naming rule. A bare other name is not a root.
pub fn is_root_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("classes") else {
        return false;
    };
    let Some(index) = rest.strip_suffix(".dex") else {
        return false;
    };
    if index.is_empty() {
        return true;
    }
    // The first root is `classes.dex`; split archives continue at `classes2.dex`.
    if index == "1" {
        return false;
    }
    match index.as_bytes() {
        [b'0', ..] => false,
        _ => index.bytes().all(|byte| byte.is_ascii_digit()),
    }
}
