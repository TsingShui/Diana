use crate::ir::Utf16String;
use smol_str::SmolStr;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JavaPrimitiveType {
    Void,
    Boolean,
    Byte,
    Short,
    Char,
    Int,
    Long,
    Float,
    Double,
}

#[derive(Clone)]
pub struct JavaIdentifier {
    spelling: IdentifierSpelling,
    local_reference: bool,
}

#[derive(Clone)]
enum IdentifierSpelling {
    Hint(SmolStr),
    Dex(SmolStr),
    Alias(std::sync::Arc<(SmolStr, SmolStr)>),
}

impl std::fmt::Debug for JavaIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JavaIdentifier")
            .field("source_name", &self.as_str())
            .field("dex_name", &self.dex_name())
            .finish()
    }
}

impl JavaIdentifier {
    /// Sanitize a DEX name the way JADX's `NameMapper` / rename checks do for
    /// fields and methods: keep valid printable identifiers, otherwise drop
    /// illegal characters instead of hex-escaping them.
    pub fn from_dex(value: &str) -> Self {
        let source_name = Self::sanitize_dex_name(value);
        Self::with_dex_spelling(value, source_name)
    }

    pub(crate) fn from_dex_alias(value: &str, source_name: impl Into<String>) -> Self {
        Self::with_dex_spelling(value, SmolStr::from(source_name.into()))
    }

    fn with_dex_spelling(original: &str, source_name: SmolStr) -> Self {
        let spelling = if original == source_name.as_str() {
            IdentifierSpelling::Dex(source_name)
        } else {
            IdentifierSpelling::Alias(std::sync::Arc::new((source_name, SmolStr::new(original))))
        };
        Self {
            spelling,
            local_reference: false,
        }
    }

    fn sanitize_dex_name(value: &str) -> SmolStr {
        if Self::is_valid_identifier(value) {
            return SmolStr::new(value);
        }
        let clean = Self::remove_invalid_chars(value, "C");
        if clean.is_empty() {
            return Self::alias_fallback(value).into();
        }
        if !Self::is_valid_identifier(&clean) {
            return format!("C{clean}").into();
        }
        clean.into()
    }

