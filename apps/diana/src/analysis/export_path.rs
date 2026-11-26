//! File creation within a private export directory. Callers check each input
//! component before constructing a host path.

use std::fs::{self, File};
use std::io;
use std::path::Path;

/// A component which can safely be appended on all supported host platforms.
/// The stem checks include Windows device names with extensions or superscript digits.
pub(crate) fn portable_component(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 255
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .any(|name| stem.eq_ignore_ascii_case(name))
    {
        return false;
    }
    let Some(prefix) = stem.get(..3) else {
        return true;
    };
    let serial = prefix.eq_ignore_ascii_case("COM") || prefix.eq_ignore_ascii_case("LPT");
    !(serial
        && matches!(
            &stem[3..],
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
        ))
}

pub(crate) fn path_conflict(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::AlreadyExists
            | io::ErrorKind::NotADirectory
            | io::ErrorKind::IsADirectory
            | io::ErrorKind::InvalidFilename
    )
}

pub(crate) fn create_file(root: &Path, relative: &Path) -> io::Result<File> {
    let path = root.join(relative);
    match File::create_new(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            create_directories(root, relative.parent().ok_or(e)?)?;
            File::create_new(path)
        }
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && path.is_dir() => {
            Err(io::ErrorKind::IsADirectory.into())
        }
        result => result,
    }
}

// Build input-derived directory chains iteratively. Sibling class workers can
// legitimately race to create the same package.
pub(crate) fn create_directories(root: &Path, relative: &Path) -> io::Result<()> {
    fn create(path: &Path) -> io::Result<()> {
        match fs::create_dir(path) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
            result => result,
        }
    }

    // Most new packages share an existing parent. Try the requested
    // directory first and walk upward only across missing parents.
    let path = root.join(relative);
    let mut missing = Vec::new();
    for directory in path.ancestors().take_while(|&p| p != root) {
        match create(directory) {
            Ok(()) => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => missing.push(directory),
            Err(error) => return Err(error),
        }
    }
    for directory in missing.into_iter().rev() {
        create(directory)?;
    }
    Ok(())
}
