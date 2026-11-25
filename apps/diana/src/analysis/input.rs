use std::path::{Path, PathBuf};

use thiserror::Error;

use super::archive::ArchiveDirectory;

/// A byte length in the original input file, distinct from entry and source
/// lengths that also appear in the application.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct InputByteLen(pub u64);

/// A position in a physical ZIP record, not a DEX class-def or method index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EntryIndex(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputKind {
    /// The whole leased input is one DEX container.
    BareDex,
    /// The leased input is a ZIP archive whose entries are available.
    Archive,
    /// A leased regular file that is neither of the above.
    Opaque,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveProblem {
    /// No end-of-central-directory record in the admissible tail.
    MissingDirectory,
    /// A record signature, count or length field contradicts the actual bytes.
    InconsistentDirectory,
    /// A record ends outside the leased bytes.
    OutOfRange,
    /// An entry name is not valid UTF-8 metadata.
    NameEncoding,
    /// The requested entry does not exist at that physical index.
    UnknownEntry,
    /// A stored entry's payload or size contradicts its record.
    DamagedEntry,
    /// The entry uses a compression method this module cannot read.
    UnsupportedCompression(u16),
    /// The entry declares encryption.
    EncryptedEntry,
    /// ZIP64 record fields are outside the supported container profile.
    UnsupportedZip64,
    /// The deflate stream is truncated, over-long or otherwise undecodable.
    DecompressionFailed,
    /// Reserved table capacity could not be allocated.
    Allocation(std::collections::TryReserveError),
}

#[derive(Debug, Error)]
pub enum InputError {
    #[error("input I/O failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("input is not a regular file: {0}")]
    NotRegularFile(PathBuf),
    #[error("input resolves outside the allowed root: {0}")]
    OutsideRoot(PathBuf),
    #[error("input changed while it was read")]
    Changed,
    #[error("archive is malformed: {0:?}")]
    MalformedArchive(ArchiveProblem),
    #[error("input size is not representable on this platform")]
    PlatformCapacity,
    #[error("input allocation failed: {0}")]
    Allocation(#[source] std::collections::TryReserveError),
    #[error("input is not a DEX container or archive")]
    NotDex,
}

impl InputError {
    /// One failure kind for callers that must classify an input-side error.
    /// I/O, path policy and an unsupported container are `Unsupported`; damaged
    /// archive records are `Malformed`; platform capacity and actual allocation
    /// failures are `ResourceExhausted`.
    pub fn kind(&self) -> diana_decomp::FailureKind {
        use diana_decomp::FailureKind as Kind;
        match self {
            Self::PlatformCapacity | Self::Allocation(_) => Kind::ResourceExhausted,
            Self::Io(_)
            | Self::NotRegularFile(_)
            | Self::OutsideRoot(_)
            | Self::Changed
            | Self::NotDex => Kind::Unsupported,
            Self::MalformedArchive(problem) => match problem {
                ArchiveProblem::Allocation(_) => Kind::ResourceExhausted,
                ArchiveProblem::UnsupportedCompression(_)
                | ArchiveProblem::EncryptedEntry
                | ArchiveProblem::UnsupportedZip64
                | ArchiveProblem::NameEncoding => Kind::Unsupported,
                ArchiveProblem::MissingDirectory
                | ArchiveProblem::InconsistentDirectory
                | ArchiveProblem::OutOfRange
                | ArchiveProblem::UnknownEntry
                | ArchiveProblem::DamagedEntry
                | ArchiveProblem::DecompressionFailed => Kind::Malformed,
            },
        }
    }
}

impl From<ArchiveProblem> for InputError {
    fn from(problem: ArchiveProblem) -> Self {
        Self::MalformedArchive(problem)
    }
}

impl From<std::io::Error> for InputError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Borrowed bytes inside a lease or one of its entry views.
pub trait BytesSource {
    fn bytes(&self) -> &[u8];
}

/// The identity of one leased input. It is request-local and never a cache key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputIdentity {
    path: PathBuf,
    len: InputByteLen,
}

impl InputIdentity {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn len(&self) -> InputByteLen {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len.0 == 0
    }
}

/// The bytes of one leased input.
///
/// A regular file is mapped instead of copied, so a 200 MiB archive costs only
/// the pages a request actually reads. The empty input has no mapping to make and
/// keeps an owned empty buffer.
#[derive(Debug)]
enum InputBytes {
    Mapped(memmap2::Mmap),
    Owned(Vec<u8>),
}

impl InputBytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Mapped(mapped) => mapped,
            Self::Owned(bytes) => bytes,
        }
    }
}

/// One leased input: its mapped bytes and its restricted entry metadata.
///
/// The mapping is read-only. Export also borrows the retained file for short-lived windows.
/// The host owns the file: truncating or rewriting it while a request holds this
/// lease is outside the supported input contract.
#[derive(Debug)]
pub struct InputLease {
    identity: InputIdentity,
    kind: InputKind,
    bytes: InputBytes,
    file: std::fs::File,
    archive: Option<ArchiveDirectory>,
}