    pub fn from_hint(value: &str) -> Self {
        let source_name = if Self::is_valid_identifier(value) {
            SmolStr::new(value)
        } else if Self::is_reserved(value) && Self::is_all_chars_printable(value) {
            format!("{value}Value").into()
        } else {
            Self::sanitize_dex_name(value)
        };
        Self {
            spelling: IdentifierSpelling::Hint(source_name),
            local_reference: false,
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.spelling {
            IdentifierSpelling::Dex(name) | IdentifierSpelling::Hint(name) => name,
            IdentifierSpelling::Alias(names) => &names.0,
        }
    }

    fn dex_name(&self) -> Option<&str> {
        match &self.spelling {
            IdentifierSpelling::Dex(name) => Some(name),
            IdentifierSpelling::Hint(_) => None,
            IdentifierSpelling::Alias(names) => Some(&names.1),
        }
    }

    pub(crate) fn original_name(&self) -> &str {
        self.dex_name().unwrap_or_else(|| self.as_str())
    }

    fn with_source_name(&self, source_name: String) -> Self {
        let mut identifier = match self.dex_name() {
            Some(original) => Self::with_dex_spelling(original, source_name.into()),
            None => Self {
                spelling: IdentifierSpelling::Hint(source_name.into()),
                local_reference: false,
            },
        };
        identifier.local_reference = self.local_reference;
        identifier
    }

    pub(super) fn local_reference(mut self) -> Self {
        self.local_reference = true;
        self
    }

    pub(super) fn is_local_reference(&self) -> bool {
        self.local_reference
    }

    fn printed_name(&self) -> &str {
        if preserve_dex_names() {
            self.original_name()
        } else {
            self.as_str()
        }
    }

    fn alias_fallback(original: &str) -> String {
        // JADX falls back to `IAliasProvider` indexes. Without a global counter,
        // mirror `DeobfAliasProvider.prepareNamePart` for over-long names:
        // `x` + hex(Java String.hashCode()).
        format!("C{:x}", java_string_hash(original) as u32)
    }

    fn remove_invalid_chars_middle(name: &str) -> String {
        if Self::is_valid_identifier(name) {
            return name.to_string();
        }
        name.chars()
            .filter(|&character| {
                Self::is_printable_ascii(character) && Self::is_java_identifier_part(character)
            })
            .collect()
    }

    fn remove_invalid_chars(name: &str, prefix: &str) -> String {
        let result = Self::remove_invalid_chars_middle(name);
        if result.is_empty() {
            return result;
        }
        let first = result.chars().next().expect("non-empty");
        if !Self::is_java_identifier_start(first) {
            return format!("{prefix}{result}");
        }
        result
    }

    fn is_printable_ascii(character: char) -> bool {
        let code = character as u32;
        (32..=126).contains(&code)
    }

    fn is_all_chars_printable(value: &str) -> bool {
        value.chars().all(Self::is_printable_ascii)
    }

    fn is_java_identifier_start(character: char) -> bool {
        // ASCII subset of `Character.isJavaIdentifierStart` under JADX's
        // printable-ASCII rename filter.
        character.is_ascii_alphabetic() || character == '_' || character == '$'
    }

    fn is_java_identifier_part(character: char) -> bool {
        Self::is_java_identifier_start(character) || character.is_ascii_digit()
    }

    fn is_valid_identifier(value: &str) -> bool {
        let Some((&first, rest)) = value.as_bytes().split_first() else {
            return false;
        };
        Self::is_java_identifier_start(char::from(first))
            && rest
                .iter()
                .all(|&byte| Self::is_java_identifier_part(char::from(byte)))
            && !Self::is_reserved(value)
    }

    fn is_reserved(value: &str) -> bool {
        matches!(
            value,
            "abstract"
                | "assert"
                | "boolean"
                | "break"
                | "byte"
                | "case"
                | "catch"
                | "char"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "do"
                | "double"
                | "else"
                | "enum"
                | "extends"
                | "final"
                | "finally"
                | "float"
                | "for"
                | "goto"
                | "if"
                | "implements"
                | "import"
                | "instanceof"
                | "int"
                | "interface"
                | "long"
                | "native"
                | "new"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "short"
                | "static"
                | "strictfp"
                | "super"
                | "switch"
                | "synchronized"
                | "this"
                | "throw"
                | "throws"
                | "transient"
                | "try"
                | "void"
                | "volatile"
                | "while"
                | "true"
                | "false"
                | "null"
                | "_"
        )
    }
}

/// Java `String.hashCode()` over UTF-16 code units.
fn java_string_hash(value: &str) -> i32 {
    let mut hash: i32 = 0;
    for unit in value.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    hash
}

impl PartialEq for JavaIdentifier {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for JavaIdentifier {}

impl PartialOrd for JavaIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JavaIdentifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::hash::Hash for JavaIdentifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.as_str(), state);
    }
}

thread_local! {
    static PRESERVE_DEX_NAMES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn preserve_dex_names() -> bool {
    PRESERVE_DEX_NAMES.get()
}

pub(crate) fn with_preserved_dex_names<T>(enabled: bool, render: impl FnOnce() -> T) -> T {
    struct Restore(bool);

    impl Drop for Restore {
        fn drop(&mut self) {
            PRESERVE_DEX_NAMES.set(self.0);
        }
    }

    let previous = PRESERVE_DEX_NAMES.replace(enabled);
    let _restore = Restore(previous);
    render()
}

impl std::fmt::Display for JavaIdentifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.printed_name())
    }
}

#[derive(Debug, Clone, Default)]
pub struct JavaNameScope {
    used: std::collections::BTreeSet<JavaIdentifier>,
    next_suffix: std::collections::BTreeMap<JavaIdentifier, u64>,
}

impl JavaNameScope {
    pub fn reserve(&mut self, name: JavaIdentifier) -> JavaIdentifier {
        self.used.insert(name.clone());
        name
    }

    pub(crate) fn contains(&self, name: &JavaIdentifier) -> bool {
        self.used.contains(name)
    }

    pub fn claim(&mut self, preferred: JavaIdentifier) -> JavaIdentifier {
        if self.used.insert(preferred.clone()) {
            return preferred;
        }
        self.claim_numbered(&preferred, |name| preferred.with_source_name(name))
    }

