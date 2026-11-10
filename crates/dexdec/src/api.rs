//! Recovery entry: original facts in, source text out.
//!
//! The caller supplies the class declarations, the method and its control-flow
//! graph. This module runs the language backend and prints the result; it never
//! reads a DEX image, resolves class names or caches a second fact model.

use std::sync::Arc;

use crate::analysis::java_backend::{JavaDecompiler, JavaDecompilerConfig};
use crate::frontend::{ClassNode, DexError, DexResult, MethodCode, MethodNode};
use crate::ir::analysis::ClassHierarchyIndex;
use crate::ir::{AnalysisObserver, NullAnalysisObserver, CFG};
use crate::local::LocalSymbols;

pub use crate::frontend::DexError as MethodError;

/// Display options for one method or class rendering.
#[derive(Debug, Clone)]
pub struct MethodOptions {
    pub indent: String,
    pub preserve_dex_names: bool,
}

impl Default for MethodOptions {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            preserve_dex_names: true,
        }
    }
}

impl MethodOptions {
    fn java(&self) -> JavaDecompilerConfig {
        JavaDecompilerConfig::default().with_preserved_dex_names(self.preserve_dex_names)
    }
}

/// Decode one method's original code into the recovery CFG.
pub(crate) fn decode_method_code(code: &MethodCode) -> DexResult<CFG> {
    let result = crate::decoder::method_decoder::MethodDecoder::from_code(code).decode();
    let mut cfg = crate::ir::splitter::Splitter::new("method")
        .instructions(result.insns)
        .handlers(result.handlers)
        .registers(result.registers)
        .ins(result.ins)
        .build();
    cfg.debug_info = code.debug_info.clone();
    Ok(cfg)
}

/// Resolve the pool indices an IR still carries through a caller-supplied table.
pub(crate) fn resolve_symbols(
    source: &dyn crate::local::SymbolSource,
    ir: &mut CFG,
) -> DexResult<()> {
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
pub(crate) fn hierarchy(classes: &[&ClassNode]) -> Arc<ClassHierarchyIndex> {
    Arc::new(crate::analysis::method_override::type_hierarchy_from_classes(classes))
}

/// The target-language ABI facts the backend needs, built from the same classes.
pub(crate) fn java_abi(
    classes: &[&ClassNode],
) -> Arc<crate::analysis::java_backend::JavaSourceAbi> {
    Arc::new(crate::analysis::java_backend::JavaSourceAbi::analyze(
        classes.iter().copied(),
        |_| (Vec::new(), None),
    ))
}

/// Render one method. `classes` holds the declaring class and the declarations
/// the method touches; nothing else is consulted.
pub fn decompile_method(
    classes: &[&ClassNode],
    method: &MethodNode,
    cfg: &mut CFG,
    options: &MethodOptions,
) -> Result<String, DexError> {
    let class = classes
        .iter()
        .find(|class| class.info.type_descriptor == method.info.declaring_class)
        .copied()
        .or_else(|| classes.first().copied())
        .ok_or_else(|| DexError::MissingDexForClass(method.info.declaring_class.clone()))?;
    let observer: Arc<dyn AnalysisObserver> = Arc::new(NullAnalysisObserver);
    JavaDecompiler::new(options.java())
        .with_shared_type_hierarchy(hierarchy(classes))
        .with_source_abi(java_abi(classes))
        .with_analysis_observer(observer)
        .generate_method_with_context(class, method, cfg)
        .map_err(|error| DexError::Backend(error.to_string()))
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
pub struct MethodInput {
    pub method: MethodNode,
    pub code: MethodCode,
    pub symbols: LocalSymbols,
}

/// Everything the class entry needs about one class.
pub struct ClassInput {
    pub class: ClassNode,
    pub methods: Vec<MethodInput>,
}

/// Per-method outcome of one class request.
pub struct MethodOutcome {
    pub name: String,
    pub descriptor: String,
    /// Present when this method alone could not be recovered.
    pub failure: Option<String>,
}

/// Class-level result: the assembled source plus one outcome per method.
pub struct ClassOutput {
    pub source: String,
    pub methods: Vec<MethodOutcome>,
}

/// Recover one class the way the engine drives it: the class declaration is
/// already established, methods decode and recover in parallel, then the class
/// source is assembled once. A method that fails keeps its own classified
/// outcome and never removes the others.
pub fn decompile_class(
    declarations: &[&ClassNode],
    input: ClassInput,
    options: &MethodOptions,
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
                let recovered = decode_method_code(&code).and_then(|mut cfg| {
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
    let source = JavaDecompiler::new(options.java())
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
