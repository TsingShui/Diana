use diana::analysis::{
    archive::{self, ArchiveDirectory},
    input::{EntryIndex, InputKind, InputOptions},
    manifest,
    request::RequestScope,
};
use diana_decomp::{
    ByteOffset, ClassDefId, ClassKey, DexId, DexImage, Engine, FailureKind, ImageOrigin, InputId,
    Request, input::DexContainer, p9::RenderOutcome,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

#[derive(Clone, Copy)]
pub enum ImageSource {
    Bare,
    Archive(EntryIndex),
}

#[derive(Clone)]
pub struct Image {
    pub label: String,
    pub header: ByteOffset,
    pub source: ImageSource,
}

#[derive(Clone)]
pub struct Class {
    pub image: usize,
    pub header: ByteOffset,
    pub id: ClassDefId,
    pub label: String,
    pub kind: ClassKind,
}

#[derive(Clone, Copy)]
pub enum ClassKind {
    Class,
    Interface,
    Enum,
    Annotation,
}

impl ClassKind {
    fn from_flags(flags: u32) -> Self {
        if flags & 0x2000 != 0 {
            Self::Annotation
        } else if flags & 0x200 != 0 {
            Self::Interface
        } else if flags & 0x4000 != 0 {
            Self::Enum
        } else {
            Self::Class
        }
    }
}

pub struct PackageGroup {
    pub name: String,
    pub classes: Vec<usize>,
    pub children: Vec<usize>,
}

#[derive(Clone)]
pub struct ManifestData {
    pub xml: String,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub struct Library {
    pub entry: EntryIndex,
    pub path: String,
    pub abi: String,
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

pub struct LibraryGroup {
    pub abi: String,
    pub libraries: Vec<usize>,
}

pub struct LibraryInspection {
    pub text: String,
    pub warnings: Vec<String>,
}

pub struct Resource {
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub archive: Option<ArchiveDirectory>,
    pub images: Vec<Image>,
    pub classes: Vec<Class>,
    pub manifest: Option<ManifestData>,
    pub libraries: Vec<Library>,
    pub library_groups: Vec<LibraryGroup>,
    pub groups: Vec<PackageGroup>,
    pub roots: Vec<usize>,
    class_index: HashMap<(usize, ByteOffset, ClassDefId), usize>,
    pub issues: Vec<String>,
}

impl Resource {
    pub fn class(&self, image: usize, header: ByteOffset, id: ClassDefId) -> Option<Class> {
        self.classes
            .get(*self.class_index.get(&(image, header, id))?)
            .cloned()
    }
}

pub fn open(path: PathBuf) -> Result<Resource, String> {
    let scope =
        RequestScope::open(&path, &InputOptions::default()).map_err(|error| error.to_string())?;
    let kind = scope.input().kind();
    if matches!(kind, InputKind::Opaque) {
        return Err("Choose an APK or DEX file.".into());
    }

    let name = scope.input().identity().path().display().to_string();
    let is_archive = matches!(kind, InputKind::Archive);
    let archive = scope.input().archive().cloned();
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(scope.input().bytes().len())
        .map_err(|error| format!("Cannot reserve input bytes: {error}"))?;
    bytes.extend_from_slice(scope.input().bytes());
    let bytes: Arc<[u8]> = bytes.into();
    let mut issues = Vec::new();

    let manifest = if is_archive {
        match manifest::read(&scope) {
            Ok(manifest) => Some(ManifestData {
                xml: manifest.xml().to_owned(),
                warnings: manifest
                    .warnings()
                    .iter()
                    .map(|warning| format!("byte {}: {}", warning.offset, warning.kind.describe()))
                    .collect(),
            }),
            Err(manifest::ManifestSelectionError::Missing) => None,
            Err(error) => {
                if matches!(
                    error.kind(),
                    FailureKind::ResourceExhausted | FailureKind::InternalInvariant
                ) {
                    return Err(error.to_string());
                }
                issues.push(format!("AndroidManifest.xml: {error}"));
                None
            }
        }
    } else {
        None
    };

    let mut libraries = Vec::new();
    if let Some(directory) = archive.as_ref() {
        for entry in directory.entries() {
            let Some(rest) = entry.name().strip_prefix("lib/") else {
                continue;
            };
            let Some((abi, name)) = rest.split_once('/') else {
                continue;
            };
            if abi.is_empty() || name.is_empty() || !name.ends_with(".so") {
                continue;
            }
            libraries
                .try_reserve(1)
                .map_err(|error| error.to_string())?;
            libraries.push(Library {
                entry: entry.index(),
                path: entry.name().to_owned(),
                abi: abi.to_owned(),
                name: name.to_owned(),
                compressed_size: entry.compressed_size().0,
                uncompressed_size: entry.uncompressed_size().0,
            });
        }
    }
    let mut library_groups = Vec::new();
    let mut abi_index = HashMap::new();
    for (index, library) in libraries.iter().enumerate() {
        let group = if let Some(&group) = abi_index.get(&library.abi) {
            group
        } else {
            library_groups
                .try_reserve(1)
                .map_err(|error| error.to_string())?;
            abi_index
                .try_reserve(1)
                .map_err(|error| error.to_string())?;
            let group = library_groups.len();
            library_groups.push(LibraryGroup {
                abi: library.abi.clone(),
                libraries: Vec::new(),
            });
            abi_index.insert(library.abi.clone(), group);
            group
        };
        library_groups[group]
            .libraries
            .try_reserve(1)
            .map_err(|error| error.to_string())?;
        library_groups[group].libraries.push(index);
    }

    let roots = scope.roots().map_err(|error| error.to_string())?;
    if roots.is_empty() {
        return Err(if is_archive {
            "No classes.dex or classes<N>.dex entries were found in this APK.".into()
        } else {
            "This file is not a DEX container.".into()
        });
    }

    let mut images = Vec::new();
    let mut classes = Vec::new();
    let mut scratch = Vec::new();
    for root in roots {
        let label = root.label().to_owned();
        let source = root.entry().map_or(ImageSource::Bare, |entry| {
            ImageSource::Archive(entry.index())
        });
        let payload = match root.bytes_into(&mut scratch) {
            Ok(payload) => payload,
            Err(error) => {
                if fatal_input(&error) {
                    return Err(error.to_string());
                }
                issues.push(format!("{label}: {error}"));
                continue;
            }
        };
        let container = match DexContainer::open(payload) {
            Ok(container) => container,
            Err(error) => {
                if fatal(&error) {
                    return Err(error.to_string());
                }
                issues.push(format!("{label}: {error}"));
                continue;
            }
        };
        let headers = match container.logical_headers() {
            Ok(headers) => headers,
            Err(error) => {
                if fatal(&error) {
                    return Err(error.to_string());
                }
                issues.push(format!("{label}: {error}"));
                continue;
            }
        };
        for header in headers {
            let image = images.len();
            images.push(Image {
                label: label.clone(),
                header,
                source,
            });
            let program = match container.program(header) {
                Ok(program) => program,
                Err(error) => {
                    if fatal(&error) {
                        return Err(error.to_string());
                    }
                    issues.push(format!("{label} at {}: {error}", header.0));
                    continue;
                }
            };
            let visit = program.visit_classes(|id, result| {
                match result {
                    Ok(class) => match class.descriptor.units() {
                        Ok(units) => {
                            classes.try_reserve(1).map_err(|_| {
                                diana_decomp::input::InputError::Allocation {
                                    offset_byte: header,
                                }
                            })?;
                            classes.push(Class {
                                image,
                                header,
                                id,
                                label: String::from_utf16_lossy(&units),
                                kind: ClassKind::from_flags(class.access_flags),
                            });
                        }
                        Err(error) => {
                            if fatal(&error) {
                                return Err(error);
                            }
                            issues.push(format!("class {}: {error}", id.0));
                        }
                    },
                    Err(error) => issues.push(format!("class {}: {error}", id.0)),
                }
                Ok(())
            });
            if let Err(error) = visit {
                if fatal(&error) {
                    return Err(error.to_string());
                }
                issues.push(format!("{label} at {}: {error}", header.0));
            }
        }
    }

    let mut groups: Vec<PackageGroup> = Vec::new();
    let mut roots = Vec::new();
    let mut package_index = HashMap::new();
    let mut class_index = HashMap::new();
    for (index, class) in classes.iter().enumerate() {
        class_index
            .try_reserve(1)
            .map_err(|error| error.to_string())?;
        class_index.insert((class.image, class.header, class.id), index);
        let descriptor = class
            .label
            .strip_prefix('L')
            .and_then(|value| value.strip_suffix(';'))
            .unwrap_or(&class.label);
        let package = descriptor
            .rsplit_once('/')
            .map_or("", |(package, _)| package);
        let mut parent: Option<usize> = None;
        for component in package.split('/') {
            let key = (parent, package.is_empty(), component.to_owned());
            let node = if let Some(&node) = package_index.get(&key) {
                node
            } else {
                groups.try_reserve(1).map_err(|error| error.to_string())?;
                package_index
                    .try_reserve(1)
                    .map_err(|error| error.to_string())?;
                let node = groups.len();
                let siblings = match parent {
                    Some(parent) => &mut groups[parent].children,
                    None => &mut roots,
                };
                siblings.try_reserve(1).map_err(|error| error.to_string())?;
                siblings.push(node);
                groups.push(PackageGroup {
                    name: if package.is_empty() {
                        "(default package)".to_owned()
                    } else {
                        component.to_owned()
                    },
                    classes: Vec::new(),
                    children: Vec::new(),
                });
                package_index.insert(key, node);
                node
            };
            parent = Some(node);
        }
        let node = parent.ok_or("package has no component")?;
        groups[node]
            .classes
            .try_reserve(1)
            .map_err(|error| error.to_string())?;
        groups[node].classes.push(index);
    }

    Ok(Resource {
        name,
        bytes,
        archive,
        images,
        classes,
        manifest,
        libraries,
        library_groups,
        groups,
        roots,
        class_index,
        issues,
    })
}

fn fatal_input(error: &diana::analysis::input::InputError) -> bool {
    matches!(
        error.kind(),
        FailureKind::ResourceExhausted | FailureKind::InternalInvariant
    )
}

fn fatal(error: &diana_decomp::input::InputError) -> bool {
    matches!(
        error.kind(),
        FailureKind::ResourceExhausted | FailureKind::InternalInvariant
    )
}

pub fn decompile(
    bytes: Arc<[u8]>,
    archive: Option<ArchiveDirectory>,
    image: Image,
    class: Class,
) -> Result<String, String> {
    let payload;
    let dex_bytes = match image.source {
        ImageSource::Bare => bytes.as_ref(),
        ImageSource::Archive(index) => {
            let archive = archive
                .as_ref()
                .ok_or_else(|| "The APK directory is no longer available.".to_owned())?;
            let entry = archive
                .entry(index)
                .ok_or_else(|| "The selected DEX entry is no longer available.".to_owned())?;
            payload = archive::read_entry(&bytes, entry)
                .map_err(|error| format!("Cannot read {} from the APK: {error:?}", image.label))?;
            payload.bytes()
        }
    };
    let engine = Engine::open(vec![DexImage {
        bytes: dex_bytes,
        header_offset: image.header,
        origin: ImageOrigin { input: InputId(0) },
    }])
    .map_err(|error| error.to_string())?;
    let result = engine
        .analyze(Request {
            class: ClassKey {
                dex: DexId(0),
                class_def: class.id,
            },
            method: None,
        })
        .map_err(|error| error.to_string())?;
    match result.rendered {
        RenderOutcome::Ready(value) => {
            let mut text = value.text().to_owned();
            for issue in value.issues() {
                text.push_str(&format!(
                    "\n/* {:?}: {} ({:?}) */",
                    issue.kind, issue.detail, issue.cause
                ));
            }
            Ok(text)
        }
        RenderOutcome::Stopped(value) => Ok(format!(
            "{}\n\n/* Rendering stopped: {} */",
            value.text_prefix(),
            value.error()
        )),
    }
}

pub fn inspect_library(
    bytes: Arc<[u8]>,
    archive: Option<ArchiveDirectory>,
    library: Library,
) -> Result<LibraryInspection, String> {
    let archive = archive.ok_or_else(|| "The APK directory is no longer available.".to_owned())?;
    let entry = archive
        .entry(library.entry)
        .ok_or_else(|| "The selected library entry is no longer available.".to_owned())?;
    let payload = archive::read_entry(&bytes, entry)
        .map_err(|error| format!("Cannot read {} from the APK: {error:?}", library.path))?;

    let mut text = format!(
        "Path: {}\nABI: {}\nAPK entry: {}\nCompressed size: {}\nUncompressed size: {}\n",
        library.path,
        library.abi,
        library.entry.0,
        human_bytes(library.compressed_size),
        human_bytes(library.uncompressed_size),
    );
    let mut warnings = Vec::new();
    match elf_summary(payload.bytes()) {
        Ok(summary) => text.push_str(&format!("ELF: {summary}\n")),
        Err(error) => warnings.push(error),
    }
    Ok(LibraryInspection { text, warnings })
}

fn elf_summary(bytes: &[u8]) -> Result<String, String> {
    if !bytes.starts_with(b"\x7fELF") {
        return Err("The selected library payload is not an ELF image.".into());
    }
    let class = match bytes.get(4).copied() {
        Some(1) => "ELF32",
        Some(2) => "ELF64",
        Some(value) => return Err(format!("Unsupported ELF class {value}.")),
        None => return Err("The ELF header is truncated.".into()),
    };
    let little_endian = match bytes.get(5).copied() {
        Some(1) => true,
        Some(2) => false,
        Some(value) => return Err(format!("Unsupported ELF byte order {value}.")),
        None => return Err("The ELF header is truncated.".into()),
    };
    let endian = if little_endian {
        "little-endian"
    } else {
        "big-endian"
    };
    let kind = elf_u16(bytes, 16, little_endian)
        .ok_or_else(|| "The ELF type field is truncated.".to_owned())?;
    let machine = elf_u16(bytes, 18, little_endian)
        .ok_or_else(|| "The ELF machine field is truncated.".to_owned())?;
    let kind = match kind {
        1 => "relocatable",
        2 => "executable",
        3 => "shared object",
        4 => "core",
        _ => "other",
    };
    let machine = match machine {
        3 => "x86",
        8 => "MIPS",
        40 => "ARM",
        62 => "x86-64",
        183 => "AArch64",
        243 => "RISC-V",
        _ => "unknown machine",
    };
    Ok(format!("{class} · {endian} · {machine} · {kind}"))
}

fn elf_u16(bytes: &[u8], offset: usize, little_endian: bool) -> Option<u16> {
    let value = bytes.get(offset..offset + 2)?;
    Some(if little_endian {
        u16::from_le_bytes([value[0], value[1]])
    } else {
        u16::from_be_bytes([value[0], value[1]])
    })
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.1} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}
