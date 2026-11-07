use diana_decomp::input::{PrefixClasses, prefix};
use diana_decomp::{FailureKind, RequestError};

use super::apk::DexRoot;
use super::archive::EntryRecord;
use super::dex::{
    self, ClassFilter, ClassKind, ClassRow, FieldRow, ImageIndex, MethodRow, StringRow, contains,
};
use super::images::LogicalDex;
use super::input::{EntryIndex, InputError};
use super::request::RequestScope;

/// One physical ZIP record, listed in directory order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryRow {
    pub entry: EntryIndex,
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub method: u16,
}

/// One result row. Every variant keeps the original DEX/entry identity that a
/// caller can pass back, independent of display order.
///
/// `Class` and `String` borrow the container bytes: a class index or a string pool
/// is millions of rows, and owning their text cost one allocation per row. A row
/// never outlives the scan of the root it came from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Row<'a> {
    Entry(EntryRow),
    Class(ClassRow<'a>),
    Method(MethodRow<'a>),
    Field(FieldRow<'a>),
    String(StringRow<'a>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryKind {
    Entries,
    Classes,
    Methods,
    Fields,
    Strings,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error(transparent)]
    Input(#[from] InputError),
    /// A checked DEX read failure; its own kind survives unchanged.
    #[error(transparent)]
    Engine(#[from] diana_decomp::input::InputError),
    /// A checked method-read failure; its own kind survives unchanged.
    #[error(transparent)]
    Method(#[from] diana_decomp::input::MethodError),
    #[error(transparent)]
    Request(#[from] RequestError),
    #[error("query worker stopped before delivering its rows")]
    WorkerLost,
    #[error("entries lists physical archive records only; a bare DEX has none")]
    NotArchive,
    #[error("the job count must be positive")]
    InvalidJobs,
}

impl QueryError {
    pub fn kind(&self) -> FailureKind {
        match self {
            Self::Input(error) => error.kind(),
            Self::Engine(error) => error.kind(),
            Self::Method(error) => error.kind(),
            Self::Request(error) => error.kind(),
            Self::WorkerLost => FailureKind::InternalInvariant,
            Self::NotArchive | Self::InvalidJobs => FailureKind::Unsupported,
        }
    }
}

/// One root image's display metadata. It is not an original identity by itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageInfo {
    pub index: ImageIndex,
    pub label: String,
    pub entry: Option<EntryIndex>,
}

/// How much formatted text one worker buffers before handing a chunk over.
const CHUNK_BYTES: usize = 1 << 20;

/// A query request over one leased input.
#[derive(Clone, Debug)]
pub struct QueryPlan {
    pub kind: QueryKind,
    /// Literal UTF-16 substring over the primary column text.
    pub filter: Option<Vec<u16>>,
    /// Exact class descriptor restricting methods/fields to one definition.
    pub class: Option<Vec<u16>>,
    /// Direct hierarchy relations restricting `classes`.
    pub class_filter: super::dex::ClassFilter,
    pub jobs: usize,
}

/// Prepared query: root images are resolved once, rows stream in completion order.
pub struct Query<'a> {
    roots: Vec<DexRoot<'a>>,
    images: Vec<ImageInfo>,
    kind: QueryKind,
    filter: Option<Vec<u16>>,
    class: Option<Vec<u16>>,
    class_filter: super::dex::ClassFilter,
    jobs: usize,
    entries: Option<&'a [EntryRecord]>,
}

impl<'a> Query<'a> {
    /// Resolves the root images and their display labels. No DEX analysis runs
    /// here, and no row is produced yet.
    pub fn new(scope: &'a RequestScope, plan: QueryPlan) -> Result<Self, QueryError> {
        if plan.jobs == 0 {
            return Err(QueryError::InvalidJobs);
        }
        let roots = if plan.kind == QueryKind::Entries {
            Vec::new()
        } else {
            scope.roots()?
        };
        let mut images = Vec::new();
        for (ordinal, root) in roots.iter().enumerate() {
            images.try_reserve(1).map_err(|_| QueryError::WorkerLost)?;
            images.push(ImageInfo {
                index: ImageIndex(ordinal),
                label: root.label().to_owned(),
                entry: root.entry().map(EntryRecord::index),
            });
        }
        let entries = scope.input().archive().map(|directory| directory.entries());
        Ok(Self {
            roots,
            images,
            kind: plan.kind,
            filter: plan.filter,
            class: plan.class,
            class_filter: plan.class_filter,
            jobs: plan.jobs,
            entries,
        })
    }

    pub fn images(&self) -> &[ImageInfo] {
        &self.images
    }

    /// Streams every matching row, formatted inside the scan.
    ///
    /// `render` runs on the workers: a root's rows are formatted into one chunk and
    /// only the chunk crosses to `sink`, which runs on the caller's thread. Splitting
    /// the two is what keeps formatting off the single consumer thread; `first` says
    /// whether the row is the first of its chunk, which is where the JSON separator
    /// goes.
    pub fn run<R, S>(&self, render: &R, sink: &mut S) -> Result<(), QueryError>
    where
        R: for<'r> Fn(Row<'r>, &mut Vec<u8>, bool) -> Result<(), QueryError> + Sync,
        S: FnMut(Vec<u8>) -> Result<(), QueryError>,
    {
        if self.kind == QueryKind::Entries {
            let entries = self.entries.ok_or(QueryError::NotArchive)?;
            let mut chunk = Vec::new();
            let mut first = true;
            for entry in entries {
                let row = EntryRow {
                    entry: entry.index(),
                    name: entry.name().to_owned(),
                    compressed_size: entry.compressed_size().0,
                    uncompressed_size: entry.uncompressed_size().0,
                    method: entry.method(),
                };
                if let Some(needle) = &self.filter
                    && !contains(&row.name.encode_utf16().collect::<Vec<_>>(), needle)
                {
                    continue;
                }
                render(Row::Entry(row), &mut chunk, first)?;
                first = false;
            }
            if !chunk.is_empty() {
                sink(chunk)?;
            }
            return Ok(());
        }
        let kind = self.kind;
        let mut work = Vec::new();
        work.try_reserve_exact(self.roots.len())
            .map_err(InputError::Allocation)?;
        work.extend(0..self.roots.len());
        // Start expensive roots first without moving them: ImageIndex remains the
        // original directory ordinal, independent of worker assignment/completion.
        work.sort_unstable_by_key(|&ordinal| std::cmp::Reverse(self.roots[ordinal].declared_len()));
        super::runner::run(
            self.roots.len(),
            self.jobs,
            Vec::new,
            |scratch: &mut Vec<u8>, task, emit| {
                let ordinal = work[task];
                let root = &self.roots[ordinal];
                let image = ImageIndex(ordinal);
                let filter = self.filter.as_deref();
                let class = self.class.as_deref();
                let mut chunk = Vec::new();
                {
                    let mut first = true;
                    let mut push = |row: Row<'_>| -> Result<(), QueryError> {
                        render(row, &mut chunk, first)?;
                        first = false;
                        // Bound how much formatted text a worker holds: a large DEX
                        // can produce tens of MiB for one root, and the consumer
                        // writes a chunk as soon as it arrives. Splitting mid-root is
                        // safe because a chunk boundary always carries the JSON
                        // separator, and rows inside a chunk carry their own.
                        if chunk.len() >= CHUNK_BYTES {
                            emit(std::mem::replace(
                                &mut chunk,
                                Vec::with_capacity(CHUNK_BYTES),
                            ))?;
                            // The next row starts a new chunk: it carries no
                            // separator of its own, the sink adds one.
                            first = true;
                        }
                        Ok(())
                    };
                    // Declaration-only queries can finish before the code/map tail.
                    // No rows are emitted until the prefix contains the complete list.
                    let from_prefix = match kind {
                        QueryKind::Classes => classes_from_prefix(
                            root,
                            scratch,
                            image,
                            filter,
                            &self.class_filter,
                            &mut push,
                        )?,
                        QueryKind::Strings => {
                            strings_from_prefix(root, scratch, image, filter, &mut push)?
                        }
                        _ => false,
                    };
                    if !from_prefix {
                        dex::visit_logical(
                            root.bytes_into(scratch)?,
                            |logical, index| match kind {
                                QueryKind::Entries => Ok(()),
                                QueryKind::Classes => dex::classes(
                                    &index,
                                    image,
                                    logical,
                                    filter,
                                    &self.class_filter,
                                    &mut push,
                                ),
                                QueryKind::Methods => {
                                    dex::methods(&index, image, logical, class, filter, &mut push)
                                }
                                QueryKind::Fields => {
                                    dex::fields(&index, image, logical, class, filter, &mut push)
                                }
                                QueryKind::Strings => {
                                    dex::strings(&index, image, logical, filter, &mut push)
                                }
                            },
                        )?;
                    }
                }
                if !chunk.is_empty() {
                    emit(chunk)?;
                }
                Ok(())
            },
            sink,
        )
    }
}

/// Publishes a complete string pool from the shared declaration-prefix reader.
fn strings_from_prefix(
    root: &DexRoot<'_>,
    scratch: &mut Vec<u8>,
    image: ImageIndex,
    filter: Option<&[u16]>,
    emit: &mut dyn FnMut(Row<'_>) -> Result<(), QueryError>,
) -> Result<bool, QueryError> {
    let mut reader = root.prefix_reader(scratch)?;
    let mut aim = 0x70;
    for _ in 0..6 {
        let bytes = reader.read(aim)?;
        match prefix::strings(bytes)? {
            prefix::PrefixStrings::Needs(more) => {
                if more <= bytes.len()
                    || more.saturating_mul(5) > root.declared_len().saturating_mul(3)
                {
                    return Ok(false);
                }
                aim = more;
            }
            prefix::PrefixStrings::Whole => return Ok(false),
            prefix::PrefixStrings::Ready(values) => {
                for (ordinal, value) in values.into_iter().enumerate() {
                    if let Some(needle) = filter
                        && !super::dex::contains_ignore_case(&value.units()?, needle)
                    {
                        continue;
                    }
                    let string = diana_decomp::StringId(
                        u32::try_from(ordinal).map_err(|_| InputError::PlatformCapacity)?,
                    );
                    emit(Row::String(StringRow {
                        image,
                        logical: LogicalDex(0),
                        string,
                        value,
                    }))?;
                }
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Lists class definitions from a member prefix, when the filter needs nothing the
/// prefix cannot answer.
///
/// Returns `false` when the prefix path is not applicable or could not answer: the
/// caller then reads the whole member, so a short prefix can never shorten a result.
fn classes_from_prefix(
    root: &DexRoot<'_>,
    scratch: &mut Vec<u8>,
    image: ImageIndex,
    filter: Option<&[u16]>,
    relation: &ClassFilter,
    emit: &mut dyn FnMut(Row<'_>) -> Result<(), QueryError>,
) -> Result<bool, QueryError> {
    // Hierarchy filters need the superclass and interface lists, which the prefix
    // view does not read; `kind` only needs the access flags, which it does.
    if relation.extends.is_some()
        || relation.implements.is_some()
        || relation.subinterface.is_some()
    {
        return Ok(false);
    }
    let mut reader = root.prefix_reader(scratch)?;
    let mut aim = 0x70usize;
    for _ in 0..6 {
        let bytes = reader.read(aim)?;
        match prefix::classes(bytes)? {
            PrefixClasses::Needs(more) => {
                // A prefix only pays off while it is clearly shorter than the
                // member: every round re-inflates from the start, so asking for most
                // of the member costs more than inflating it once.
                if more <= bytes.len()
                    || more.saturating_mul(5) > root.declared_len().saturating_mul(3)
                {
                    return Ok(false);
                }
                aim = more;
            }
            PrefixClasses::Whole => return Ok(false),
            PrefixClasses::Ready(classes) => {
                for class in classes {
                    if let Some(needle) = filter
                        && !contains(&class.descriptor.units()?, needle)
                    {
                        continue;
                    }
                    if let Some(kind) = relation.kind {
                        let interface = class.access_flags & 0x200 != 0;
                        if interface != (kind == ClassKind::Interface) {
                            continue;
                        }
                    }
                    emit(Row::Class(ClassRow {
                        image,
                        logical: LogicalDex(0),
                        class_def: class.class_def,
                        descriptor: class.descriptor,
                        access_flags: class.access_flags,
                    }))?;
                }
                return Ok(true);
            }
        }
    }
    Ok(false)
}
