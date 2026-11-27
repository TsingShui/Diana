use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::apk::{self, DexRoot};
use super::input::{EntryIndex, InputError, InputKind, InputLease, InputOptions, open_input};

/// One CLI request's input ownership.
///
/// The scope leases the input for the whole request and hands out only borrowed
/// root views inside that lifetime. Dropping the scope releases the mapped input
/// and every future temporary owner it holds; it never modifies or
/// deletes the user's input.
#[derive(Debug)]
pub struct RequestScope {
    lease: InputLease,
    project: Option<JvmProject>,
}

/// A physically declared project input. Unavailable artifacts retain their
/// original ordinals and remain independent of their siblings.
#[derive(Debug)]
pub struct JvmArtifact {
    path: Result<PathBuf, InputError>,
    lease: OnceLock<Result<InputLease, InputError>>,
}

impl JvmArtifact {
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref().ok()
    }
}

#[derive(Debug)]
struct JvmProject {
    artifacts: Vec<JvmArtifact>,
    options: InputOptions,
    java_version: Option<u16>,
}

impl JvmProject {
    fn read(lease: &InputLease) -> Result<Self, InputError> {
        const MAX_DESCRIPTOR_BYTES: usize = 1024 * 1024;
        if lease.bytes().len() > MAX_DESCRIPTOR_BYTES {
            return Err(InputError::EntrySizeLimit {
                size: lease.identity().len().0,
                limit: MAX_DESCRIPTOR_BYTES as u64,
            });
        }
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Descriptor {
            archives: Vec<String>,
            java_version: Option<u16>,
        }
        let descriptor: Descriptor = serde_json::from_slice(lease.bytes())
            .map_err(|error| InputError::JvmProject(error.to_string()))?;
        if descriptor.archives.is_empty() || descriptor.java_version == Some(0) {
            return Err(InputError::JvmProject(
                "at least one archive and a positive java_version are required when specified"
                    .to_owned(),
            ));
        }
        let directory = lease.identity().path().parent().ok_or_else(|| {
            InputError::JvmProject("descriptor has no containing directory".to_owned())
        })?;
        let mut artifacts = Vec::new();
        artifacts
            .try_reserve_exact(descriptor.archives.len())
            .map_err(InputError::Allocation)?;
        for name in descriptor.archives {
            let path = if name.split('/').all(super::export_path::portable_component) {
                Ok(directory.join(name))
            } else {
                Err(InputError::JvmProject(
                    "archive entry requires a portable relative path".to_owned(),
                ))
            };
            artifacts.push(JvmArtifact {
                path,
                lease: OnceLock::new(),
            });
        }
        Ok(Self {
            artifacts,
            options: InputOptions {
                allowed_root: Some(directory.to_path_buf()),
            },
            java_version: descriptor.java_version,
        })
    }
}

impl RequestScope {
    pub fn open(path: &Path, options: &InputOptions) -> Result<Self, InputError> {
        let lease = open_input(path, options)?;
        let project = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".diana-jvm.json"))
            .then(|| JvmProject::read(&lease))
            .transpose()?;
        Ok(Self { lease, project })
    }

    pub fn input(&self) -> &InputLease {
        &self.lease
    }

    pub fn jvm_artifacts(&self) -> Option<&[JvmArtifact]> {
        self.project
            .as_ref()
            .map(|project| project.artifacts.as_slice())
    }

    pub fn jvm_artifact(&self, index: usize) -> Option<Result<&InputLease, &InputError>> {
        let project = self.project.as_ref()?;
        let artifact = project.artifacts.get(index)?;
        Some(artifact.path.as_ref().and_then(|path| {
            artifact
                .lease
                .get_or_init(|| {
                    let lease = open_input(path, &project.options)?;
                    let supported = match lease.kind() {
                        InputKind::BareClass => true,
                        InputKind::Archive => lease.archive().is_some_and(|directory| {
                            !directory
                                .entries()
                                .iter()
                                .any(|entry| apk::is_root_name(entry.name()))
                        }),
                        _ => false,
                    };
                    if supported {
                        Ok(lease)
                    } else {
                        Err(InputError::NotJvm)
                    }
                })
                .as_ref()
        }))
    }

    pub fn java_version(&self) -> Option<u16> {
        self.project
            .as_ref()
            .and_then(|project| project.java_version)
    }

    pub fn roots(&self) -> Result<Vec<DexRoot<'_>>, InputError> {
        apk::roots(&self.lease, None)
    }

    /// Materializes only the physical root named by a validated locator prefix.
    /// A missing/non-root record yields no roots, not an invented empty DEX.
    pub(super) fn roots_in_entry(&self, entry: EntryIndex) -> Result<Vec<DexRoot<'_>>, InputError> {
        apk::roots(&self.lease, Some(entry))
    }
}
