//! Logical DEX identities over borrowed physical input roots.
//!
//! Engine IDs follow the logical image vector, not physical root ordinals.
//! Every image retains its original header offset and physical entry.

use diana_decomp::input::{DexContainer, PrefixClasses, ProgramIndex, prefix};
use diana_decomp::{ByteOffset, ClassDefId, DexId, Engine, ImageLoader, ImageOrigin, InputId};

use super::apk::DexRoot;
use super::input::{EntryIndex, InputError};
use super::request::RequestScope;
use super::runner;
use super::selector::{ClassSelector, SelectorError};

/// Original header-chain ordinal within one physical DEX container.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct LogicalDex(pub u32);

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error(transparent)]
    Input(#[from] InputError),
    #[error(transparent)]
    Engine(#[from] diana_decomp::input::InputError),
    #[error("image workers stopped before reporting every root")]
    WorkerLost,
}

impl From<super::runner::EmitFailed> for ImageError {
    fn from(_: super::runner::EmitFailed) -> Self {
        Self::WorkerLost
    }
}

/// One logical DEX inside the leased input.
#[derive(Debug)]
pub(super) struct ImageRef {
    pub dex: DexId,
    pub entry: Option<EntryIndex>,
    pub logical: LogicalDex,
    pub header_offset: ByteOffset,
    root: usize,
}

/// All logical images in physical directory/header-chain order.
#[derive(Debug)]
pub(super) struct ImageSet<'a> {
    roots: Vec<DexRoot<'a>>,
    images: Vec<ImageRef>,
    /// Roots a whole-input index could not read. The outline keeps them as
    /// empty images and reports each one instead of dropping the request.
    index_issues: Vec<String>,
}

impl<'a> ImageSet<'a> {
    /// Materializes the requested physical roots and their logical headers.
    pub fn open(
        scope: &'a RequestScope,
        entry: Option<u32>,
        jobs: usize,
    ) -> Result<Self, ImageError> {
        Ok(Self::scan(scope, entry, jobs, None, false)?.0)
    }

    /// Opens the requested roots and resolves `selector` from the same pass.
    ///
    /// Reading headers and scanning declarations are one traversal: a root that
    /// loses resolution is never retained, so peak memory follows the image that
    /// answered rather than the whole multi-DEX set.
    pub(super) fn resolve(
        scope: &'a RequestScope,
        selector: &ClassSelector,
        jobs: usize,
    ) -> Result<(Self, Selection), SelectorError> {
        let (set, scanned, _) = Self::scan(scope, selector.entry, jobs, Some(selector), false)?;
        let selection = set.select(scanned)?;
        Ok((set, selection))
    }

    /// Opens every root and resolves `selector` from the same pass.
    ///
    /// The returned index names every class definition in the input, so the
    /// engine can follow a call into another image without opening all of them
    /// up front. Resolution still considers only the roots the locator names,
    /// so an entry-scoped selector keeps its original ambiguity rules.
    pub(super) fn resolve_indexed(
        scope: &'a RequestScope,
        selector: &ClassSelector,
        jobs: usize,
    ) -> Result<(Self, Selection, ClassIndex), SelectorError> {
        let (set, scanned, locations) =
            Self::scan(scope, selector.entry, jobs, Some(selector), true)?;
        let selection = set.select(scanned)?;
        Ok((set, selection, locations))
    }

    /// One lazy engine loader per logical image: bytes are materialized when a
    /// command first reaches that image, then kept for the request.
    pub fn loaders(&self) -> Result<Vec<ImageLoader<'_>>, ImageError> {
        let mut loaders = Vec::new();
        loaders
            .try_reserve_exact(self.images.len())
            .map_err(InputError::Allocation)?;
        for image in &self.images {
            let root = &self.roots[image.root];
            let header_offset = image.header_offset;
            let input = InputId(
                u64::try_from(image.entry.map_or(0, |entry| entry.0))
                    .map_err(|_| InputError::PlatformCapacity)?,
            );
            loaders.push(Box::new(move || {
                Some(diana_decomp::DexImage {
                    bytes: root.bytes().ok()?,
                    header_offset,
                    origin: ImageOrigin { input },
                })
            }) as ImageLoader<'_>);
        }
        Ok(loaders)
    }

