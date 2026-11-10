//! Method node - represents a method in a class
//!
//! Similar to jadx's MethodNode, stores method information including:
//! - Method signature (name, parameters, return type)
//! - Access flags
//! - Code data (registers, instructions, try-catch handlers)
//! - Debug information

use super::{AccessInfo, AnnotationNode};
use crate::ir::generic_types::MethodSignature;
use crate::ir::ty::ArgType;

/// Unique identifier for a method within a DEX file
pub type MethodId = u32;

/// Method information (name + prototype + declaring class)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodInfo {
    /// Declaring class type (e.g., "Ljava/lang/String;")
    pub declaring_class: String,
    /// Method name
    pub name: String,
    /// Parameter types
    pub param_types: Vec<ArgType>,
    /// Return type
    pub return_type: ArgType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodOverrideSemantics {
    pub overridden_methods: Vec<MethodReference>,
    pub base_methods: Vec<MethodReference>,
    pub inherited_signature: Option<MethodSignature>,
    pub inherited_throws: Vec<ArgType>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodReference {
    pub declaring_class: String,
    pub short_id: String,
}

impl MethodInfo {
    /// Create new method info
    pub fn new(
        declaring_class: String,
        name: String,
        param_types: Vec<ArgType>,
        return_type: ArgType,
    ) -> Self {
        Self {
            declaring_class,
            name,
            param_types,
            return_type,
        }
    }

    /// Get short ID (name + prototype signature)
    /// Format: name(params)retType
    pub fn short_id(&self) -> String {
        format!("{}{}", self.name, self.descriptor())
    }

    /// Get descriptor signature
    /// Format: (params)retType
    pub fn descriptor(&self) -> String {
        let params: String = self.param_types.iter().map(|t| t.to_descriptor()).collect();
        format!("({}){}", params, self.return_type.to_descriptor())
    }

    /// Get full ID (class->name+prototype)
    pub fn full_id(&self) -> String {
        format!("{}->{}", self.declaring_class, self.short_id())
    }

    /// Check if this is a constructor
    pub fn is_constructor(&self) -> bool {
        self.name == "<init>"
    }

    /// Check if this is a static initializer
    pub fn is_class_init(&self) -> bool {
        self.name == "<clinit>"
    }

    /// Get readable signature
    pub fn readable_signature(&self) -> String {
        let params: Vec<String> = self.param_types.iter().map(|t| t.to_string()).collect();
        format!(
            "{}({}) -> {}",
            self.name,
            params.join(", "),
            self.return_type
        )
    }
}

/// Debug information for a method
#[derive(Debug, Clone, Default)]
pub struct DebugInfo {
    /// Line number table: (instruction offset, line number)
    pub line_numbers: Vec<(u32, u32)>,
    /// Local variable info: (name, type, start, end, register)
    pub local_vars: Vec<LocalVarInfo>,
    /// Parameter names
    pub param_names: Vec<Option<String>>,
}

/// Local variable debug information
#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    /// Variable name
    pub name: String,
    /// Variable type
    pub var_type: ArgType,
    /// Start instruction offset
    pub start_addr: u32,
    /// End instruction offset
    pub end_addr: u32,
    /// Register number
    pub register: u32,
}

/// Method node - represents a method in a class
#[derive(Debug)]
pub struct MethodNode {
    /// Method ID within DEX file
    pub id: MethodId,
    /// Method information
    pub info: MethodInfo,
    /// Access flags
    pub access_flags: AccessInfo,
    /// Checked exceptions declared through dalvik.annotation.Throws.
    pub throws: Vec<ArgType>,
    /// JVM generic signature recovered from the
    /// `Ldalvik/annotation/Signature;` method annotation, when present.
    pub signature: Option<String>,
    /// Kotlin source annotations preserved from DEX annotation sets.
    pub annotations: Vec<AnnotationNode>,
    /// Kotlin source annotations attached to each declared parameter.
    pub parameter_annotations: Vec<Vec<AnnotationNode>>,
    /// Semantic override relationship synthesized from class hierarchy.
    pub override_semantics: Option<MethodOverrideSemantics>,
}

impl MethodNode {
    /// Create a new method node without code
    pub fn new(id: MethodId, info: MethodInfo, access_flags: AccessInfo) -> Self {
        Self {
            id,
            info,
            access_flags,
            throws: Vec::new(),
            signature: None,
            annotations: Vec::new(),
            parameter_annotations: Vec::new(),
            override_semantics: None,
        }
    }

