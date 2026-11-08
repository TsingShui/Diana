//! Checked result publication to stdout or an atomic file replacement.

mod buffer;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::analysis::input::InputError;
use crate::analysis::query::QueryError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Format {
    Text,
    Json,
}

pub struct Failure {
    pub code: u8,
    pub message: String,
}

impl Failure {
    pub fn input(error: InputError) -> Self {
        Self {
            code: 1,
            message: error.to_string(),
        }
    }
    pub fn query(error: QueryError, code: u8) -> Self {
        Self {
            code,
            message: error.to_string(),
        }
    }
    pub(super) fn output(error: std::io::Error) -> Self {
        Self {
            code: 1,
            message: format!("result output failed: {error}"),
        }
    }
}

/// Writes an already encoded payload without accepting a truncated successful write.
pub fn publish(
    output: Option<&Path>,
    input: &Path,
    bytes: &[u8],
    format: Format,
) -> Result<(), Failure> {
    publish_with(output, input, |writer| {
        writer.write_all(bytes).map_err(Failure::output)?;
        if output.is_none() && format == Format::Text && !bytes.ends_with(b"\n") {
            writer.write_all(b"\n").map_err(Failure::output)?;
        }
        Ok(())
    })
}

/// Streams through one fallibly allocated transfer buffer. A writer/flush/rename
/// failure, including a closed pipe, is an error. Callbacks handle short writes
/// and propagate write errors.
/// Files become visible only after complete encoding and explicit flush;
/// cleanup also runs when the serializer, rather than the file writer, fails.
/// The concrete writer keeps token-sized serialization writes inlineable.
pub fn publish_with(
    output: Option<&Path>,
    input: &Path,
    write: impl FnOnce(&mut buffer::Buffered<'_>) -> Result<(), Failure>,
) -> Result<(), Failure> {
    match output {
        Some(path) => {
            check_target(path, input).map_err(Failure::output)?;
            let directory = path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let (mut file, temporary) = tempfile_in(directory).map_err(Failure::output)?;
            let written = write_buffered(&mut file, write);
            // Close before cleanup: Windows does not unlink an open temporary.
            drop(file);
            written?;
            std::fs::rename(&temporary.0, path).map_err(Failure::output)
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            write_buffered(&mut stdout, write)
        }
    }
}

fn write_buffered(
    sink: &mut dyn Write,
    write: impl FnOnce(&mut buffer::Buffered<'_>) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let mut writer = buffer::Buffered::new(sink)?;
    let encoded = write(&mut writer);
    // Retain a serializer's available prefix, but never retry a failed sink.
    // The original callback failure takes precedence over a later flush failure.
    let flushed = if !writer.failed() || encoded.is_ok() {
        writer.flush().map_err(Failure::output)
    } else {
        Ok(())
    };
    encoded?;
    flushed
}

fn check_target(target: &Path, input: &Path) -> std::io::Result<()> {
    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        return Err(std::io::Error::other("output target is not a regular file"));
    }
    let aliases = target.canonicalize()? == input.canonicalize()?;
    #[cfg(unix)]
    let aliases = {
        use std::os::unix::fs::MetadataExt;
        let original = std::fs::metadata(input)?;
        aliases || (metadata.dev() == original.dev() && metadata.ino() == original.ino())
    };
    if aliases {
        return Err(std::io::Error::other("output target aliases the input"));
    }
    Ok(())
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn tempfile_in(directory: &Path) -> std::io::Result<(std::fs::File, Temporary)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    for _ in 0..32 {
        let ordinal = COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = directory.join(format!(".diana-{}-{ordinal}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(file) => return Ok((file, Temporary(candidate))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no unique temporary name",
    ))
}
