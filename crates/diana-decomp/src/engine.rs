//! Thin Engine facade over the original DEX facts and the recovery backend.

use std::sync::Arc;

use crate::analysis::java_backend::{JavaDecompiler, JavaDecompilerConfig};
use crate::frontend::{ClassNode, DexError, DexResult, MethodCode, MethodNode};
use crate::input::{DexContainer, InputError, InputProblem, ProgramIndex};
use crate::ir::analysis::ClassHierarchyIndex;
use crate::ir::{AnalysisObserver, NullAnalysisObserver, CFG};
use crate::local::LocalSymbols;
use crate::p0::{ClassFacts, Fact};
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
}

pub struct Engine<'a> {
    images: Vec<IndexedImage<'a>>,
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

/// Decode one method's original code into the recovery CFG.
fn decode_method_code(code: &MethodCode, method: &MethodNode) -> DexResult<CFG> {
    let owner = method
        .info
        .declaring_class
        .parse::<crate::ir::ArgType>()
        .map_err(|source| crate::frontend::DexError::InvalidDescriptor {
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
    let result = crate::decoder::method_decoder::MethodDecoder::from_code(code).decode();
    let mut cfg = crate::ir::splitter::Splitter::new("method")
        .method(context)
        .instructions(result.insns)
        .handlers(result.handlers)
        .registers(result.registers)
        .ins(result.ins)
        .build();
    cfg.debug_info = code.debug_info.clone();
    Ok(cfg)
}

/// Resolve the pool indices an IR still carries through a caller-supplied table.
fn resolve_symbols(source: &dyn crate::local::SymbolSource, ir: &mut CFG) -> DexResult<()> {
    for block in ir.blocks.values_mut() {
        for insn in &mut block.insns {
            if let Some(index) = insn.payload.method_index {
                let raw = source
                    .method(index)
                    .ok_or(crate::frontend::DexError::InvalidMethodIndex(index))?;
                let method = raw
                    .parse::<crate::ir::MethodReference>()
                    .map_err(|source| crate::frontend::DexError::InvalidMemberReference {
                        reference: raw.to_string(),
                        source,
                    })?;
                apply_invoke_return_type(insn, &method);
                insn.payload.reference = Some(Box::new(crate::ir::MemberReference::Method(method)));
            }
            if let Some(index) = insn.payload.field_index {
                let raw = source
                    .field(index)
                    .ok_or(crate::frontend::DexError::InvalidFieldIndex(index))?;
                let field = raw.parse::<crate::ir::FieldReference>().map_err(|source| {
                    crate::frontend::DexError::InvalidMemberReference {
                        reference: raw.to_string(),
                        source,
                    }
                })?;
                insn.payload.reference = Some(Box::new(crate::ir::MemberReference::Field(field)));
            }
            if let Some(index) = insn.payload.string_index {
                if insn.payload.string_value.is_none() {
                    let units = source
                        .string(index)
                        .ok_or(crate::frontend::DexError::InvalidStringIndex(index))?;
                    insn.payload.string_value =
                        Some(Box::new(crate::ir::Utf16String::from_utf16(units.to_vec())));
                }
            }
            if let Some(index) = insn.payload.type_index {
                if insn.payload.class_type.is_none() {
                    let descriptor = source
                        .ty(index)
                        .ok_or(crate::frontend::DexError::InvalidTypeIndex(index))?;
                    insn.payload.class_type =
                        Some(Box::new(descriptor.parse::<crate::ir::ArgType>().map_err(
                            |source| crate::frontend::DexError::InvalidDescriptor {
                                descriptor: descriptor.to_string(),
                                source,
                            },
                        )?));
                }
            }
        }
    }
    Ok(())
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

fn apply_invoke_return_type(insn: &mut crate::ir::InsnNode, method: &crate::ir::MethodReference) {
    if insn.insn_type != crate::ir::InsnType::Invoke {
        return;
    }
    let return_type = method.descriptor.return_type.clone();
    if return_type == crate::ir::ArgType::VOID {
        return;
    }
    if let Some(result) = insn.result.as_mut() {
        result.ty = return_type;
    }
}

/// One method handed to the class entry: its declaration, original code and the
/// identities its pool indices resolve to.
struct MethodInput {
    pub method: MethodNode,
    pub code: MethodCode,
    pub symbols: LocalSymbols,
}

/// Everything the class entry needs about one class.
struct ClassInput {
    pub class: ClassNode,
    pub methods: Vec<MethodInput>,
}

/// Per-method outcome of one class request.
struct MethodOutcome {
    pub name: String,
    pub descriptor: String,
    /// Present when this method alone could not be recovered.
    pub failure: Option<String>,
}

/// Class-level result: the assembled source plus one outcome per method.
struct ClassOutput {
    pub source: String,
    pub methods: Vec<MethodOutcome>,
}

/// Recover one class the way the engine drives it: the class declaration is
/// already established, methods decode and recover in parallel, then the class
/// source is assembled once. A method that fails keeps its own classified
/// outcome and never removes the others.
fn decompile_class(
    declarations: &[&ClassNode],
    input: ClassInput,
) -> Result<ClassOutput, DexError> {
    use rayon::prelude::*;

    let ClassInput { class, methods } = input;
    let mut prepared: Vec<(
        String,
        String,
        Option<String>,
        Option<crate::analysis::ClassMethodInput>,
    )> = methods
        .into_par_iter()
        .map(
            |MethodInput {
                 method,
                 code,
                 symbols,
             }| {
                let name = method.info.name.clone();
                let descriptor = method.info.descriptor().to_string();
                let recovered = decode_method_code(&code, &method).and_then(|mut cfg| {
                    resolve_symbols(&symbols, &mut cfg)?;
                    Ok(cfg)
                });
                match recovered {
                    Ok(cfg) => (
                        name,
                        descriptor,
                        None,
                        Some(crate::analysis::ClassMethodInput::decoded(method, cfg)),
                    ),
                    Err(error) => (
                        name,
                        descriptor,
                        Some(error.to_string()),
                        Some(crate::analysis::ClassMethodInput::failed(
                            method,
                            crate::analysis::MethodRecoveryFailure::new(
                                crate::analysis::MethodRecoveryStage::Decode,
                                error,
                            ),
                        )),
                    ),
                }
            },
        )
        .collect();
    let outcomes = prepared
        .iter()
        .map(|(name, descriptor, failure, _)| MethodOutcome {
            name: name.clone(),
            descriptor: descriptor.clone(),
            failure: failure.clone(),
        })
        .collect();
    let mut inputs: Vec<crate::analysis::ClassMethodInput> = prepared
        .drain(..)
        .filter_map(|(_, _, _, input)| input)
        .collect();
    let observer: Arc<dyn AnalysisObserver> = Arc::new(NullAnalysisObserver);
    let source =
        JavaDecompiler::new(JavaDecompilerConfig::default().with_preserved_dex_names(true))
            .with_shared_type_hierarchy(hierarchy(declarations))
            .with_source_abi(java_abi(declarations))
            .with_analysis_observer(observer)
            .generate_class_with_nested(&class, &mut inputs, Vec::new())
            .map_err(|error| DexError::Backend(error.to_string()))?;
    Ok(ClassOutput {
        source,
        methods: outcomes,
    })
}

impl<'a> Engine<'a> {
    pub fn with_method_jobs(self, _jobs: usize) -> Self {
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
            indexed.push(IndexedImage { image, index, dex });
        }
        Ok(Self { images: indexed })
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
        images.push(IndexedImage { image, index, dex });
        Ok(Self { images })
    }

    fn image(&self, dex: DexId) -> Option<&IndexedImage<'a>> {
        self.images.iter().find(|image| image.dex == dex)
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
        let facts = image.index.class_facts_cached(request.class.class_def)?;
        let mut node = crate::backend::class_node(&facts).ok_or(
            RequestError::InternalInvariant("class node assembly failed"),
        )?;
        let mut methods = Vec::new();
        let mut inputs = Vec::new();
        for entry in facts.methods() {
            let id = entry.method();
            if let Some(selector) = &request.method {
                let selected = facts.resolve_method(&selector.name, &selector.prototype)?;
                if selected != id {
                    continue;
                }
            }
            let Ok(source) = image.index.method_source(&facts, id) else {
                continue;
            };
            let crate::input::RawPrintable::Method(view) = source.raw() else {
                continue;
            };
            let Ok(decoded) = image.index.decode_method_source(&source) else {
                continue;
            };
            let Some(decoded) = decoded.decoded else {
                continue;
            };
            let Some(method) = node
                .methods()
                .iter()
                .find(|method| method.id == id.0)
                .cloned()
            else {
                continue;
            };
            methods.push(MethodSource {
                method: id,
                name: method.info.name.encode_utf16().collect(),
                prototype: method.info.descriptor().encode_utf16().collect(),
                failure: None,
            });
            let code =
                crate::backend::method_code(view, crate::backend::tries(&image.index, &decoded));
            if let Some(target) = node
                .methods_mut()
                .iter_mut()
                .find(|method| method.id == id.0)
            {
                target.code = Some(code.clone());
            }
            let mut method = method;
            method.code = Some(code.clone());
            inputs.push(MethodInput {
                method,
                code,
                symbols: crate::backend::symbols(&image.index, &decoded),
            });
        }

        let declaration_copy = node.clone();
        let mut related = Vec::new();
        let mut wanted = Vec::new();
        if let Fact::Present(super_class) = facts.superclass() {
            if let Ok(text) = String::from_utf16(super_class.units()) {
                wanted.push(text);
            }
        }
        if let Fact::Present(interfaces) = facts.interfaces() {
            for interface in interfaces {
                if let Fact::Present(descriptor) = &interface.descriptor {
                    if let Ok(text) = String::from_utf16(descriptor.units()) {
                        wanted.push(text);
                    }
                }
            }
        }
        for descriptor in wanted {
            for ordinal in 0..image.index.class_count() {
                let key = crate::ClassDefId(ordinal);
                let Ok(other) = image.index.class_facts_cached(key) else {
                    continue;
                };
                let Ok(text) = String::from_utf16(other.descriptor().units()) else {
                    continue;
                };
                if text == descriptor {
                    if let Some(node) = crate::backend::class_node(&other) {
                        related.push(node);
                    }
                    break;
                }
            }
        }

        let mut declarations: Vec<&crate::ClassNode> = vec![&declaration_copy];
        declarations.extend(related.iter());
        let output = decompile_class(
            &declarations,
            ClassInput {
                class: node,
                methods: inputs,
            },
        )
        .map_err(|error| {
            RequestError::InternalInvariant(Box::leak(
                format!("backend failed: {error}").into_boxed_str(),
            ))
        })?;
        for outcome in &output.methods {
            if let Some(failure) = &outcome.failure {
                if let Some(method) = methods.iter_mut().find(|method| {
                    String::from_utf16(&method.name).ok().as_deref() == Some(outcome.name.as_str())
                }) {
                    method.failure = Some(failure.clone());
                }
            }
        }
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
