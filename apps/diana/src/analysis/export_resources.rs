//! Writes ordinary files inside an export's private staging directory. ZIP
//! modes, links, and absolute or parent-relative paths never control the sink.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::apk::is_root_name;
use super::archive::{CopyError, EntryRecord, ExportReader};
use super::axml;
use super::export_path::{create_directories, create_file, path_conflict, portable_component};
use super::input::{InputOptions, open_input};
use super::manifest::{MANIFEST_NAME, selected};
use super::request::RequestScope;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceNotice {
    /// The full payload has a physical-index path.
    Relocated,
    /// The raw payload is complete; its optional XML presentation has a diagnostic.
    Warning,
    /// Input damage or an unsupported input prevented complete delivery.
    Issue,
}

enum Target {
    Directory,
    File(File),
}

fn create_target(root: &Path, name: &str, directory: bool) -> io::Result<(PathBuf, Target)> {
    if !name.split('/').all(portable_component) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidFilename,
            "nonportable archive name",
        ));
    }
    let relative = Path::new(name);
    let target = if directory {
        create_directories(root, relative)?;
        Target::Directory
    } else {
        Target::File(create_file(root, relative)?)
    };
    Ok((root.join(relative), target))
}

/// Writes non-root-DEX payloads and the selected readable manifest. The caller
/// owns the new staging directory and its final publication. Input issues leave
/// unrelated files available. An output or reporting failure aborts publication.
pub fn write(
    scope: &RequestScope,
    stage: &Path,
    mut notice: impl FnMut(&EntryRecord, ResourceNotice, &str) -> io::Result<()>,
) -> io::Result<usize> {
    let Some(archive) = scope.input().archive() else {
        return Ok(0);
    };
    let resource_root = stage.join("resources");
    fs::create_dir(&resource_root)?;
    let manifest = selected(archive).map(EntryRecord::index);
    let mut copier = ExportReader::new(scope.input());
    let mut count = 0;
    for entry in archive
        .entries()
        .iter()
        .filter(|entry| !is_root_name(entry.name()))
    {
        let directory = entry.name().ends_with('/') && entry.uncompressed_size().0 == 0;
        if directory {
            match copier.copy(entry, io::sink()) {
                Ok(()) => {}
                Err(CopyError::Output(error)) => return Err(error),
                Err(error) => {
                    notice(entry, ResourceNotice::Issue, &error.to_string())?;
                    continue;
                }
            }
        }
        let name = if directory {
            entry.name().strip_suffix('/').unwrap_or(entry.name())
        } else {
            entry.name()
        };
        let (path, target) = match create_target(&resource_root, name, directory) {
            Ok(output) => output,
            Err(error) if path_conflict(&error) => {
                let relative = format!("resource-entries/{}", entry.index().0);
                let target = create_target(stage, &relative, directory)?;
                notice(
                    entry,
                    ResourceNotice::Relocated,
                    &format!("placed at {relative}: {error}"),
                )?;
                target
            }
            Err(error) => return Err(error),
        };
        let Target::File(mut file) = target else {
            continue;
        };
        match copier.copy(entry, &mut file) {
            Ok(()) => file.flush()?,
            Err(CopyError::Output(error)) => return Err(error),
            Err(error) => {
                drop(file);
                fs::remove_file(&path)?;
                notice(entry, ResourceNotice::Issue, &error.to_string())?;
                continue;
            }
        }
        count += 1;
        if Some(entry.index()) == manifest {
            drop(file);
            write_manifest(&path, &stage.join(MANIFEST_NAME), |message| {
                notice(entry, ResourceNotice::Warning, message)
            })?;
        }
    }
    Ok(count)
}

fn write_manifest(
    raw: &Path,
    target: &Path,
    mut issue: impl FnMut(&str) -> io::Result<()>,
) -> io::Result<()> {
    // Reuse the verified-length staged payload. This mapping avoids a second
    // inflate and allocating a manifest buffer from an untrusted ZIP length.
    let input = match open_input(raw, &InputOptions::default()) {
        Ok(input) => input,
        Err(error) => return issue(&format!("cannot open the manifest for decoding: {error}")),
    };
    let bytes = input.bytes();
    let text = bytes
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(bytes)
        .trim_ascii_start();
    if text.starts_with(b"<") || bytes.starts_with(b"\xff\xfe") || bytes.starts_with(b"\xfe\xff") {
        fs::copy(raw, target)?;
        return Ok(());
    }
    match axml::decode(bytes) {
        Ok(xml) => {
            fs::write(target, xml.xml())?;
            for warning in xml.warnings() {
                issue(&format!(
                    "manifest byte {}: {}",
                    warning.offset,
                    warning.kind.describe()
                ))?;
            }
            Ok(())
        }
        Err(error) => issue(&error.to_string()),
    }
}