    /// One pass over the requested roots: logical headers always, declarations
    /// when a selector is supplied. Engine IDs are assigned after the rows are
    /// re-ordered by physical ordinal, so identity never depends on which worker
    /// finished first.
    fn scan(
        scope: &'a RequestScope,
        entry: Option<u32>,
        jobs: usize,
        selector: Option<&ClassSelector>,
        indexed: bool,
    ) -> Result<(Self, Vec<ImageHit>, ClassIndex), ImageError> {
        // An indexed scan reads every root so the class index covers the whole
        // input; a locator-scoped scan reads only the roots it names.
        let roots = match entry.filter(|_| !indexed) {
            Some(entry) => scope.roots_in_entry(EntryIndex(
                usize::try_from(entry).map_err(|_| InputError::PlatformCapacity)?,
            ))?,
            None => scope.roots()?,
        };
        // A single root is the whole request: keep its payload instead of
        // inflating it once for the scan and again for the answer.
        let retain = roots.len() == 1;
        let entry = entry
            .map(usize::try_from)
            .transpose()
            .map_err(|_| ImageError::Input(InputError::PlatformCapacity))?;
        // Roots the locator names must answer; an unnamed root that cannot be
        // read only loses its own declarations.
        let mut named = Vec::new();
        named
            .try_reserve_exact(roots.len())
            .map_err(InputError::Allocation)?;
        named.extend(roots.iter().map(|root| {
            entry.is_none_or(|entry| root.entry().map_or(0, |record| record.index().0) == entry)
        }));
        let mut scanned = Vec::new();
        scanned
            .try_reserve_exact(roots.len())
            .map_err(InputError::Allocation)?;
        // Claim the largest physical roots first: reading one 9 MB member must
        // start immediately instead of waiting behind forty small ones.
        let mut work = Vec::new();
        work.try_reserve_exact(roots.len())
            .map_err(InputError::Allocation)?;
        work.extend(0..roots.len());
        work.sort_unstable_by_key(|&ordinal| std::cmp::Reverse(roots[ordinal].declared_len()));
        runner::run::<_, _, _, ImageError>(
            roots.len(),
            jobs.max(1),
            Vec::new,
            |scratch, task, emit| {
                let ordinal = work[task];
                // A damaged root is reported through its own row so the failure
                // that surfaces is still the lowest physical ordinal's, not
                // whichever worker happened to finish first.
                let row = {
                    let root = &roots[ordinal];
                    let selector = selector.filter(|_| named[ordinal]);
                    if retain || !indexed && selector.is_none() {
                        scan_whole(root, scratch, selector, retain, indexed)
                    } else {
                        scan_declarations(root, scratch, selector, indexed)
                    }
                };
                emit((ordinal, row))?;
                Ok(())
            },
            &mut |row| {
                scanned.push(row);
                Ok(())
            },
        )?;
        if scanned.len() != roots.len() {
            return Err(ImageError::WorkerLost);
        }
        scanned.sort_unstable_by_key(|(ordinal, _)| *ordinal);
        let mut images = Vec::new();
        let mut hits = Vec::new();
        let mut locations = ClassIndex::default();
        let mut index_issues = Vec::new();
        for (ordinal, row) in scanned {
            let row = match row {
                Ok(row) => row,
                Err(error) if indexed && !named[ordinal] => {
                    index_issues.push(format!(
                        "{} declarations are unavailable: {error}",
                        roots[ordinal].label()
                    ));
                    ScannedRoot {
                        headers: vec![ByteOffset(0)],
                        hits: vec![Ok(None)],
                        classes: vec![Vec::new()],
                        entry: roots[ordinal].entry().map(|record| record.index()),
                    }
                }
                Err(error) => return Err(error),
            };
            let mut pending = row.hits.into_iter();
            let mut declared = row.classes.into_iter();
            for (logical, header) in row.headers.into_iter().enumerate() {
                let position = images.len();
                let dex = DexId(u32::try_from(position).map_err(|_| InputError::PlatformCapacity)?);
                let logical =
                    LogicalDex(u32::try_from(logical).map_err(|_| InputError::PlatformCapacity)?);
                if let Some(classes) = declared.next() {
                    locations
                        .try_reserve(classes.len())
                        .map_err(InputError::Allocation)?;
                    for (class_def, hash) in classes {
                        if let Some(packed) = Engine::pack_class_location(position, class_def) {
                            locations.entry(hash).or_insert(packed);
                        }
                    }
                }
                images.try_reserve(1).map_err(InputError::Allocation)?;
                images.push(ImageRef {
                    dex,
                    entry: row.entry,
                    logical,
                    header_offset: header,
                    root: ordinal,
                });
                if let Some(hit) = pending.next() {
                    hits.try_reserve(1).map_err(InputError::Allocation)?;
                    hits.push(hit);
                }
            }
        }
        Ok((
            Self {
                roots,
                images,
                index_issues,
            },
            hits,
            locations,
        ))
    }

