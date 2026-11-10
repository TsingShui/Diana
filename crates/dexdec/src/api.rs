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
pub fn decode_method_code(code: &MethodCode) -> DexResult<CFG> {
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
pub fn resolve_symbols(source: &dyn crate::local::SymbolSource, ir: &mut CFG) -> DexResult<()> {
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
