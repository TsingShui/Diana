//! Engine facade over P0 class facts and P1 decoded methods.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use crate::analysis::java_backend::{JavaDecompiler, JavaDecompilerConfig};
use crate::frontend::{
    ClassNode, DexError, DexResult, ExceptionHandler, MethodNode, TryCatchBlock,
};
use crate::input::{DexContainer, InputError, InputProblem, ProgramIndex};
use crate::ir::analysis::ClassHierarchyIndex;
use crate::ir::{AnalysisObserver, NullAnalysisObserver, CFG};
use crate::p0::{ClassFacts, Fact};
use crate::p1::{
    BootstrapDeclaration, CheckedBootstrapFact, CheckedMemberFact, CheckedMethodHandleFact,
    CheckedPrototype, CheckedReferenceFact, DecodedMethod,
};
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
    let source = facts.source();
    let bytes = source.instructions();
    let mut insns = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        insns.push(u16::from_le_bytes([pair[0], pair[1]]));
    }
    let tries = tries(facts);
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
    let result = crate::decoder::method_decoder::MethodDecoder::from_parts(
        &insns,
        &tries,
        source.registers_size(),
        source.ins_size(),
    )
    .decode();
    let mut cfg = crate::ir::splitter::Splitter::new("method")
        .method(context)
        .instructions(result.insns)
        .handlers(result.handlers)
        .registers(result.registers)
        .ins(result.ins)
        .build();
    resolve_symbols(facts, &mut cfg)?;
    Ok(cfg)
}

/// Resolve IR references directly from P1 checked reference facts.
fn resolve_symbols(facts: &DecodedMethod<'_>, ir: &mut CFG) -> DexResult<()> {
    for block in ir.blocks.values_mut() {
        for insn in &mut block.insns {
            let pc = crate::CodeUnitOffset(insn.offset);
            if insn.insn_type == crate::ir::InsnType::Invoke {
                match insn.payload.invoke_type {
                    Some(crate::ir::InvokeType::Polymorphic) => {
                        let fact = reference(facts, pc)?;
                        let CheckedReferenceFact::Method { owner, name, .. } = fact else {
                            return Err(DexError::Backend(format!(
                                "polymorphic invoke at {pc:?} has another kind"
                            )));
                        };
                        let proto = reference_slot(facts, pc, crate::raw::ReferenceSlot::Second)?;
                        let CheckedReferenceFact::Proto { prototype } = proto else {
                            return Err(DexError::Backend(format!(
                                "polymorphic invoke at {pc:?} has no checked prototype"
                            )));
                        };
                        let method = crate::ir::MethodReference {
                            owner: descriptor(*owner)?,
                            name: text(*name)?,
                            descriptor: prototype_descriptor(prototype)?,
                        };
                        apply_invoke_return_type(insn, &method);
                        regroup_invoke_args(insn, facts, pc, &method.descriptor)?;
                        insn.payload.reference =
                            Some(Box::new(crate::ir::MemberReference::Method(method)));
                        continue;
                    }
                    Some(crate::ir::InvokeType::Custom) => {
                        let fact = reference(facts, pc)?;
                        let CheckedReferenceFact::CallSite {
                            name: sam_name,
                            prototype,
                            handle,
                            declarations,
                            ..
                        } = fact
                        else {
                            return Err(DexError::Backend(format!(
                                "custom invoke at {pc:?} has another kind"
                            )));
                        };
                        let mut bootstrap = None;
                        for declaration in declarations {
                            if declaration.reference != handle.member() {
                                continue;
                            }
                            if let CheckedBootstrapFact::Member(CheckedMemberFact::Method {
                                owner,
                                name,
                                ..
                            }) = &declaration.fact
                            {
                                bootstrap = Some((*owner, *name));
                            }
                            break;
                        }
                        let Some((owner, name)) = bootstrap else {
                            return Err(DexError::Backend(format!(
                                "custom invoke at {pc:?} has no bootstrap method declaration"
                            )));
                        };
                        let bootstrap_owner = text(owner)?;
                        let bootstrap_name = text(name)?;

                        let bootstrap_owner_name = bootstrap_owner
                            .strip_prefix('L')
                            .and_then(|owner| owner.strip_suffix(';'))
                            .unwrap_or(&bootstrap_owner);
                        if bootstrap_owner_name == "java/lang/invoke/LambdaMetafactory"
                            && matches!(bootstrap_name.as_str(), "metafactory" | "altMetafactory")
                        {
                            insn.payload.lambda =
                                lambda_site(*sam_name, prototype, declarations)?.map(Box::new);
                        }
                        let method = crate::ir::MethodReference {
                            owner: descriptor(owner)?,
                            name: bootstrap_name,
                            descriptor: prototype_descriptor(prototype)?,
                        };
                        apply_invoke_return_type(insn, &method);
                        regroup_invoke_args(insn, facts, pc, &method.descriptor)?;
                        insn.payload.reference =
                            Some(Box::new(crate::ir::MemberReference::Method(method)));
                        continue;
                    }
                    _ => {}
                }
            }
            if insn.payload.method_index.is_some() {
                let fact = reference(facts, pc)?;
                let CheckedReferenceFact::Method {
                    owner,
                    name,
                    prototype,
                    ..
                } = fact
                else {
                    return Err(DexError::Backend(format!(
                        "method reference at {pc:?} has another kind"
                    )));
                };
                let method = crate::ir::MethodReference {
                    owner: descriptor(*owner)?,
                    name: text(*name)?,
                    descriptor: prototype_descriptor(prototype)?,
                };
                apply_invoke_return_type(insn, &method);
                regroup_invoke_args(insn, facts, pc, &method.descriptor)?;
                insn.payload.reference = Some(Box::new(crate::ir::MemberReference::Method(method)));
            }
            if insn.payload.field_index.is_some() {
                let fact = reference(facts, pc)?;
                let CheckedReferenceFact::Field {
                    owner,
                    name,
                    field_type,
                    ..
                } = fact
                else {
                    return Err(DexError::Backend(format!(
                        "field reference at {pc:?} has another kind"
                    )));
                };
                let field = crate::ir::FieldReference {
                    owner: descriptor(*owner)?,
                    name: text(*name)?,
                    field_type: descriptor(*field_type)?,
                };
                insn.payload.reference = Some(Box::new(crate::ir::MemberReference::Field(field)));
            }
            if insn.payload.string_index.is_some() && insn.payload.string_value.is_none() {
                let fact = reference(facts, pc)?;
                let CheckedReferenceFact::String { value } = fact else {
                    return Err(DexError::Backend(format!(
                        "string reference at {pc:?} has another kind"
                    )));
                };
                let units = value
                    .units()
                    .map_err(|error| DexError::Backend(error.to_string()))?;
                insn.payload.string_value =
                    Some(Box::new(crate::ir::Utf16String::from_utf16(units)));
            }
            if insn.payload.type_index.is_some() && insn.payload.class_type.is_none() {
                let fact = reference(facts, pc)?;
                let CheckedReferenceFact::Type { descriptor, .. } = fact else {
                    return Err(DexError::Backend(format!(
                        "type reference at {pc:?} has another kind"
                    )));
                };
                insn.payload.class_type = Some(Box::new(self::descriptor(*descriptor)?));
            }
        }
    }
    Ok(())
}