impl BytesSource for InputLease {
    fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl InputLease {
    pub fn identity(&self) -> &InputIdentity {
        &self.identity
    }
    pub fn kind(&self) -> InputKind {
        self.kind
    }
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
    pub(super) fn window(&self) -> InputWindow<'_> {
        InputWindow {
            lease: self,
            start: 0,
            mapped: InputBytes::Owned(Vec::new()),
        }
    }

    pub fn archive(&self) -> Option<&ArchiveDirectory> {
        self.archive.as_ref()
    }
}

/// A sliding view of the same open file used for the lease. Full archive scans
/// consume small adjacent ranges and release the previous mapping on a miss.
/// This keeps already-copied file pages out of the process's resident set.
pub(super) struct InputWindow<'a> {
    lease: &'a InputLease,
    start: u64,
    mapped: InputBytes,
}

impl InputWindow<'_> {
    pub(super) fn read(&mut self, offset: u64, length: usize) -> Result<&[u8], InputError> {
        let length64 = u64::try_from(length).map_err(|_| InputError::PlatformCapacity)?;
        let end = offset
            .checked_add(length64)
            .filter(|&end| end <= self.lease.identity.len.0)
            .ok_or(InputError::MalformedArchive(ArchiveProblem::OutOfRange))?;
        if offset < self.start || end - self.start > self.mapped.as_slice().len() as u64 {
            const READ_AHEAD: u64 = 1024 * 1024;
            let size = length64
                .max(READ_AHEAD)
                .min(self.lease.identity.len.0 - offset);
            self.mapped = map_input(&self.lease.file, offset, size)?;
            self.start = offset;
        }
        let start =
            usize::try_from(offset - self.start).map_err(|_| InputError::PlatformCapacity)?;
        Ok(&self.mapped.as_slice()[start..start + length])
    }

    pub(super) fn len(&self) -> u64 {
        self.lease.identity.len.0
    }
}

/// Path policy and explicit bounds for one lease.
#[derive(Clone, Debug, Default)]
pub struct InputOptions {
    /// When set, the resolved input must stay inside this directory. The default
    /// is to follow the platform's normal path resolution for a regular file.
    pub allowed_root: Option<PathBuf>,
}

/// Leases one regular file after applying the path and size policy.
///
/// Non-regular files (directories, devices, FIFOs, sockets) are rejected from
/// metadata before any open, so a FIFO cannot block the request. The bytes are
/// mapped, not copied: reading them is left to the request. A file whose length
/// changes around the open yields `Changed`.
pub fn open_input(path: &Path, options: &InputOptions) -> Result<InputLease, InputError> {
    let resolved = std::fs::canonicalize(path).map_err(InputError::Io)?;
    if let Some(root) = &options.allowed_root {
        let root = std::fs::canonicalize(root).map_err(InputError::Io)?;
        if !resolved.starts_with(&root) {
            return Err(InputError::OutsideRoot(resolved));
        }
    }
    let metadata = std::fs::metadata(&resolved).map_err(InputError::Io)?;
    if !metadata.is_file() {
        return Err(InputError::NotRegularFile(resolved));
    }
    let len = metadata.len();
    let file = std::fs::File::open(&resolved).map_err(InputError::Io)?;
    if file.metadata().map_err(InputError::Io)?.len() != len {
        return Err(InputError::Changed);
    }
    let bytes = map_input(&file, 0, len)?;
    if file.metadata().map_err(InputError::Io)?.len() != len {
        return Err(InputError::Changed);
    }
    let kind = classify(bytes.as_slice());
    let archive = match kind {
        InputKind::Archive => {
            Some(ArchiveDirectory::parse(bytes.as_slice()).map_err(InputError::MalformedArchive)?)
        }
        _ => None,
    };
    Ok(InputLease {
        identity: InputIdentity {
            path: resolved,
            len: InputByteLen(len),
        },
        kind,
        bytes,
        file,
        archive,
    })
}

/// Maps the leased file read-only.
///
/// This is the only `unsafe` in the application crate: mapping is an inherently
/// unsafe operation because the bytes are outside Rust's aliasing model. The
/// obligations this function relies on are that the mapping is never written and
/// that the file is not truncated for as long as the lease lives; both belong to
/// the host.
#[allow(unsafe_code)]
fn map_input(file: &std::fs::File, offset: u64, len: u64) -> Result<InputBytes, InputError> {
    if len == 0 {
        return Ok(InputBytes::Owned(Vec::new()));
    }
    let length = usize::try_from(len).map_err(|_| InputError::PlatformCapacity)?;
    // SAFETY: the range is inside the leased regular file. The host keeps its
    // contents stable for the lease, and every mapping is read-only.
    let mapped = unsafe {
        memmap2::MmapOptions::new()
            .offset(offset)
            .len(length)
            .map(file)
    }
    .map_err(InputError::Io)?;
    if mapped.len() != length {
        return Err(InputError::Changed);
    }
    Ok(InputBytes::Mapped(mapped))
}

fn classify(bytes: &[u8]) -> InputKind {
    if bytes.starts_with(b"dex\n") {
        InputKind::BareDex
    } else if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x06\x06")
    {
        InputKind::Archive
    } else {
        InputKind::Opaque
    }
}
