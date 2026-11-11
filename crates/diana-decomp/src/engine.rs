//! Engine facade over P0 class facts and P1 decoded methods.

use std::sync::Arc;

use crate::analysis::java_backend::{JavaDecompiler, JavaDecompilerConfig};
use crate::frontend::{ClassNode, DexError, DexResult, MethodNode};
use crate::input::{InputError, InputProblem};
use crate::ir::analysis::ClassHierarchyIndex;
use crate::ir::{AnalysisEvent, AnalysisEventKind, AnalysisObserver, CFG};
use crate::p0::{ClassFacts, Fact};
use crate::p1::DecodedMethod;
use crate::render::Rendered;
use crate::{
    ByteOffset, ClassDefId, ClassKey, DexId, ImageFact, ImageOrigin, MethodId, RequestError,
};

mod outline;
mod request;

use request::{BackendInput, IndexedImage, MethodFacts};

#[derive(Clone, Copy)]
pub struct DexImage<'a> {
    pub bytes: &'a [u8],
    pub header_offset: ByteOffset,
    pub origin: ImageOrigin,
}

pub struct MethodSelector {
    pub name: Vec<u16>,
    pub prototype: Vec<u16>,
}

pub struct Request {
    pub class: ClassKey,
    pub method: Option<MethodSelector>,
}

/// Produces one image's bytes on first use.
pub type ImageLoader<'a> = Box<dyn FnOnce() -> Option<DexImage<'a>> + 'a>;

pub struct Engine<'a> {
    images: Vec<std::cell::OnceCell<Option<IndexedImage<'a>>>>,
    /// Loaders for images that are inflated only when first referenced.
    loaders: Vec<std::cell::RefCell<Option<ImageLoader<'a>>>>,
    jobs: Option<usize>,
    /// Descriptor hash to packed (image position, class_def) for cross-image
    /// outline resolution. Built once on first use.
    locations: std::sync::OnceLock<std::collections::HashMap<u64, u32>>,
    /// True when the caller supplied the whole-APK index, which needs no
    /// per-hit descriptor verification.
    trusted_locations: bool,
}

pub struct ClassSource {
    pub class: ClassKey,
    pub text: String,
    pub methods: Vec<MethodSource>,
    pub failures: Vec<MethodFailure>,
}

pub struct MethodSource {
    pub method: MethodId,
    pub name: Vec<u16>,
    pub prototype: Vec<u16>,
    pub failure: Option<String>,
}

pub struct ClassAnalysis {
    pub image: ImageFact,
    pub facts: Arc<ClassFacts>,
    pub rendered: Rendered,
    /// Method-local failures retained beside the published class text.
    pub methods: Vec<MethodSource>,
    pub failures: Vec<MethodFailure>,
}