/// Recover the `LambdaMetafactory` bootstrap arguments that describe a SAM
/// implementation. The first two prototypes are the SAM and instantiated
/// signatures; the first method handle is the implementation.
fn lambda_site<'a>(
    sam_name: crate::input::DexString<'a>,
    interface: &CheckedPrototype<'a>,
    declarations: &[BootstrapDeclaration<'a>],
) -> DexResult<Option<crate::ir::LambdaSite>> {
    let mut sam = None;
    let mut implementation = None;
    let mut prototypes = 0usize;
    for declaration in declarations {
        match &declaration.fact {
            CheckedBootstrapFact::Proto { prototype } => {
                if prototypes == 0 {
                    sam = Some(prototype);
                }
                prototypes += 1;
            }
            CheckedBootstrapFact::MethodHandle {
                handle,
                member:
                    CheckedMemberFact::Method {
                        owner,
                        name,
                        prototype,
                        ..
                    },
            } if implementation.is_none() => {
                implementation = Some((handle, *owner, *name, prototype));
            }
            _ => {}
        }
    }
    let (Some(sam), Some((handle, owner, name, implementation_prototype))) = (sam, implementation)
    else {
        return Ok(None);
    };
    let interface = descriptor(interface.return_type())?;
    Ok(Some(crate::ir::LambdaSite {
        interface,
        sam_name: text(sam_name)?,
        sam_descriptor: prototype_descriptor(sam)?,
        implementation: crate::ir::MethodReference {
            owner: descriptor(owner)?,
            name: text(name)?,
            descriptor: prototype_descriptor(implementation_prototype)?,
        },
        // DEX method-handle kinds: 0x04 invoke-static, 0x05 invoke-instance,
        // 0x06 invoke-constructor, 0x07 invoke-direct, 0x08 invoke-interface.
        implementation_static: handle.kind() == 4,
        implementation_constructor: handle.kind() == 6,
    }))
}

/// Collapse the flat encoded register list into one IR operand per declared
/// parameter, using P1's checked layout. A wide parameter keeps its first
/// register and drops the high half; the register type comes from the checked
/// prototype instead of a later guess.
fn regroup_invoke_args(
    insn: &mut crate::ir::InsnNode,
    facts: &DecodedMethod<'_>,
    pc: crate::CodeUnitOffset,
    descriptor: &crate::ir::MethodDescriptor,
) -> DexResult<()> {
    let Some(Ok(layout)) = facts.invoke_layout_at(pc) else {
        return Ok(());
    };
    let mut words = insn.args.iter().cloned();
    let mut args = Vec::new();
    args.try_reserve_exact(usize::from(layout.has_receiver()) + layout.arguments().len())
        .map_err(|_| DexError::Backend("invoke argument allocation failed".to_owned()))?;
    if layout.has_receiver() {
        let receiver = words
            .next()
            .ok_or_else(|| DexError::Backend(format!("invoke at {pc:?} lost its receiver")))?;
        args.push(receiver);
    }
    for (width, parameter) in layout.arguments().iter().zip(&descriptor.parameters) {
        let Some(crate::ir::InsnArg::Reg(mut register)) = words.next() else {
            return Err(DexError::Backend(format!(
                "invoke at {pc:?} lost a declared argument"
            )));
        };
        register.ty = parameter.clone();
        args.push(crate::ir::InsnArg::Reg(register));
        if width.words() == 2 {
            words.next();
        }
    }
    insn.args = args;
    Ok(())
}