    /// Combines per-image candidates in physical order: the same
    /// single-definition and ambiguity rules hold, and the reported failure is
    /// still the first one the serial order would have met.
    fn select(
        &self,
        hits: Vec<Result<Option<Hit>, SelectorError>>,
    ) -> Result<Selection, SelectorError> {
        let mut found = None;
        for (position, hit) in hits.into_iter().enumerate() {
            let Some((class_def, descriptor)) = hit? else {
                continue;
            };
            if found.replace((position, class_def, descriptor)).is_some() {
                return Err(SelectorError::Ambiguous);
            }
        }
        let (position, class_def, descriptor) = found.ok_or(SelectorError::NotFound)?;
        Ok(Selection {
            position,
            class_def,
            descriptor,
        })
    }

    /// Borrowed view of one resolved selection.
    pub fn selection<'s>(&'s self, selection: &Selection) -> ClassSelection<'s> {
        ClassSelection {
            image: &self.images[selection.position],
            class_def: selection.class_def,
            descriptor: selection.descriptor.clone(),
        }
    }

    pub fn images(&self) -> &[ImageRef] {
        &self.images
    }

    /// Roots the whole-input index could not read, in physical order.
    pub fn index_issues(&self) -> &[String] {
        &self.index_issues
    }

    /// Original root bytes; offsets remain container-relative for DEX 041.
    pub fn bytes<'s>(&'s self, image: &ImageRef) -> Result<&'s [u8], ImageError> {
        Ok(self.roots[image.root].bytes()?)
    }

    pub fn program<'s>(&'s self, image: &ImageRef) -> Result<ProgramIndex<'s>, ImageError> {
        Ok(DexContainer::open(self.bytes(image)?)?.program(image.header_offset)?)
    }
}

/// Whole-input class index: descriptor hash to a packed image and class_def.
pub(super) type ClassIndex = std::collections::HashMap<u64, u32>;

/// One root's logical headers and its per-logical-image candidates.
struct ScannedRoot {
    headers: Vec<ByteOffset>,
    hits: Vec<ImageHit>,
    /// Per logical image, every declared class and its descriptor hash.
    classes: Vec<Vec<(ClassDefId, u64)>>,
    entry: Option<EntryIndex>,
}

/// One logical image's resolution outcome in this scan.
type ImageHit = Result<Option<Hit>, SelectorError>;

/// One candidate definition and the descriptor it was matched by.
type Hit = (ClassDefId, Vec<u16>);

/// A resolved class as an image position plus index facts, so it does not borrow
/// the set it was resolved from.
#[derive(Debug)]
pub(super) struct Selection {
    position: usize,
    class_def: ClassDefId,
    descriptor: Vec<u16>,
}

/// An original class whose locator agrees with its checked table entry.
#[derive(Debug)]
pub(super) struct ClassSelection<'a> {
    pub image: &'a ImageRef,
    pub class_def: ClassDefId,
    pub descriptor: Vec<u16>,
}

/// Candidate definitions in one image. A second definition inside the same image
/// is already ambiguous, so only a single index is returned.
fn scan_image(
    index: &ProgramIndex<'_>,
    selector: &ClassSelector,
) -> Result<Option<Hit>, SelectorError> {
    if let Some(id) = selector.class_def {
        if id >= index.class_count() {
            return Err(SelectorError::InvalidIndex);
        }
        let class = index.class(ClassDefId(id))?;
        if !selector.descriptor.is_empty() && !class.descriptor.matches(&selector.descriptor)? {
            return Err(SelectorError::InvalidIndex);
        }
        return Ok(Some((ClassDefId(id), class.descriptor.units()?)));
    }
    let mut hit = None;
    for id in 0..index.class_count() {
        let class = index.class(ClassDefId(id))?;
        if !class.descriptor.matches(&selector.descriptor)? {
            continue;
        }
        if hit.is_some() {
            return Err(SelectorError::Ambiguous);
        }
        hit = Some(ClassDefId(id));
    }
    let Some(class_def) = hit else {
        return Ok(None);
    };
    Ok(Some((
        class_def,
        index.class(class_def)?.descriptor.units()?,
    )))
}

/// Reads one root's declarations for selection and, when requested, the index.
///
/// A class list needs the header, three id tables and the descriptor strings, so a
/// declaration prefix answers it without inflating the code and map tail. Chained
/// DEX 041 containers and late descriptor pools fall back to the whole member.
fn scan_declarations(
    root: &DexRoot<'_>,
    scratch: &mut Vec<u8>,
    selector: Option<&ClassSelector>,
    indexed: bool,
) -> Result<ScannedRoot, ImageError> {
    match scan_prefix(root, scratch, selector, indexed)? {
        Some(row) => Ok(row),
        None => scan_whole(root, scratch, selector, false, indexed),
    }
}