pub struct SingleMethodAnalysis {
    pub rendered: Rendered,
    pub failures: Vec<MethodFailure>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MethodFailure {
    pub class: String,
    pub name: String,
    pub prototype: String,
    pub stage: String,
    pub reason: String,
}

struct ClassOutput {
    source: String,
    methods: Vec<MethodSource>,
    failures: Vec<MethodFailure>,
}

#[derive(Default)]
struct MethodFailureObserver {
    failures: std::sync::Mutex<Vec<MethodFailure>>,
}

impl MethodFailureObserver {
    fn snapshot(&self) -> Vec<MethodFailure> {
        match self.failures.lock() {
            Ok(failures) => failures.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

impl AnalysisObserver for MethodFailureObserver {
    fn is_enabled(&self, kind: AnalysisEventKind) -> bool {
        kind == AnalysisEventKind::MethodFailure
    }

    fn observe(&self, event: AnalysisEvent<'_>) {
        let AnalysisEvent::MethodFailure {
            class,
            method,
            descriptor,
            stage,
            reason,
        } = event
        else {
            return;
        };
        let failure = MethodFailure {
            class: class.to_owned(),
            name: method.to_owned(),
            prototype: descriptor.to_owned(),
            stage: stage.to_owned(),
            reason: reason.to_owned(),
        };
        match self.failures.lock() {
            Ok(mut failures) => failures.push(failure),
            Err(poisoned) => poisoned.into_inner().push(failure),
        }
    }
}

/// Decode one P1 method directly into the recovery CFG.
fn decode_decoded_method(method: &MethodNode, facts: &DecodedMethod<'_>) -> DexResult<CFG> {
    let ir = facts.ir().ok_or_else(|| {
        DexError::Backend(
            facts
                .ir_error()
                .unwrap_or("P1 produced no recovery IR")
                .to_owned(),
        )
    })?;
    let owner = method
        .info
        .declaring_class
        .parse::<crate::ir::ArgType>()
        .map_err(|source| DexError::InvalidDescriptor {
            descriptor: method.info.declaring_class.clone(),
            source,
        })?;
    let context = crate::ir::MethodContext::new(
        owner,
        method.info.name.clone(),
        crate::ir::MethodDescriptor {
            parameters: method.info.param_types.clone(),
            return_type: method.info.return_type.clone(),
        },
        method.access_flags.is_static(),
    )
    .with_declared_synchronization(method.access_flags.is_synchronized());
    Ok(crate::ir::splitter::Splitter::new("method")
        .method(context)
        .instructions(ir.instructions().to_vec())
        .handlers(ir.handlers().to_vec())
        .registers(ir.registers())
        .ins(ir.ins())
        .build())
}

/// The class hierarchy facts the backend needs, built from the supplied classes.
fn hierarchy(classes: &[&ClassNode]) -> Arc<ClassHierarchyIndex> {
    Arc::new(crate::analysis::method_override::type_hierarchy_from_classes(classes))
}

/// The target-language ABI facts the backend needs, built from the same classes.
fn java_abi(classes: &[&ClassNode]) -> Arc<crate::analysis::java_backend::JavaSourceAbi> {
    Arc::new(crate::analysis::java_backend::JavaSourceAbi::analyze(
        classes.iter().copied(),
        |_| (Vec::new(), None),
    ))
}

struct ClassParts {
    class: ClassNode,
    methods: Vec<crate::analysis::ClassMethodInput>,
    nested: Vec<crate::analysis::NestedClassInput>,
}

fn failed_method_input(
    method: MethodNode,
    error: impl std::fmt::Display,
) -> crate::analysis::ClassMethodInput {
    crate::analysis::ClassMethodInput::failed(
        method,
        crate::analysis::MethodRecoveryFailure::new(
            crate::analysis::MethodRecoveryStage::Decode,
            error,
        ),
    )
}

fn build_method_inputs<'a>(
    class: &ClassNode,
    methods: Vec<MethodFacts<'a>>,
    jobs: Option<usize>,
) -> Result<Vec<crate::analysis::ClassMethodInput>, DexError> {
    use rayon::prelude::*;

    let prepare = |facts: MethodFacts<'a>| {
        let method = class
            .methods()
            .iter()
            .find(|method| method.id == facts.method.0)
            .cloned()?;
        Some(match facts.decoded {
            Ok(decoded) => match decode_decoded_method(&method, &decoded) {
                Ok(cfg) => crate::analysis::ClassMethodInput::decoded(method, cfg),
                Err(error) => failed_method_input(method, error),
            },
            Err(error) => failed_method_input(method, error),
        })
    };
    let prepared: Vec<Option<crate::analysis::ClassMethodInput>> = if jobs == Some(1) {
        methods.into_iter().map(prepare).collect()
    } else {
        match jobs.and_then(|jobs| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(jobs)
                .build()
                .ok()
        }) {
            Some(pool) => pool.install(|| methods.into_par_iter().map(prepare).collect()),
            None => methods.into_par_iter().map(prepare).collect(),
        }
    };
    Ok(prepared.into_iter().flatten().collect())
}

fn method_sources(methods: &[crate::analysis::ClassMethodInput]) -> Vec<MethodSource> {
    methods
        .iter()
        .map(|input| {
            let method = input.method();
            MethodSource {
                method: MethodId(method.id),
                name: method.info.name.encode_utf16().collect(),
                prototype: method.info.descriptor().encode_utf16().collect(),
                failure: input.failure().map(|failure| failure.cause().to_owned()),
            }
        })
        .collect()
}

fn build_class_parts<'a>(
    input: BackendInput<'a>,
    jobs: Option<usize>,
) -> Result<ClassParts, DexError> {
    let class = input
        .class
        .node()
        .ok_or_else(|| DexError::Backend("class node assembly failed".to_owned()))?
        .clone();
    let methods = build_method_inputs(&class, input.methods, jobs)?;
    let mut nested = Vec::new();
    nested
        .try_reserve_exact(input.nested.len())
        .map_err(|_| DexError::Backend("nested class allocation failed".to_owned()))?;
    for child in input.nested {
        let parts = build_class_parts(child, jobs)?;
        nested.push(crate::analysis::NestedClassInput {
            class: parts.class,
            methods: parts.methods,
            nested: parts.nested,
        });
    }
    Ok(ClassParts {
        class,
        methods,
        nested,
    })
}