    /// Attach declared checked exceptions.
    pub fn with_throws(mut self, throws: Vec<ArgType>) -> Self {
        self.throws = throws;
        self
    }

    /// Attach a recovered JVM generic signature.
    pub fn with_signature(mut self, signature: Option<String>) -> Self {
        self.signature = signature;
        self
    }

    /// Get method name
    #[inline]
    pub fn name(&self) -> &str {
        &self.info.name
    }

    /// Get return type
    #[inline]
    pub fn return_type(&self) -> &ArgType {
        &self.info.return_type
    }

    /// Get parameter types
    #[inline]
    pub fn param_types(&self) -> &[ArgType] {
        &self.info.param_types
    }

    /// Get declared checked exceptions.
    #[inline]
    pub fn throws(&self) -> &[ArgType] {
        &self.throws
    }

    /// Get declaring class
    #[inline]
    pub fn declaring_class(&self) -> &str {
        &self.info.declaring_class
    }

    /// Check if static
    #[inline]
    pub fn is_static(&self) -> bool {
        self.access_flags.is_static()
    }

    /// Check if native
    #[inline]
    pub fn is_native(&self) -> bool {
        self.access_flags.is_native()
    }

    /// Check if abstract
    #[inline]
    pub fn is_abstract(&self) -> bool {
        self.access_flags.is_abstract()
    }

    /// Check if constructor
    #[inline]
    pub fn is_constructor(&self) -> bool {
        self.info.is_constructor()
    }

    /// Check if class initializer
    #[inline]
    pub fn is_class_init(&self) -> bool {
        self.info.is_class_init()
    }

    /// Count total arguments (including 'this' for instance methods)
    pub fn args_count(&self) -> usize {
        let explicit_args: usize = self
            .info
            .param_types
            .iter()
            .map(|t| if t.is_wide() { 2 } else { 1 })
            .sum();
        if self.is_static() {
            explicit_args
        } else {
            explicit_args + 1 // 'this' reference
        }
    }

    /// Get short ID
    pub fn short_id(&self) -> String {
        self.info.short_id()
    }

    /// Get full ID
    pub fn full_id(&self) -> String {
        self.info.full_id()
    }
}

impl std::fmt::Display for MethodNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.access_flags.to_modifier_string(),
            self.info.readable_signature()
        )
    }
}

impl Clone for MethodNode {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            info: self.info.clone(),
            access_flags: self.access_flags,
            throws: self.throws.clone(),
            signature: self.signature.clone(),
            annotations: self.annotations.clone(),
            parameter_annotations: self.parameter_annotations.clone(),
            override_semantics: self.override_semantics.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_info() {
        let info = MethodInfo::new(
            "Ljava/lang/String;".to_string(),
            "substring".to_string(),
            vec![ArgType::INT, ArgType::INT],
            ArgType::object("java/lang/String"),
        );
        assert_eq!(info.short_id(), "substring(II)Ljava/lang/String;");
        assert!(!info.is_constructor());
    }

    #[test]
    fn test_constructor() {
        let info = MethodInfo::new(
            "LTest;".to_string(),
            "<init>".to_string(),
            vec![],
            ArgType::VOID,
        );
        assert!(info.is_constructor());
        assert_eq!(info.short_id(), "<init>()V");
    }

    #[test]
    fn test_method_node() {
        let info = MethodInfo::new(
            "LTest;".to_string(),
            "add".to_string(),
            vec![ArgType::INT, ArgType::INT],
            ArgType::INT,
        );
        let method = MethodNode::new(0, info, AccessInfo::for_method(0x0009)); // public static

        assert!(method.is_static());
        assert_eq!(method.name(), "add");
        assert_eq!(method.args_count(), 2); // static, no 'this'
    }

    #[test]
    fn test_instance_method_args() {
        let info = MethodInfo::new(
            "LTest;".to_string(),
            "getName".to_string(),
            vec![],
            ArgType::object("java/lang/String"),
        );
        let method = MethodNode::new(0, info, AccessInfo::for_method(0x0001)); // public

        assert!(!method.is_static());
        assert_eq!(method.args_count(), 1); // 'this' only
    }
}
