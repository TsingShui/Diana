//! Engine facade over P0 class facts and P1 decoded methods.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use crate::analysis::java_backend::{JavaDecompiler, JavaDecompilerConfig};
use crate::frontend::{ClassNode, DexError, DexResult, MethodNode};
use crate::input::{DexContainer, InputError, InputProblem, ProgramIndex};
use crate::ir::analysis::ClassHierarchyIndex;
use crate::ir::{AnalysisObserver, NullAnalysisObserver, CFG};
use crate::p0::{ClassFacts, Fact};
use crate::p1::DecodedMethod;
use crate::p9::{RenderOutcome, Rendered};
use crate::{ByteOffset, ClassKey, DexId, ImageFact, ImageOrigin, MethodId, RequestError};

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

struct IndexedImage<'a> {
    image: DexImage<'a>,
    index: ProgramIndex<'a>,
    dex: DexId,
    classes_by_descriptor: OnceLock<HashMap<String, crate::ClassDefId>>,
}

impl<'a> IndexedImage<'a> {
    fn classes_by_descriptor(&self) -> &HashMap<String, crate::ClassDefId> {
        self.classes_by_descriptor.get_or_init(|| {
            let mut classes = HashMap::new();
            for ordinal in 0..self.index.class_count() {
                let key = crate::ClassDefId(ordinal);
                let Ok(view) = self.index.class(key) else {
                    continue;
                };
                let Ok(descriptor) = view.descriptor.utf8_lossy() else {
                    continue;
                };
                classes.insert(descriptor.into_owned(), key);
            }
            classes
        })
    }
}

pub struct Engine<'a> {
    images: Vec<IndexedImage<'a>>,
    jobs: Option<usize>,
}

pub struct ClassSource {
    pub class: ClassKey,
    pub text: String,
    pub methods: Vec<MethodSource>,
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
    pub rendered: RenderOutcome,
}

pub struct SingleMethodAnalysis {
    pub rendered: RenderOutcome,
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

enum MethodFacts<'a> {
    Decoded(DecodedMethod<'a>),
    Failed { method: MethodId, failure: String },
}

/// One complete P0/P1 delivery for a backend class request.
struct BackendInput<'a> {
    class: Arc<ClassFacts>,
    related: Vec<Arc<ClassFacts>>,
    methods: Vec<MethodFacts<'a>>,
    nested: Vec<BackendInput<'a>>,
}

struct MethodOutcome {
    pub method: MethodId,
    pub name: String,
    pub descriptor: String,
    pub failure: Option<String>,
}

struct ClassOutput {
    pub source: String,
    pub methods: Vec<MethodOutcome>,
}

struct ClassParts {
    class: ClassNode,
    methods: Vec<crate::analysis::ClassMethodInput>,
    outcomes: Vec<MethodOutcome>,
    nested: Vec<crate::analysis::NestedClassInput>,
}