fn collect_nested_declarations<'a>(
    input: &'a crate::analysis::NestedClassInput,
    declarations: &mut Vec<&'a ClassNode>,
) {
    declarations.push(&input.class);
    for nested in &input.nested {
        collect_nested_declarations(nested, declarations);
    }
}

fn decompile_class<'a>(
    input: BackendInput<'a>,
    jobs: Option<usize>,
) -> Result<ClassOutput, DexError> {
    let related = input.related.clone();
    let parts = build_class_parts(input, jobs)?;
    let mut related_nodes = Vec::new();
    related_nodes
        .try_reserve_exact(related.len())
        .map_err(|_| DexError::Backend("related class allocation failed".to_owned()))?;
    for facts in &related {
        if let Some(node) = facts.node() {
            related_nodes.push(node.clone());
        }
    }
    let mut declarations: Vec<&ClassNode> = Vec::new();
    declarations
        .try_reserve_exact(related_nodes.len() + 1)
        .map_err(|_| DexError::Backend("declaration allocation failed".to_owned()))?;
    declarations.push(&parts.class);
    declarations.extend(related_nodes.iter());
    for nested in &parts.nested {
        collect_nested_declarations(nested, &mut declarations);
    }
    let method_sources = method_sources(&parts.methods);
    let observer = Arc::new(MethodFailureObserver::default());
    let analysis_observer: Arc<dyn AnalysisObserver> = observer.clone();
    let mut methods = parts.methods;
    let source =
        JavaDecompiler::new(JavaDecompilerConfig::default().with_preserved_dex_names(true))
            .with_shared_type_hierarchy(hierarchy(&declarations))
            .with_source_abi(java_abi(&declarations))
            .with_analysis_observer(analysis_observer)
            .generate_class_with_nested(&parts.class, &mut methods, parts.nested)
            .map_err(|error| DexError::Backend(error.to_string()))?;
    Ok(ClassOutput {
        source,
        methods: method_sources,
        failures: observer.snapshot(),
    })
}

impl<'a> Engine<'a> {
    pub fn with_method_jobs(mut self, jobs: usize) -> Self {
        self.jobs = Some(jobs.max(1));
        self
    }

    pub fn with_outline(self) -> Self {
        self
    }

    pub fn open(images: Vec<DexImage<'a>>) -> Result<Self, InputError> {
        if images.is_empty() {
            return Err(InputError::Malformed {
                code: InputProblem::InvalidReference,
                offset_byte: ByteOffset(0),
            });
        }
        let mut indexed = Vec::new();
        indexed
            .try_reserve_exact(images.len())
            .map_err(|_| InputError::Allocation {
                offset_byte: ByteOffset(0),
            })?;
        for image in images {
            let dex =
                DexId(
                    u32::try_from(indexed.len()).map_err(|_| InputError::PlatformCapacity {
                        offset_byte: image.header_offset,
                    })?,
                );
            indexed.push(IndexedImage::open(image, dex)?);
        }
        Ok(Self {
            images: indexed
                .into_iter()
                .map(|image| std::cell::OnceCell::from(Some(image)))
                .collect(),
            loaders: Vec::new(),
            jobs: None,
            locations: std::sync::OnceLock::new(),
            trusted_locations: false,
        })
    }

    pub fn focus(image: DexImage<'a>, dex: DexId) -> Result<Self, InputError> {
        let mut images = Vec::new();
        images
            .try_reserve_exact(1)
            .map_err(|_| InputError::Allocation {
                offset_byte: image.header_offset,
            })?;
        images.push(IndexedImage::open(image, dex)?);
        Ok(Self {
            images: images
                .into_iter()
                .map(|image| std::cell::OnceCell::from(Some(image)))
                .collect(),
            loaders: Vec::new(),
            jobs: None,
            locations: std::sync::OnceLock::new(),
            trusted_locations: false,
        })
    }