    pub(crate) fn claim_hint_variant(&mut self, preferred: &JavaIdentifier) -> JavaIdentifier {
        self.claim_numbered(preferred, |name| JavaIdentifier::from_hint(&name))
    }

    fn claim_numbered(
        &mut self,
        preferred: &JavaIdentifier,
        identifier: impl Fn(String) -> JavaIdentifier,
    ) -> JavaIdentifier {
        // Names stay reserved for the lifetime of this scope, including earlier suffixes.
        let next = self.next_suffix.entry(preferred.clone()).or_insert(2);
        for suffix in *next.. {
            let candidate = identifier(format!("{}{suffix}", preferred.as_str()));
            if self.used.insert(candidate.clone()) {
                *next = suffix + 1;
                return candidate;
            }
        }
        unreachable!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaClassName {
    components: Vec<JavaIdentifier>,
}

impl JavaClassName {
    pub fn from_source(name: &str) -> Self {
        Self::from_components(name.split('.'))
    }

    pub fn from_components<'a>(components: impl IntoIterator<Item = &'a str>) -> Self {
        let components = components
            .into_iter()
            .map(JavaIdentifier::from_dex)
            .collect();
        Self { components }
    }

    pub fn from_identifiers(components: impl IntoIterator<Item = JavaIdentifier>) -> Self {
        Self {
            components: components.into_iter().collect(),
        }
    }

    pub fn simple(name: JavaIdentifier) -> Self {
        Self {
            components: vec![name],
        }
    }

    pub fn components(&self) -> &[JavaIdentifier] {
        &self.components
    }
}

impl std::fmt::Display for JavaClassName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, component) in self.components.iter().enumerate() {
            if index != 0 {
                formatter.write_str(".")?;
            }
            component.fmt(formatter)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JavaClassTypeSegment {
    pub name: JavaIdentifier,
    pub arguments: Vec<JavaTypeArgument>,
}

impl std::fmt::Display for JavaClassTypeSegment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(formatter)?;
        if !self.arguments.is_empty() {
            formatter.write_str("<")?;
            for (index, argument) in self.arguments.iter().enumerate() {
                if index != 0 {
                    formatter.write_str(", ")?;
                }
                argument.fmt(formatter)?;
            }
            formatter.write_str(">")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JavaClassType {
    pub segments: Arc<[JavaClassTypeSegment]>,
}

impl JavaClassType {
    pub fn segments_mut(&mut self) -> &mut [JavaClassTypeSegment] {
        Arc::make_mut(&mut self.segments)
    }

    pub fn raw(name: JavaClassName) -> Self {
        Self {
            segments: name
                .components
                .into_iter()
                .map(|name| JavaClassTypeSegment {
                    name,
                    arguments: Vec::new(),
                })
                .collect(),
        }
    }

    pub fn from_source(name: &str) -> Self {
        Self {
            segments: name
                .split('.')
                .map(|component| JavaClassTypeSegment {
                    name: JavaIdentifier::from_dex(component),
                    arguments: Vec::new(),
                })
                .collect(),
        }
    }

    pub(super) fn same_name(&self, other: &Self) -> bool {
        self.segments.len() == other.segments.len()
            && self
                .segments
                .iter()
                .zip(other.segments.iter())
                .all(|(left, right)| left.name == right.name)
    }

    pub(crate) fn printed_class_name(&self) -> std::borrow::Cow<'_, str> {
        let mut names = self
            .segments
            .iter()
            .map(|segment| segment.name.printed_name());
        let first = names.next().unwrap_or_default();
        if names.len() == 0 {
            return first.into();
        }
        let mut spelling = first.to_owned();
        for name in names {
            spelling.push('.');
            spelling.push_str(name);
        }
        spelling.into()
    }

    pub fn name(&self) -> JavaClassName {
        JavaClassName {
            components: self
                .segments
                .iter()
                .map(|segment| segment.name.clone())
                .collect(),
        }
    }
}

impl std::fmt::Display for JavaClassType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, segment) in self.segments.iter().enumerate() {
            if index != 0 {
                formatter.write_str(".")?;
            }
            segment.fmt(formatter)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JavaTypeArgument {
    Any,
    Exact(JavaType),
    Extends(JavaType),
    Super(JavaType),
}

impl std::fmt::Display for JavaTypeArgument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => formatter.write_str("?"),
            Self::Exact(ty) => ty.fmt(formatter),
            Self::Extends(ty) => write!(formatter, "? extends {ty}"),
            Self::Super(ty) => write!(formatter, "? super {ty}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JavaType {
    Primitive(JavaPrimitiveType),
    Class(JavaClassType),
    Variable(JavaIdentifier),
    Array(Box<JavaType>),
}

impl JavaType {
    pub fn source_class(source_name: &str) -> Self {
        Self::Class(JavaClassType::from_source(source_name))
    }