/// Reads one root's declarations from its whole payload. `retain` keeps the
/// payload for the lazy loader instead of dropping it with the scan.
fn scan_whole(
    root: &DexRoot<'_>,
    scratch: &mut Vec<u8>,
    selector: Option<&ClassSelector>,
    retain: bool,
    indexed: bool,
) -> Result<ScannedRoot, ImageError> {
    let bytes = if retain {
        root.bytes()?
    } else {
        root.bytes_into(scratch)?
    };
    let container = DexContainer::open(bytes)?;
    let headers = container.logical_headers()?;
    let mut classes = Vec::new();
    let mut hits = Vec::new();
    if indexed {
        classes
            .try_reserve_exact(headers.len())
            .map_err(InputError::Allocation)?;
    }
    if indexed || selector.is_some() {
        hits.try_reserve_exact(headers.len())
            .map_err(InputError::Allocation)?;
    }
    for (logical, header) in headers.iter().enumerate() {
        let selected = selector.filter(|selector| {
            selector
                .logical
                .is_none_or(|wanted| u32::try_from(logical) == Ok(wanted))
        });
        if !indexed && selected.is_none() {
            if selector.is_some() {
                hits.push(Ok(None));
            }
            continue;
        }
        let program = container.program(*header)?;
        if indexed {
            let mut declared = Vec::new();
            declared
                .try_reserve_exact(
                    usize::try_from(program.class_count())
                        .map_err(|_| InputError::PlatformCapacity)?,
                )
                .map_err(InputError::Allocation)?;
            for ordinal in 0..program.class_count() {
                let class_def = ClassDefId(ordinal);
                let view = program.class(class_def)?;
                declared.push((
                    class_def,
                    Engine::class_descriptor_hash(&view.descriptor.utf8_lossy()?),
                ));
            }
            classes.push(declared);
        }
        hits.push(match selected {
            Some(selector) => scan_image(&program, selector),
            None => Ok(None),
        });
    }
    Ok(ScannedRoot {
        headers,
        hits,
        classes,
        entry: root.entry().map(|record| record.index()),
    })
}

/// One root's declarations from a growing prefix, or `None` when a prefix cannot
/// answer and the caller must read the whole member.
fn scan_prefix(
    root: &DexRoot<'_>,
    scratch: &mut Vec<u8>,
    selector: Option<&ClassSelector>,
    indexed: bool,
) -> Result<Option<ScannedRoot>, ImageError> {
    let mut reader = root.prefix_reader(scratch)?;
    let mut aim = 0x70usize;
    for _ in 0..6 {
        let bytes = reader.read(aim)?;
        match prefix::classes(bytes)? {
            PrefixClasses::Needs(more) => {
                // Prefix growth resumes the stream. Switch to the whole reader
                // before touching most of the payload only to fall back and
                // inflate it again with the whole-entry backend.
                if more <= bytes.len()
                    || more.saturating_mul(5) > root.declared_len().saturating_mul(3)
                {
                    return Ok(None);
                }
                aim = more;
            }
            PrefixClasses::Whole => return Ok(None),
            PrefixClasses::Ready(classes) => {
                let mut declared = Vec::new();
                if indexed {
                    declared
                        .try_reserve_exact(classes.len())
                        .map_err(InputError::Allocation)?;
                    for class in &classes {
                        declared.push((
                            class.class_def,
                            Engine::class_descriptor_hash(&class.descriptor.utf8_lossy()?),
                        ));
                    }
                }
                let hit = match selector {
                    Some(selector) if selector.logical.is_none_or(|wanted| wanted == 0) => {
                        scan_declared(&classes, selector)
                    }
                    _ => Ok(None),
                };
                return Ok(Some(ScannedRoot {
                    headers: vec![ByteOffset(0)],
                    hits: vec![hit],
                    classes: if indexed { vec![declared] } else { Vec::new() },
                    entry: root.entry().map(|record| record.index()),
                }));
            }
        }
    }
    Ok(None)
}

/// Candidate definitions among one image's declared classes.
fn scan_declared(
    classes: &[diana_decomp::input::DeclaredClass<'_>],
    selector: &ClassSelector,
) -> Result<Option<Hit>, SelectorError> {
    if let Some(id) = selector.class_def {
        let Some(class) = classes.iter().find(|class| class.class_def.0 == id) else {
            return Err(SelectorError::InvalidIndex);
        };
        if !selector.descriptor.is_empty() && !class.descriptor.matches(&selector.descriptor)? {
            return Err(SelectorError::InvalidIndex);
        }
        return Ok(Some((class.class_def, class.descriptor.units()?)));
    }
    let mut hit = None;
    for class in classes {
        if !class.descriptor.matches(&selector.descriptor)? {
            continue;
        }
        if hit.is_some() {
            return Err(SelectorError::Ambiguous);
        }
        hit = Some(class);
    }
    match hit {
        Some(class) => Ok(Some((class.class_def, class.descriptor.units()?))),
        None => Ok(None),
    }
}