    /// Opens images on demand: each loader runs when its image is first used.
    pub fn open_lazy(loaders: Vec<ImageLoader<'a>>) -> Self {
        let count = loaders.len();
        Self {
            images: (0..count).map(|_| std::cell::OnceCell::new()).collect(),
            loaders: loaders
                .into_iter()
                .map(|loader| std::cell::RefCell::new(Some(loader)))
                .collect(),
            jobs: None,
            locations: std::sync::OnceLock::new(),
            trusted_locations: false,
        }
    }

    /// Supplies a whole-APK descriptor index built by the caller.
    ///
    /// Keys are [`Engine::class_descriptor_hash`] values and values are
    /// [`Engine::pack_class_location`] words. The index answers cross-image
    /// lookups without opening every image.
    pub fn with_class_locations(mut self, locations: std::collections::HashMap<u64, u32>) -> Self {
        if self.locations.set(locations).is_ok() {
            self.trusted_locations = true;
        }
        self
    }

    /// Descriptor hash used by the supplied class index.
    pub fn class_descriptor_hash(descriptor: &str) -> u64 {
        descriptor_hash(descriptor)
    }

    /// Packs an image position and class_def the way the index expects.
    pub fn pack_class_location(position: usize, class_def: ClassDefId) -> Option<u32> {
        let position = u32::try_from(position).ok()?;
        (position < (1 << 8) && class_def.0 < (1 << 24)).then_some((position << 24) | class_def.0)
    }