    pub fn array(element: JavaType) -> Self {
        Self::Array(Box::new(element))
    }

    pub const fn int() -> Self {
        Self::Primitive(JavaPrimitiveType::Int)
    }

    pub const fn boolean() -> Self {
        Self::Primitive(JavaPrimitiveType::Boolean)
    }

    pub(crate) fn into_raw(mut self) -> Self {
        let mut ty = &mut self;
        while let Self::Array(element) = ty {
            ty = element;
        }
        if let Self::Class(class) = ty {
            if class.segments.iter().all(|s| s.arguments.is_empty()) {
                return self;
            }
            for segment in class.segments_mut() {
                segment.arguments.clear();
            }
        }
        self
    }

    pub(crate) fn array_shape(&self) -> (&Self, usize) {
        let mut element = self;
        let mut rank = 0usize;
        while let Self::Array(inner) = element {
            element = inner;
            rank += 1;
        }
        (element, rank)
    }
}

impl std::fmt::Display for JavaType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive(primitive) => formatter.write_str(match primitive {
                JavaPrimitiveType::Void => "void",
                JavaPrimitiveType::Boolean => "boolean",
                JavaPrimitiveType::Byte => "byte",
                JavaPrimitiveType::Short => "short",
                JavaPrimitiveType::Char => "char",
                JavaPrimitiveType::Int => "int",
                JavaPrimitiveType::Long => "long",
                JavaPrimitiveType::Float => "float",
                JavaPrimitiveType::Double => "double",
            }),
            Self::Class(class) => class.fmt(formatter),
            Self::Variable(variable) => variable.fmt(formatter),
            Self::Array(element) => write!(formatter, "{element}[]"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaTypeParameter {
    pub name: JavaIdentifier,
    pub bounds: Vec<JavaType>,
}

impl std::fmt::Display for JavaTypeParameter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(formatter)?;
        if !self.bounds.is_empty() {
            formatter.write_str(" extends ")?;
            for (index, bound) in self.bounds.iter().enumerate() {
                if index != 0 {
                    formatter.write_str(" & ")?;
                }
                bound.fmt(formatter)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum JavaLiteral {
    Null,
    Boolean(bool),
    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Character(u16),
    String(Utf16String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaUnaryOp {
    Negate,
    LogicalNot,
    BitwiseNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaUpdateOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaBinaryOp {
    Add,
    /// The outer operator of a runtime concatenation that yields a new string.
    StringConcat,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Less,
    GreaterEqual,
    Greater,
    LessEqual,
    // These relations carry proof of integral promotion. Unlike the
    // unqualified relations, their opposites are exact complements.
    IntegralLess,
    IntegralGreaterEqual,
    IntegralGreater,
    IntegralLessEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaAssignOp {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
}

impl JavaUnaryOp {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::LogicalNot => "!",
            Self::BitwiseNot => "~",
        }
    }
}

impl JavaUpdateOp {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Increment => "++",
            Self::Decrement => "--",
        }
    }
}

impl JavaBinaryOp {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Add | Self::StringConcat => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::UnsignedShiftRight => ">>>",
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less | Self::IntegralLess => "<",
            Self::GreaterEqual | Self::IntegralGreaterEqual => ">=",
            Self::Greater | Self::IntegralGreater => ">",
            Self::LessEqual | Self::IntegralLessEqual => "<=",
        }
    }
}

impl JavaAssignOp {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Add => "+=",
            Self::Subtract => "-=",
            Self::Multiply => "*=",
            Self::Divide => "/=",
            Self::Remainder => "%=",
            Self::BitAnd => "&=",
            Self::BitOr => "|=",
            Self::BitXor => "^=",
            Self::ShiftLeft => "<<=",
            Self::ShiftRight => ">>=",
            Self::UnsignedShiftRight => ">>>=",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum JavaExpr {
    This,
    QualifiedThis(JavaType),
    Super,
    Name(JavaIdentifier),
    Literal(JavaLiteral),
    ClassLiteral(JavaType),
    Allocate(JavaType),
    Unavailable {
        reason: String,
        operands: Vec<JavaExpr>,
    },
    RecordOperation {
        site: std::sync::Arc<crate::ir::RecordSite>,
        operands: Vec<JavaExpr>,
    },
    DirectInit {
        receiver: Box<JavaExpr>,
        target: Box<crate::ir::MethodReference>,
        args: Vec<JavaExpr>,
    },
    Field {
        owner: Box<JavaExpr>,
        name: JavaIdentifier,
    },
    StaticField {
        owner: JavaType,
        name: JavaIdentifier,
    },
    ArrayAccess {
        array: Box<JavaExpr>,
        index: Box<JavaExpr>,
    },
    Call {
        receiver: Option<Box<JavaExpr>>,
        owner: Option<JavaType>,
        type_arguments: Vec<JavaType>,
        method: JavaIdentifier,
        args: Vec<JavaExpr>,
    },
    MethodReference {
        receiver: Box<JavaExpr>,
        method: JavaIdentifier,
    },
    Lambda {
        parameters: Vec<JavaIdentifier>,
        body: Box<JavaExpr>,
    },
    BlockLambda {
        parameters: Vec<JavaIdentifier>,
        body: Box<JavaStmt>,
    },
    New {
        enclosing: Option<Box<JavaExpr>>,
        ty: JavaType,
        target_type: Option<JavaType>,
        args: Vec<JavaExpr>,
        anonymous_body: Option<Box<super::JavaAnonymousClassBody>>,
    },
    NewArray {
        element_type: JavaType,
        dimensions: Vec<JavaExpr>,
        initializer: Vec<JavaExpr>,
    },
    Unary {
        op: JavaUnaryOp,
        operand: Box<JavaExpr>,
    },
    Update {
        op: JavaUpdateOp,
        target: Box<JavaExpr>,
        prefix: bool,
    },
    Binary {
        left: Box<JavaExpr>,
        op: JavaBinaryOp,
        right: Box<JavaExpr>,
    },
    Cast {
        ty: JavaType,
        value: Box<JavaExpr>,
    },
    InstanceOf {
        value: Box<JavaExpr>,
        ty: JavaType,
    },
    Conditional {
        condition: Box<JavaExpr>,
        when_true: Box<JavaExpr>,
        when_false: Box<JavaExpr>,
    },
    Assignment {
        target: Box<JavaExpr>,
        op: JavaAssignOp,
        value: Box<JavaExpr>,
    },
}

#[derive(Clone, Copy)]
enum NegationEdit {
    Boolean(bool),
    Comparison(JavaBinaryOp),
    RemoveNot,
    AddNot,
    Distribute(JavaBinaryOp),
}

impl NegationEdit {
    fn atom(expression: &JavaExpr) -> Option<Self> {
        Some(match expression {
            JavaExpr::Literal(JavaLiteral::Boolean(value)) => Self::Boolean(!value),
            JavaExpr::Unary {
                op: JavaUnaryOp::LogicalNot,
                ..
            } => Self::RemoveNot,
            JavaExpr::Binary {
                op: JavaBinaryOp::LogicalAnd | JavaBinaryOp::LogicalOr,
                ..
            } => return None,
            JavaExpr::Binary { op, .. } => JavaExpr::complementary_operator(*op)
                .map(Self::Comparison)
                .unwrap_or(Self::AddNot),
            _ => Self::AddNot,
        })
    }

    fn atom_cost(self) -> (u8, isize) {
        match self {
            Self::Boolean(_) | Self::Comparison(_) => (0, 0),
            Self::RemoveNot => (0, -1),
            Self::AddNot => (1, 1),
            Self::Distribute(_) => unreachable!("junction cost comes from its children"),
        }
    }

    fn apply_atom(self, current: &mut JavaExpr) {
        match (self, current) {
            (Self::Boolean(value), current) => {
                *current = JavaExpr::Literal(JavaLiteral::Boolean(value))
            }
            (Self::Comparison(complement), JavaExpr::Binary { op, .. }) => *op = complement,
            (Self::RemoveNot, current) => {
                let JavaExpr::Unary { operand, .. } = std::mem::replace(current, JavaExpr::This)
                else {
                    unreachable!("planned logical not");
                };
                *current = *operand;
            }
            (Self::AddNot, current) => {
                let operand = Box::new(std::mem::replace(current, JavaExpr::This));
                *current = JavaExpr::Unary {
                    op: JavaUnaryOp::LogicalNot,
                    operand,
                };
            }
            _ => unreachable!("negation edit matches its original expression"),
        }
    }
}

struct NegationPlan {
    edits: smallvec::SmallVec<[NegationEdit; 8]>,
    cost_delta: isize,
    #[cfg(test)]
    visits: usize,
}

impl NegationPlan {
    /// Decide whether each logical junction distributes its negation before
    /// editing any operand. A rejected distribution keeps its original subtree.
    fn analyze(expression: &JavaExpr) -> Self {
        enum Task<'a> {
            Visit(&'a JavaExpr),
            Join(usize),
        }
        let mut tasks = smallvec::SmallVec::<[Task<'_>; 8]>::new();
        tasks.push(Task::Visit(expression));
        let mut costs = smallvec::SmallVec::<[(u8, isize); 8]>::new();
        let mut plan = Self {
            edits: smallvec::SmallVec::new(),
            cost_delta: 0,
            #[cfg(test)]
            visits: 0,
        };
        while let Some(task) = tasks.pop() {
            let expression = match task {
                Task::Visit(expression) => expression,
                Task::Join(index) => {
                    let (right, right_delta) = costs.pop().expect("right negation cost");
                    let (left, left_delta) = costs.pop().expect("left negation cost");
                    let budget = left + right;
                    if budget <= 3 {
                        costs.push((budget, left_delta + right_delta));
                    } else {
                        // A wrapped junction counts as three for its parent,
                        // but adds only one unary node to the printed expression.
                        plan.edits.truncate(index + 1);
                        plan.edits[index] = NegationEdit::AddNot;
                        costs.push((3, 1));
                    }
                    continue;
                }
            };
            #[cfg(test)]
            {
                plan.visits += 1;
            }
            if let Some(edit) = NegationEdit::atom(expression) {
                plan.edits.push(edit);
                costs.push(edit.atom_cost());
            } else if let JavaExpr::Binary { left, op, right } = expression {
                let index = plan.edits.len();
                let inverse = if *op == JavaBinaryOp::LogicalAnd {
                    JavaBinaryOp::LogicalOr
                } else {
                    JavaBinaryOp::LogicalAnd
                };
                plan.edits.push(NegationEdit::Distribute(inverse));
                tasks.extend([Task::Join(index), Task::Visit(right), Task::Visit(left)]);
            } else {
                unreachable!("only logical junctions have a non-atomic negation");
            }
        }
        plan.cost_delta = costs.pop().expect("root negation cost").1;
        plan
    }

    fn apply(self, expression: &mut JavaExpr) {
        let mut pending = smallvec::SmallVec::<[&mut JavaExpr; 8]>::new();
        pending.push(expression);
        for edit in self.edits {
            let current = pending.pop().expect("negation operand");
            match (edit, current) {
                (NegationEdit::Distribute(inverse), JavaExpr::Binary { left, op, right }) => {
                    *op = inverse;
                    pending.push(right);
                    pending.push(left);
                }
                (edit, current) => edit.apply_atom(current),
            }
        }
    }
}

impl JavaExpr {
    pub(crate) fn negated(mut self) -> Self {
        if let Some(edit) = NegationEdit::atom(&self) {
            edit.apply_atom(&mut self);
        } else {
            NegationPlan::analyze(&self).apply(&mut self);
        }
        self
    }

    pub(super) fn negation_reduces_cost(&self) -> bool {
        match NegationEdit::atom(self) {
            Some(edit) => edit.atom_cost().1 < 0,
            None => NegationPlan::analyze(self).cost_delta < 0,
        }
    }

    fn complementary_operator(operator: JavaBinaryOp) -> Option<JavaBinaryOp> {
        Some(match operator {
            JavaBinaryOp::Equal => JavaBinaryOp::NotEqual,
            JavaBinaryOp::NotEqual => JavaBinaryOp::Equal,
            JavaBinaryOp::IntegralLess => JavaBinaryOp::IntegralGreaterEqual,
            JavaBinaryOp::IntegralGreaterEqual => JavaBinaryOp::IntegralLess,
            JavaBinaryOp::IntegralGreater => JavaBinaryOp::IntegralLessEqual,
            JavaBinaryOp::IntegralLessEqual => JavaBinaryOp::IntegralGreater,
            JavaBinaryOp::Less
            | JavaBinaryOp::GreaterEqual
            | JavaBinaryOp::Greater
            | JavaBinaryOp::LessEqual
            | JavaBinaryOp::Add
            | JavaBinaryOp::StringConcat
            | JavaBinaryOp::Subtract
            | JavaBinaryOp::Multiply
            | JavaBinaryOp::Divide
            | JavaBinaryOp::Remainder
            | JavaBinaryOp::BitAnd
            | JavaBinaryOp::BitOr
            | JavaBinaryOp::BitXor
            | JavaBinaryOp::ShiftLeft
            | JavaBinaryOp::ShiftRight
            | JavaBinaryOp::UnsignedShiftRight
            | JavaBinaryOp::LogicalAnd
            | JavaBinaryOp::LogicalOr => return None,
        })
    }

    pub(crate) fn cost(&self) -> usize {
        let mut cost = 0usize;
        let mut pending = vec![self];
        while let Some(expression) = pending.pop() {
            cost += match expression {
                Self::Conditional { .. } => 16,
                Self::Call { .. } => 2,
                _ => 1,
            };
            match expression {
                Self::Allocate(_) => {}
                Self::Unavailable { operands, .. } | Self::RecordOperation { operands, .. } => {
                    pending.extend(operands)
                }
                Self::DirectInit { receiver, args, .. } => {
                    pending.extend(args);
                    pending.push(receiver);
                }
                Self::Field { owner, .. } => pending.push(owner),
                Self::ArrayAccess { array, index } => {
                    pending.extend([array.as_ref(), index.as_ref()]);
                }
                Self::Call { receiver, args, .. } => {
                    pending.extend(args);
                    pending.extend(receiver.as_deref());
                }
                Self::MethodReference { receiver, .. } => pending.push(receiver),
                Self::Lambda { body, .. } => pending.push(body),
                Self::BlockLambda { .. } => {}
                Self::New {
                    enclosing, args, ..
                } => {
                    pending.extend(args);
                    pending.extend(enclosing.as_deref());
                }
                Self::NewArray {
                    dimensions,
                    initializer,
                    ..
                } => {
                    pending.extend(dimensions);
                    pending.extend(initializer);
                }
                Self::Unary { operand, .. }
                | Self::Update {
                    target: operand, ..
                }
                | Self::Cast { value: operand, .. }
                | Self::InstanceOf { value: operand, .. } => pending.push(operand),
                Self::Binary { left, right, .. }
                | Self::Assignment {
                    target: left,
                    value: right,
                    ..
                } => pending.extend([left.as_ref(), right.as_ref()]),
                Self::Conditional {
                    condition,
                    when_true,
                    when_false,
                } => pending.extend([condition.as_ref(), when_true.as_ref(), when_false.as_ref()]),
                Self::This
                | Self::QualifiedThis(_)
                | Self::Super
                | Self::Name(_)
                | Self::Literal(_)
                | Self::ClassLiteral(_)
                | Self::StaticField { .. } => {}
            }
        }
        cost
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JavaField {
    pub owner: JavaExpr,
    pub name: JavaIdentifier,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JavaCatch {
    pub types: Vec<JavaType>,
    pub variable: JavaIdentifier,
    pub body: JavaStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JavaSwitchCase {
    pub labels: Vec<JavaExpr>,
    pub body: Vec<JavaStmt>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaConstructorTarget {
    This,
    Super,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JavaStmt {
    Empty,
    /// Unavailable original code, rendered only as a Diana diagnostic comment.
    RecoveryFailure(String),
    /// An unresolved part of an otherwise recovered method.
    RecoveryGap(String),
    Block(Vec<JavaStmt>),
    Labeled {
        label: JavaIdentifier,
        body: Box<JavaStmt>,
    },
    Variable {
        ty: JavaType,
        name: JavaIdentifier,
        value: Option<JavaExpr>,
    },
    Expression(JavaExpr),
    ConstructorInvocation {
        target: JavaConstructorTarget,
        args: Vec<JavaExpr>,
    },
    Assign {
        target: JavaExpr,
        op: JavaAssignOp,
        value: JavaExpr,
    },
    If {
        condition: JavaExpr,
        then_stmt: Box<JavaStmt>,
        else_stmt: Option<Box<JavaStmt>>,
    },
    While {
        label: Option<JavaIdentifier>,
        condition: JavaExpr,
        body: Box<JavaStmt>,
    },
    DoWhile {
        label: Option<JavaIdentifier>,
        body: Box<JavaStmt>,
        condition: JavaExpr,
    },
    For {
        label: Option<JavaIdentifier>,
        init: Vec<JavaStmt>,
        condition: Option<JavaExpr>,
        update: Vec<JavaExpr>,
        body: Box<JavaStmt>,
    },
    ForEach {
        label: Option<JavaIdentifier>,
        ty: JavaType,
        variable: JavaIdentifier,
        iterable: JavaExpr,
        body: Box<JavaStmt>,
    },
    Switch {
        label: Option<JavaIdentifier>,
        selector: JavaExpr,
        cases: Vec<JavaSwitchCase>,
    },
    Try {
        body: Box<JavaStmt>,
        catches: Vec<JavaCatch>,
        finally: Option<Box<JavaStmt>>,
    },
    Synchronized {
        lock: JavaExpr,
        body: Box<JavaStmt>,
    },
    Return(Option<JavaExpr>),
    Throw(JavaExpr),
    Break(Option<JavaIdentifier>),
    Continue(Option<JavaIdentifier>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct JavaMethodBody {
    pub root: JavaStmt,
}

#[cfg(test)]
mod tests {
    use super::{with_preserved_dex_names, JavaIdentifier, JavaNameScope};

    #[test]
    fn invalid_dex_identifiers_drop_illegal_characters_like_jadx() {
        assert_eq!(JavaIdentifier::from_dex("measure-3").as_str(), "measure3");
        assert_eq!(JavaIdentifier::from_dex("do").as_str(), "Cdo");
        assert_eq!(JavaIdentifier::from_dex("1Foo").as_str(), "C1Foo");
        assert_eq!(
            JavaIdentifier::from_dex("$$$reportNull$$$0").as_str(),
            "$$$reportNull$$$0"
        );
    }

    #[test]
    fn long_printable_identifiers_are_kept_for_cut_file_name() {
        let name = "LongName".repeat(40);
        let identifier = JavaIdentifier::from_dex(&name);
        assert_eq!(identifier.as_str(), name);
        assert!(identifier.as_str().len() > 128);
    }

    #[test]
    fn original_dex_identifiers_are_selected_only_while_printing() {
        let identifier = JavaIdentifier::from_dex("02me");
        assert_eq!(identifier.as_str(), "C02me");
        assert_eq!(identifier.to_string(), "C02me");
        assert_eq!(
            with_preserved_dex_names(true, || identifier.to_string()),
            "02me"
        );
        assert_eq!(identifier.to_string(), "C02me");
    }

    #[test]
    fn collision_aliases_retain_the_original_dex_identifier() {
        let mut scope = JavaNameScope::default();
        scope.claim(JavaIdentifier::from_dex("a-"));
        let second = scope.claim(JavaIdentifier::from_dex("a!"));
        assert_eq!(second.as_str(), "a2");
        assert_eq!(with_preserved_dex_names(true, || second.to_string()), "a!");

        scope.reserve(JavaIdentifier::from_hint("a4"));
        scope.reserve(JavaIdentifier::from_hint("a22"));
        for (dex, expected) in [("a?", "a3"), ("a2!", "a23"), ("a%", "a5")] {
            let alias = scope.claim(JavaIdentifier::from_dex(dex));
            assert_eq!(alias.as_str(), expected);
            assert_eq!(with_preserved_dex_names(true, || alias.to_string()), dex);
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/java_negation.rs"]
mod negation_tests;

#[cfg(test)]
#[path = "../../../tests/internal/identifier_spelling.rs"]
mod spelling_tests;