fn reference<'a>(
    facts: &'a DecodedMethod<'_>,
    pc: crate::CodeUnitOffset,
) -> DexResult<&'a CheckedReferenceFact<'a>> {
    reference_slot(facts, pc, crate::raw::ReferenceSlot::First)
}

fn reference_slot<'a>(
    facts: &'a DecodedMethod<'_>,
    pc: crate::CodeUnitOffset,
    slot: crate::raw::ReferenceSlot,
) -> DexResult<&'a CheckedReferenceFact<'a>> {
    facts
        .reference_fact(pc, slot)
        .ok_or_else(|| DexError::Backend(format!("missing reference fact at {pc:?}")))?
        .map_err(|error| DexError::Backend(error.to_string()))
}

fn text(value: crate::input::DexString<'_>) -> DexResult<String> {
    value
        .utf8_lossy()
        .map(|text| text.into_owned())
        .map_err(|error| DexError::Backend(error.to_string()))
}

fn descriptor(value: crate::input::DexString<'_>) -> DexResult<crate::ir::ArgType> {
    let text = text(value)?;
    text.parse().map_err(|source| DexError::InvalidDescriptor {
        descriptor: text,
        source,
    })
}

fn prototype_descriptor(
    prototype: &CheckedPrototype<'_>,
) -> DexResult<crate::ir::MethodDescriptor> {
    let mut parameters = Vec::new();
    parameters
        .try_reserve_exact(prototype.parameters().len())
        .map_err(|_| DexError::Backend("prototype parameter allocation failed".to_owned()))?;
    for parameter in prototype.parameters() {
        parameters.push(descriptor(parameter.descriptor())?);
    }
    Ok(crate::ir::MethodDescriptor {
        parameters,
        return_type: descriptor(prototype.return_type())?,
    })
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

fn tries(facts: &DecodedMethod<'_>) -> Vec<TryCatchBlock> {
    let boundaries: Vec<u32> = facts.try_boundaries().collect();
    let mut blocks = Vec::new();
    for pair in boundaries.chunks_exact(2) {
        let (start, end) = (pair[0], pair[1]);
        let clauses = facts
            .catch_clauses_at(crate::CodeUnitOffset(start))
            .unwrap_or_default();
        let mut handlers = Vec::new();
        for (exception, target) in clauses {
            let handler = match exception {
                Some(id) => facts
                    .catch_type(crate::TypeId(id))
                    .and_then(|fact| fact.descriptor().ok())
                    .and_then(|text| text.utf8_lossy().ok())
                    .and_then(|text| text.parse::<crate::ir::ArgType>().ok())
                    .map_or_else(
                        || ExceptionHandler::catch_all(target),
                        |ty| ExceptionHandler::typed(ty, target),
                    ),
                None => ExceptionHandler::catch_all(target),
            };
            handlers.push(handler);
        }
        blocks.push(TryCatchBlock {
            start_addr: start,
            end_addr: end,
            handlers,
        });
    }
    blocks
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
) -> Result<(Vec<crate::analysis::ClassMethodInput>, Vec<MethodOutcome>), DexError> {
    use rayon::prelude::*;

    let prepared: Vec<(MethodOutcome, Option<crate::analysis::ClassMethodInput>)> = methods
        .into_par_iter()
        .map(|facts| {
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
        })
        .collect();
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

fn build_class_parts<'a>(input: BackendInput<'a>) -> Result<ClassParts, DexError> {
    let class = crate::local::class_node(&input.class)
        .ok_or_else(|| DexError::Backend("class node assembly failed".to_owned()))?;
    let (methods, outcomes) = build_method_inputs(&class, input.methods)?;
    let mut nested = Vec::new();
    nested
        .try_reserve_exact(input.nested.len())
        .map_err(|_| DexError::Backend("nested class allocation failed".to_owned()))?;
    for child in input.nested {
        let parts = build_class_parts(child)?;
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

fn decompile_class<'a>(input: BackendInput<'a>) -> Result<ClassOutput, DexError> {
    let related = input.related.clone();
    let parts = build_class_parts(input)?;
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
            indexed.push(IndexedImage {
                image,
                index,
                dex,
                classes_by_descriptor: OnceLock::new(),
            });
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
        images.push(IndexedImage {
            image,
            index,
            dex,
            classes_by_descriptor: OnceLock::new(),
        });
        Ok(Self { images })
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
        let output = decompile_class(input).map_err(|error| {
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
