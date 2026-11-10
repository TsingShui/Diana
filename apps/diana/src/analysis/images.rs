//! Logical DEX identities over borrowed physical input roots.
//!
//! Engine IDs follow the logical image vector, not physical root ordinals.
//! Every image retains its original header offset and physical entry.

use diana_decomp::input::{DexContainer, ProgramIndex};
use diana_decomp::{ByteOffset, ClassDefId, DexId};

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
}

impl<'a> ImageSet<'a> {
    /// Materializes the requested physical roots and their logical headers.
    pub fn open(
        scope: &'a RequestScope,
        entry: Option<u32>,
        jobs: usize,
    ) -> Result<Self, ImageError> {
        Ok(Self::scan(scope, entry, jobs, None)?.0)
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
        let (set, scanned) = Self::scan(scope, selector.entry, jobs, Some(selector))?;
        let selection = set.select(scanned)?;
        Ok((set, selection))
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
    ) -> Result<(Self, Vec<ImageHit>), ImageError> {
        let roots = match entry {
            Some(entry) => scope.roots_in_entry(EntryIndex(
                usize::try_from(entry).map_err(|_| InputError::PlatformCapacity)?,
            ))?,
            None => scope.roots()?,
        };
        // A single root is the whole request: keep its payload instead of
        // inflating it once for the scan and again for the answer.
        let retain = roots.len() == 1;
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
        let entry = entry
            .map(usize::try_from)
            .transpose()
            .map_err(|_| ImageError::Input(InputError::PlatformCapacity))?;
        runner::run::<_, _, _, ImageError>(
            roots.len(),
            jobs.max(1),
            Vec::new,
            |scratch, task, emit| {
                let ordinal = work[task];
                // A damaged root is reported through its own row so the failure
                // that surfaces is still the lowest physical ordinal's, not
                // whichever worker happened to finish first.
                let row = (|| -> Result<ScannedRoot, ImageError> {
                    let root = &roots[ordinal];
                    let bytes = if retain {
                        root.bytes()?
                    } else {
                        root.bytes_into(scratch)?
                    };
                    let container = DexContainer::open(bytes)?;
                    let headers = container.logical_headers()?;
                    let mut hits = Vec::new();
                    if let Some(selector) = selector {
                        hits.try_reserve_exact(headers.len())
                            .map_err(InputError::Allocation)?;
                        for (logical, header) in headers.iter().enumerate() {
                            let selected = entry.is_none_or(|entry| {
                                root.entry().map_or(0, |record| record.index().0) == entry
                            }) && selector
                                .logical
                                .is_none_or(|wanted| u32::try_from(logical) == Ok(wanted));
                            let hit = if selected {
                                scan_image(&container.program(*header)?, selector)
                            } else {
                                Ok(None)
                            };
                            hits.push(hit);
                        }
                    }
                    Ok(ScannedRoot {
                        headers,
                        hits,
                        entry: root.entry().map(|record| record.index()),
                    })
                })();
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
        for (ordinal, row) in scanned {
            let row = row?;
            let mut pending = row.hits.into_iter();
            for (logical, header) in row.headers.into_iter().enumerate() {
                let dex =
                    DexId(u32::try_from(images.len()).map_err(|_| InputError::PlatformCapacity)?);
                let logical =
                    LogicalDex(u32::try_from(logical).map_err(|_| InputError::PlatformCapacity)?);
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
        Ok((Self { roots, images }, hits))
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

    /// Original root bytes; offsets remain container-relative for DEX 041.
    pub fn bytes<'s>(&'s self, image: &ImageRef) -> Result<&'s [u8], ImageError> {
        Ok(self.roots[image.root].bytes()?)
    }

    pub fn program<'s>(&'s self, image: &ImageRef) -> Result<ProgramIndex<'s>, ImageError> {
        Ok(DexContainer::open(self.bytes(image)?)?.program(image.header_offset)?)
    }
}

/// One root's logical headers and its per-logical-image candidates.
struct ScannedRoot {
    headers: Vec<ByteOffset>,
    hits: Vec<ImageHit>,
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