fn build_method_inputs<'a>(
    class: &ClassNode,
    methods: Vec<MethodFacts<'a>>,
    jobs: Option<usize>,
) -> Result<(Vec<crate::analysis::ClassMethodInput>, Vec<MethodOutcome>), DexError> {
    use rayon::prelude::*;

    let prepare = |facts| {
        let method_id = match &facts {
            MethodFacts::Decoded(facts) => facts.method_id(),
            MethodFacts::Failed { method, .. } => *method,
        };
        let Some(method) = class
            .methods()
            .iter()
            .find(|method| method.id == method_id.0)
            .cloned()
        else {
            return (
                MethodOutcome {
                    method: method_id,
                    name: format!("method#{}", method_id.0),
                    descriptor: String::new(),
                    failure: Some("class node method is missing".to_owned()),
                },
                None,
            );
        };
        let name = method.info.name.clone();
        let descriptor = method.info.descriptor().to_string();
        match facts {
            MethodFacts::Decoded(facts) => match decode_decoded_method(&method, &facts) {
                Ok(cfg) => (
                    MethodOutcome {
                        method: method_id,
                        name,
                        descriptor,
                        failure: None,
                    },
                    Some(crate::analysis::ClassMethodInput::decoded(method, cfg)),
                ),
                Err(error) => (
                    MethodOutcome {
                        method: method_id,
                        name,
                        descriptor,
                        failure: Some(error.to_string()),
                    },
                    Some(crate::analysis::ClassMethodInput::failed(
                        method,
                        crate::analysis::MethodRecoveryFailure::new(
                            crate::analysis::MethodRecoveryStage::Decode,
                            error,
                        ),
                    )),
                ),
            },
            MethodFacts::Failed { failure, .. } => (
                MethodOutcome {
                    method: method_id,
                    name,
                    descriptor,
                    failure: Some(failure.clone()),
                },
                Some(crate::analysis::ClassMethodInput::failed(
                    method,
                    crate::analysis::MethodRecoveryFailure::new(
                        crate::analysis::MethodRecoveryStage::Decode,
                        failure,
                    ),
                )),
            ),
        }
    };
    let prepared: Vec<(MethodOutcome, Option<crate::analysis::ClassMethodInput>)> =
        if jobs == Some(1) {
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
    let outcomes = prepared
        .iter()
        .map(|(outcome, _)| MethodOutcome {
            method: outcome.method,
            name: outcome.name.clone(),
            descriptor: outcome.descriptor.clone(),
            failure: outcome.failure.clone(),
        })
        .collect();
    let inputs = prepared
        .into_iter()
        .filter_map(|(_, input)| input)
        .collect();
    Ok((inputs, outcomes))
}

fn build_class_parts<'a>(
    input: BackendInput<'a>,
    jobs: Option<usize>,
) -> Result<ClassParts, DexError> {
    let class = crate::local::class_node(&input.class)
        .ok_or_else(|| DexError::Backend("class node assembly failed".to_owned()))?;
    let (methods, outcomes) = build_method_inputs(&class, input.methods, jobs)?;
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
        outcomes,
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
        if let Some(node) = crate::local::class_node(facts) {
            related_nodes.push(node);
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
    let observer: Arc<dyn AnalysisObserver> = Arc::new(NullAnalysisObserver);
    let mut methods = parts.methods;
    let source =
        JavaDecompiler::new(JavaDecompilerConfig::default().with_preserved_dex_names(true))
            .with_shared_type_hierarchy(hierarchy(&declarations))
            .with_source_abi(java_abi(&declarations))
            .with_analysis_observer(observer)
            .generate_class_with_nested(&parts.class, &mut methods, parts.nested)
            .map_err(|error| DexError::Backend(error.to_string()))?;
    Ok(ClassOutput {
        source,
        methods: parts.outcomes,
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
            let container = DexContainer::open(image.bytes)?;
            let index = container.program(image.header_offset)?.with_dex(dex);
            indexed.push(IndexedImage {
                image,
                index,
                dex,
                classes_by_descriptor: OnceLock::new(),
            });
        }
        Ok(Self {
            images: indexed,
            jobs: None,
        })
    }

    pub fn focus(image: DexImage<'a>, dex: DexId) -> Result<Self, InputError> {
        let container = DexContainer::open(image.bytes)?;
        let index = container.program(image.header_offset)?.with_dex(dex);
        let mut images = Vec::new();
        images
            .try_reserve_exact(1)
            .map_err(|_| InputError::Allocation {
                offset_byte: image.header_offset,
            })?;
        images.push(IndexedImage {
            image,
            index,
            dex,
            classes_by_descriptor: OnceLock::new(),
        });
        Ok(Self { images, jobs: None })
    }

    fn image(&self, dex: DexId) -> Option<&IndexedImage<'a>> {
        self.images.iter().find(|image| image.dex == dex)
    }

    pub fn class_facts(&self, class: ClassKey) -> Result<Arc<ClassFacts>, RequestError> {
        let image = self.image(class.dex).ok_or(RequestError::UnknownDex)?;
        image.index.class_facts_cached(class.class_def)
    }

    fn backend_input(
        &self,
        image: &IndexedImage<'a>,
        class_def: crate::ClassDefId,
        selector: Option<&MethodSelector>,
        include_nested: bool,
    ) -> Result<BackendInput<'a>, RequestError> {
        let facts = image.index.class_facts_cached(class_def)?;
        let mut methods = Vec::new();
        for entry in facts.methods() {
            let id = entry.method();
            if let Some(selector) = selector {
                let selected = facts.resolve_method(&selector.name, &selector.prototype)?;
                if selected != id {
                    continue;
                }
            }
            let source = match image.index.method_source(&facts, id) {
                Ok(source) => source,
                Err(error) => {
                    methods.push(MethodFacts::Failed {
                        method: id,
                        failure: error.to_string(),
                    });
                    continue;
                }
            };
            if !matches!(source.raw(), crate::input::RawPrintable::Method(_)) {
                continue;
            }
            let decoded = match image.index.decode_method_source(&source) {
                Ok(decoded) => decoded,
                Err(error) => {
                    methods.push(MethodFacts::Failed {
                        method: id,
                        failure: error.to_string(),
                    });
                    continue;
                }
            };
            match decoded.decoded {
                Some(decoded) => methods.push(MethodFacts::Decoded(decoded)),
                None => methods.push(MethodFacts::Failed {
                    method: id,
                    failure: decoded
                        .error
                        .map_or_else(|| "method decoding stopped".to_owned(), |e| e.to_string()),
                }),
            }
        }

        let classes = image.classes_by_descriptor();
        let mut related = Vec::new();
        let mut visited = BTreeSet::new();
        visited.insert(class_def);
        let mut frontier = vec![facts.clone()];
        while let Some(current) = frontier.pop() {
            let mut wanted = Vec::new();
            if let Fact::Present(super_class) = current.superclass() {
                wanted.push(super_class);
            }
            if let Fact::Present(interfaces) = current.interfaces() {
                for interface in interfaces {
                    if let Fact::Present(descriptor) = &interface.descriptor {
                        wanted.push(descriptor);
                    }
                }
            }
            for descriptor in wanted {
                let Ok(text) = String::from_utf16(descriptor.units()) else {
                    continue;
                };
                let Some(&key) = classes.get(&text) else {
                    continue;
                };
                if !visited.insert(key) {
                    continue;
                }
                let Ok(other) = image.index.class_facts_cached(key) else {
                    continue;
                };
                frontier.push(other.clone());
                related.push(other);
            }
        }

        let mut nested = Vec::new();
        if include_nested {
            let Ok(current) = String::from_utf16(facts.descriptor().units()) else {
                return Err(RequestError::InternalInvariant(
                    "class descriptor is not UTF-16",
                ));
            };
            let base = current.strip_suffix(';').unwrap_or(&current);
            let prefix = format!("{base}$");
            let mut children = Vec::new();
            for (descriptor, key) in classes {
                if *key == class_def {
                    continue;
                }
                let Some(rest) = descriptor.strip_prefix(&prefix) else {
                    continue;
                };
                let name = rest.strip_suffix(';').unwrap_or(rest);
                if name.contains('$') {
                    continue;
                }
                children.push(*key);
            }
            children.sort_unstable();
            for key in children {
                nested.push(self.backend_input(image, key, None, true)?);
            }
        }

        Ok(BackendInput {
            class: facts,
            related,
            methods,
            nested,
        })
    }

    /// Recover one class from P0/P1 facts through the backend.
    pub fn class_source(&self, request: Request) -> Result<ClassSource, RequestError> {
        let image = self
            .image(request.class.dex)
            .ok_or(RequestError::UnknownDex)?;
        let input = self.backend_input(
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
        let methods = output
            .methods
            .into_iter()
            .map(|outcome| MethodSource {
                method: outcome.method,
                name: outcome.name.encode_utf16().collect(),
                prototype: outcome.descriptor.encode_utf16().collect(),
                failure: outcome.failure,
            })
            .collect();
        Ok(ClassSource {
            class: request.class,
            text: output.source,
            methods,
        })
    }

    pub fn analyze(&self, request: Request) -> Result<ClassAnalysis, RequestError> {
        let class = request.class;
        let image = self.image(class.dex).ok_or(RequestError::UnknownDex)?;
        let facts = image.index.class_facts_cached(class.class_def)?;
        let source = self.class_source(request)?;
        let rendered = RenderOutcome::Ready(Rendered::external(class, source.text, facts.clone()));
        Ok(ClassAnalysis {
            image: ImageFact {
                dex: class.dex,
                origin: image.image.origin,
                header_offset: image.image.header_offset,
            },
            facts,
            rendered,
        })
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
            rendered: RenderOutcome::Ready(Rendered::external(class, source.text, facts)),
        })
    }
}