    pub(in crate::engine) fn image(&self, dex: DexId) -> Option<&IndexedImage<'a>> {
        // Positions and identities agree for images the engine opened itself;
        // `focus` keeps a caller-provided identity, so fall back to a search
        // over the images that are already open.
        if let Some(image) = self.image_at(dex.0 as usize) {
            if image.dex == dex {
                return Some(image);
            }
        }
        self.images
            .iter()
            .filter_map(|cell| cell.get().and_then(Option::as_ref))
            .find(|image| image.dex == dex)
    }

    /// Returns the image at one position, inflating it on first use.
    pub(in crate::engine) fn image_at(&self, position: usize) -> Option<&IndexedImage<'a>> {
        let cell = self.images.get(position)?;
        cell.get_or_init(|| {
            let loader = self.loaders.get(position)?.borrow_mut().take()?;
            let image = loader()?;
            IndexedImage::open(image, DexId(u32::try_from(position).ok()?)).ok()
        })
        .as_ref()
    }

    /// Locates a class definition across every opened image.
    ///
    /// The index keeps one descriptor hash per class instead of the descriptor
    /// strings, so a whole-APK lookup costs a few megabytes rather than a
    /// second copy of every class name.
    pub(in crate::engine) fn class_location(
        &self,
        descriptor: &str,
    ) -> Option<(usize, ClassDefId)> {
        let hash = descriptor_hash(descriptor);
        if self.trusted_locations {
            let packed = *self.locations.get()?.get(&hash)?;
            let position = (packed >> 24) as usize;
            let class_def = ClassDefId(packed & 0x00ff_ffff);
            // Verify against an image that is already open; never open one just
            // to re-check an index the caller built from the same descriptors.
            if self
                .images
                .get(position)
                .is_some_and(|cell| cell.get().is_some())
            {
                return self.verified_location(position, class_def, descriptor);
            }
            return Some((position, class_def));
        }
        let locations = self.locations.get_or_init(|| {
            let mut locations = std::collections::HashMap::new();
            for (position, cell) in self.images.iter().enumerate() {
                let Some(image) = cell.get().and_then(Option::as_ref) else {
                    continue;
                };
                let position = match u32::try_from(position) {
                    Ok(position) => position,
                    Err(_) => break,
                };
                for ordinal in 0..image.index.class_count() {
                    let key = ClassDefId(ordinal);
                    let Ok(view) = image.index.class(key) else {
                        continue;
                    };
                    let Ok(text) = view.descriptor.utf8_lossy() else {
                        continue;
                    };
                    if let Some(packed) = Self::pack_class_location(position as usize, key) {
                        locations.entry(descriptor_hash(&text)).or_insert(packed);
                    }
                }
            }
            locations
        });
        if let Some(packed) = locations.get(&hash).copied() {
            let position = (packed >> 24) as usize;
            let class_def = ClassDefId(packed & 0x00ff_ffff);
            if let Some(found) = self.verified_location(position, class_def, descriptor) {
                return Some(found);
            }
        }
        // A hash collision or a packed overflow falls back to the exact scan.
        for position in 0..self.images.len() {
            let Some(image) = self.image_at(position) else {
                continue;
            };
            for ordinal in 0..image.index.class_count() {
                let key = ClassDefId(ordinal);
                if self.verified_location(position, key, descriptor).is_some() {
                    return Some((position, key));
                }
            }
        }
        None
    }

    fn verified_location(
        &self,
        position: usize,
        class_def: ClassDefId,
        descriptor: &str,
    ) -> Option<(usize, ClassDefId)> {
        let image = self.image_at(position)?;
        let view = image.index.class(class_def).ok()?;
        let text = view.descriptor.utf8_lossy().ok()?;
        (text == descriptor).then_some((position, class_def))
    }

    pub fn class_facts(&self, class: ClassKey) -> Result<Arc<ClassFacts>, RequestError> {
        let image = self.image(class.dex).ok_or(RequestError::UnknownDex)?;
        image.index.class_facts_cached(class.class_def)
    }

    /// Recover one class from P0/P1 facts through the backend.
    pub fn class_source(&self, request: Request) -> Result<ClassSource, RequestError> {
        let image = self
            .image(request.class.dex)
            .ok_or(RequestError::UnknownDex)?;
        let input = request::backend_input(
            image,
            request.class.class_def,
            request.method.as_ref(),
            request.method.is_none(),
        )?;
        let output = decompile_class(input, self.jobs).map_err(|error| {
            RequestError::InternalInvariant(Box::leak(
                format!("backend failed: {error}").into_boxed_str(),
            ))
        })?;
        Ok(ClassSource {
            class: request.class,
            text: output.source,
            methods: output.methods,
            failures: output.failures,
        })
    }

    pub fn analyze(&self, request: Request) -> Result<ClassAnalysis, RequestError> {
        let class = request.class;
        let image = self.image(class.dex).ok_or(RequestError::UnknownDex)?;
        let facts = image.index.class_facts_cached(class.class_def)?;
        let source = self.class_source(request)?;
        let methods = source.methods;
        let failures = source.failures;
        let rendered = Rendered::external(class, source.text, facts.clone());
        Ok(ClassAnalysis {
            image: ImageFact {
                dex: class.dex,
                origin: image.image.origin,
                header_offset: image.image.header_offset,
            },
            facts,
            rendered,
            methods,
            failures,
        })
    }

    /// Builds the call-tree outline for one class or one of its methods.
    ///
    /// The outline reads P0 declarations and P1 invoke sites only; it never
    /// runs lowering or rendering.
    pub fn outline(
        &self,
        class: ClassKey,
        method: Option<MethodId>,
        depth: crate::outline::OutlineDepth,
    ) -> Result<crate::outline::OutlineDocument, RequestError> {
        outline::outline(self, class, method, depth)
    }

    pub fn analyze_method(
        &self,
        class: ClassKey,
        method: MethodId,
    ) -> Result<SingleMethodAnalysis, RequestError> {
        let facts = self.class_facts(class)?;
        let entry = facts
            .method_entry(method)
            .ok_or(RequestError::InternalInvariant("selected method is absent"))?;
        let Fact::Present(declaration) = entry.declaration() else {
            return Err(RequestError::InternalInvariant(
                "selected method declaration is unavailable",
            ));
        };
        let source = self.class_source(Request {
            class,
            method: Some(MethodSelector {
                name: declaration.name_units().to_vec(),
                prototype: declaration.prototype_units().to_vec(),
            }),
        })?;
        Ok(SingleMethodAnalysis {
            rendered: Rendered::external(class, source.text, facts),
            failures: source.failures,
        })
    }
}

fn descriptor_hash(descriptor: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    descriptor.hash(&mut hasher);
    hasher.finish()
}
