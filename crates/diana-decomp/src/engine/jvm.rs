//! Source recovery for a supplied JVM class through the ordinary Java backend.

mod annotations;
pub(crate) mod metadata;

use std::{fmt, sync::Arc};

use super::{hierarchy, java_abi, run_class_workers, MethodFailureObserver};
use crate::{
    analysis::{
        java_backend::{JavaDecompiler, JavaDecompilerConfig},
        ClassMethodInput, MethodRecoveryFailure, MethodRecoveryStage,
    },
    frontend::{
        access_flags, AccessInfo, ClassInfo, ClassNode, DexValue, FieldInfo, FieldNode, MethodInfo,
        MethodNode,
    },
    input::jvm::{constants::FieldValue, lower, ClassError, ClassFile, JvmString, Member},
    ir::{ArgType, MethodDescriptor, Utf16String},
    FailureKind,
};
use rayon::prelude::*;

pub struct Source {
    pub text: String,
    pub name: Vec<u16>,
    pub methods: Vec<SourceMember>,
    pub issues: Vec<SourceIssue>,
}

pub struct ClassText {
    pub text: String,
    pub name: Vec<u16>,
    pub issues: Vec<SourceIssue>,
}

pub enum ClassFailure<E> {
    Read(E),
    Recover(SourceError),
}

/// Borrows independent class files from a request and delivers each result
/// before claiming the next class. Caller scratch lives on its claiming
/// worker. The pool also executes method tasks for classes already in flight.
pub fn visit_class_texts<C, R, E>(
    count: usize,
    jobs: usize,
    context: impl Fn() -> C + Sync,
    read: impl for<'a> Fn(&'a mut C, usize) -> Result<ClassFile<'a>, R> + Sync,
    deliver: impl Fn(usize, Result<ClassText, ClassFailure<R>>) -> Result<(), E> + Sync,
) -> Result<(), E>
where
    E: From<SourceError> + Send,
{
    run_class_workers(
        count,
        jobs,
        |error| SourceError::Operation {
            kind: FailureKind::ResourceExhausted,
            reason: error.to_string(),
        },
        context,
        |worker, ordinal, parallel| {
            let result = read(worker, ordinal)
                .map_err(ClassFailure::Read)
                .and_then(|class| {
                    let jobs = (!parallel).then_some(1);
                    recover_impl(&class, None, jobs, false).map_err(ClassFailure::Recover)
                })
                .map(|source| ClassText {
                    text: source.text,
                    name: source.name,
                    issues: source.issues,
                });
            deliver(ordinal, result)
        },
    )
}

pub struct SourceMember {
    pub ordinal: u16,
    pub name: Vec<u16>,
    pub descriptor: Vec<u16>,
    pub access_flags: u16,
}

pub struct SourceIssue {
    pub method: Option<u16>,
    pub field: Option<u16>,
    pub offset_byte: Option<u16>,
    pub end_offset_byte: Option<u32>,
    pub stage: &'static str,
    pub kind: FailureKind,
    pub reason: String,
}

impl SourceIssue {
    fn declaration(field: Option<u16>, method: Option<u16>, error: SourceError) -> Self {
        Self {
            method,
            field,
            offset_byte: None,
            end_offset_byte: None,
            stage: "JVM declaration",
            kind: error.kind(),
            reason: error.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum SourceError {
    Input(ClassError),
    MissingMethod(u16),
    UnrepresentableName,
    Malformed(String),
    Backend(String),
    Operation { kind: FailureKind, reason: String },
}

impl SourceError {
    pub fn kind(&self) -> FailureKind {
        match self {
            Self::Input(error) => error.kind(),
            Self::UnrepresentableName => FailureKind::Unsupported,
            Self::MissingMethod(_) | Self::Malformed(_) => FailureKind::Malformed,
            Self::Backend(_) => FailureKind::InternalInvariant,
            Self::Operation { kind, .. } => *kind,
        }
    }
}

impl From<ClassError> for SourceError {
    fn from(error: ClassError) -> Self {
        Self::Input(error)
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(error) => error.fmt(formatter),
            Self::MissingMethod(ordinal) => {
                write!(formatter, "JVM method ordinal {ordinal} is absent")
            }
            Self::UnrepresentableName => {
                formatter.write_str("declaration name has unpaired UTF-16 surrogates")
            }
            Self::Malformed(reason) | Self::Backend(reason) | Self::Operation { reason, .. } => {
                formatter.write_str(reason)
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// Recovers the supplied class or one original method ordinal. Only the
/// selected class supplies declaration facts. Calls to other classes retain
/// their original constant-pool identities.
pub fn recover(
    class: &ClassFile<'_>,
    method: Option<u16>,
    jobs: usize,
) -> Result<Source, SourceError> {
    recover_impl(class, method, Some(jobs.max(1)), true)
}

fn recover_impl(
    class: &ClassFile<'_>,
    method: Option<u16>,
    jobs: Option<usize>,
    collect_members: bool,
) -> Result<Source, SourceError> {
    let original_name = class.name()?;
    let name = original_name.to_utf16()?;
    let descriptor = format!("L{};", declaration_text(original_name)?);
    let mut info = ClassInfo::from_type_descriptor(&descriptor).map_err(malformed)?;
    // A separately supplied class uses its full binary name. The enclosing
    // declaration is unavailable at this entry.
    info.name = info
        .full_name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_owned();
    let mut node = ClassNode::new(
        0,
        info,
        AccessInfo::for_class(u32::from(class.access_flags())),
    );
    let mut issues = Vec::new();
    let has_annotations = class.may_have_annotations();
    if has_annotations {
        node.annotations =
            annotations::read(class, annotations::Owner::Class, &mut issues).declaration;
    }
    match class
        .super_name()
        .map_err(SourceError::from)
        .and_then(|super_name| super_name.map(class_type).transpose())
    {
        Ok(Some(ty)) => node.set_super_class(ty),
        Ok(None) => (),
        Err(error) => issues.push(SourceIssue::declaration(None, None, error)),
    }
    for interface in class.interfaces() {
        match interface.map_err(SourceError::from).and_then(class_type) {
            Ok(ty) => node.add_interface(ty),
            Err(error) => issues.push(SourceIssue::declaration(None, None, error)),
        }
    }

    let generics = metadata::class_signature(class, &mut node, &mut issues);
    if method.is_none() {
        match class.fields() {
            Ok(fields) => {
                for (ordinal, field) in fields.enumerate() {
                    match field.map_err(SourceError::from).and_then(|field| {
                        field_node(
                            class,
                            field,
                            &descriptor,
                            generics.as_ref(),
                            has_annotations,
                            &mut issues,
                        )
                    }) {
                        Ok(field) => node.add_field(field),
                        Err(error) => issues.push(SourceIssue::declaration(
                            u16::try_from(ordinal).ok(),
                            None,
                            error,
                        )),
                    }
                }
            }
            Err(error) => issues.push(SourceIssue::declaration(None, None, error.into())),
        }
    }

    let mut methods = Vec::new();
    let mut members = Vec::new();
    let mut found = false;
    match class.methods() {
        Ok(rows) => {
            for (position, row) in rows.enumerate() {
                let ordinal = position as u16;
                let member = match row {
                    Ok(member) => member,
                    Err(error) => {
                        if method.is_none() || !found {
                            issues.push(SourceIssue::declaration(
                                None,
                                Some(ordinal),
                                error.into(),
                            ));
                        }
                        break;
                    }
                };
                let publish = method.is_none_or(|selected| member.ordinal() == selected);
                found |= publish;
                let mut context_issues = Vec::new();
                let local_issues = if publish {
                    &mut issues
                } else {
                    &mut context_issues
                };
                match method_node(
                    class,
                    member,
                    &descriptor,
                    generics.as_ref(),
                    has_annotations && publish,
                    collect_members && publish,
                    local_issues,
                ) {
                    Ok((declaration, identity)) => {
                        let position = node.methods().len();
                        if publish && !(declaration.is_native() || declaration.is_abstract()) {
                            methods.push((position, member));
                        }
                        node.add_method(declaration);
                        members.extend(identity);
                    }
                    Err(error) => local_issues.push(SourceIssue::declaration(
                        None,
                        Some(member.ordinal()),
                        error,
                    )),
                }
            }
        }
        Err(error) => issues.push(SourceIssue::declaration(None, None, error.into())),
    }
    if let Some(ordinal) = method {
        if !found {
            if let Some(issue) = issues.into_iter().next() {
                return Err(SourceError::Malformed(issue.reason));
            }
            return Err(SourceError::MissingMethod(ordinal));
        }
    }
    // Sibling declarations describe the selected method's calls and overloads.
    // Capture that context before reducing the publication to one original
    // member. Body preparation still covers only the requested definitions.
    let type_hierarchy = hierarchy(&[&node]);
    let source_abi = java_abi(&[&node]);
    if let Some(ordinal) = method {
        node.retain_methods(|declaration| declaration.id == u32::from(ordinal));
        if let Some((position, _)) = methods.first_mut() {
            *position = 0;
        }
    }
    let observer = Arc::new(MethodFailureObserver::default());
    let parallel = jobs != Some(1) && methods.len() >= 8;
    let work = || {
        let prepare = |(position, member)| prepare_method(class, &node, position, member);
        let lowered: Vec<_> = if parallel {
            methods.into_par_iter().map(prepare).collect()
        } else {
            methods.into_iter().map(prepare).collect()
        };
        let mut inputs = Vec::with_capacity(lowered.len());
        for (input, local) in lowered {
            inputs.push(input);
            issues.extend(local);
        }
        let text =
            JavaDecompiler::new(JavaDecompilerConfig::default().with_preserved_dex_names(true))
                .with_parallel_methods(parallel)
                .with_physical_methods()
                .with_shared_type_hierarchy(type_hierarchy)
                .with_source_abi(source_abi)
                .with_analysis_observer(observer.clone())
                .generate_class_with_nested(&node, inputs, Vec::new())
                .map_err(|error| SourceError::Backend(error.to_string()))?;
        Ok::<_, SourceError>((text, issues))
    };
    let (text, mut issues) = if let Some(jobs) = jobs.filter(|_| parallel) {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build()
            .map_err(|error| SourceError::Backend(error.to_string()))?;
        pool.install(work)?
    } else {
        work()?
    };
    for failure in observer.snapshot() {
        let ordinal = node
            .methods()
            .iter()
            .find(|method| {
                method.name() == failure.name && method.info.descriptor() == failure.prototype
            })
            .and_then(|method| u16::try_from(method.id).ok());
        if issues
            .iter()
            .any(|issue| issue.method == ordinal && issue.reason == failure.reason)
        {
            continue;
        }
        issues.push(SourceIssue {
            method: ordinal,
            field: None,
            offset_byte: None,
            end_offset_byte: None,
            stage: match failure.stage.as_str() {
                "method metadata" => "method metadata",
                "source lowering" => "source lowering",
                "JVM decoding" => "JVM decoding",
                _ => "semantic recovery",
            },
            kind: FailureKind::Unsupported,
            reason: failure.reason,
        });
    }
    Ok(Source {
        text,
        name,
        methods: members,
        issues,
    })
}

fn prepare_method(
    class: &ClassFile<'_>,
    node: &ClassNode,
    position: usize,
    member: Member<'_>,
) -> (ClassMethodInput, Vec<SourceIssue>) {
    let mut issues = Vec::new();
    let method = &node.methods()[position];
    let body = class
        .code_attribute(member)
        .map_err(SourceError::from)
        .and_then(|attribute| {
            attribute.ok_or_else(|| {
                SourceError::Malformed("concrete JVM method is missing Code".to_owned())
            })
        })
        .and_then(|attribute| lower::recover(class, attribute, node, method, &mut issues));
    let input = match body {
        Ok(cfg) => ClassMethodInput::decoded(position, cfg),
        Err(error) => {
            let issue = SourceIssue {
                method: Some(member.ordinal()),
                field: None,
                offset_byte: None,
                end_offset_byte: None,
                stage: "JVM decoding",
                kind: error.kind(),
                reason: error.to_string(),
            };
            let failure = MethodRecoveryFailure::new(MethodRecoveryStage::JvmDecode, &issue.reason);
            issues.push(issue);
            ClassMethodInput::failed(position, failure)
        }
    };
    (input, issues)
}

fn field_node(
    class: &ClassFile<'_>,
    member: Member<'_>,
    owner: &str,
    generics: Option<&crate::ir::generic_types::ClassSignature>,
    has_annotations: bool,
    issues: &mut Vec<SourceIssue>,
) -> Result<FieldNode, SourceError> {
    let name = declaration_text(member.name(class)?)?;
    let ty = declaration_text(member.descriptor(class)?)?
        .parse()
        .map_err(malformed)?;
    let mut field = FieldNode::new(
        u32::from(member.ordinal()),
        FieldInfo::new(owner.to_owned(), name, ty),
        AccessInfo::for_field(u32::from(member.access_flags())),
    );
    if member.access_flags() & 0x0008 != 0 {
        match class
            .field_constant(member)
            .map_err(SourceError::from)
            .and_then(|constant| {
                constant
                    .map(|value| constant_value(value.value, field.field_type()))
                    .transpose()
            }) {
            Ok(Some(value)) => {
                field = field.with_initial_value(value);
                field.jvm_constant_value = true;
            }
            Ok(None) => (),
            Err(error) => {
                field.default_preparation = false;
                issues.push(SourceIssue::declaration(
                    Some(member.ordinal()),
                    None,
                    error,
                ));
            }
        }
    }
    if has_annotations {
        field.annotations =
            annotations::read(class, annotations::Owner::Field(member), issues).declaration;
    }
    let signature = metadata::field_signature(class, member, field.field_type(), generics, issues);
    Ok(field.with_signature(signature))
}

fn method_node(
    class: &ClassFile<'_>,
    member: Member<'_>,
    owner: &str,
    generics: Option<&crate::ir::generic_types::ClassSignature>,
    has_annotations: bool,
    collect_identity: bool,
    issues: &mut Vec<SourceIssue>,
) -> Result<(MethodNode, Option<SourceMember>), SourceError> {
    let name = member.name(class)?;
    let descriptor = member.descriptor(class)?;
    let identity = if collect_identity {
        Some(SourceMember {
            ordinal: member.ordinal(),
            name: name.to_utf16()?,
            descriptor: descriptor.to_utf16()?,
            access_flags: member.access_flags(),
        })
    } else {
        None
    };
    let name = declaration_text(name)?;
    let ty: MethodDescriptor = declaration_text(descriptor)?.parse().map_err(malformed)?;
    let flags = u32::from(member.access_flags())
        | if name == "<init>" {
            access_flags::CONSTRUCTOR
        } else {
            0
        };
    let mut method = MethodNode::new(
        u32::from(member.ordinal()),
        MethodInfo::new(owner.to_owned(), name, ty.parameters, ty.return_type),
        AccessInfo::for_method(flags),
    );
    match class
        .declared_exceptions(member)
        .map_err(SourceError::from)
        .and_then(|entries| {
            entries
                .map(|entry| class_type(entry?))
                .collect::<Result<Vec<_>, _>>()
        }) {
        Ok(throws) => method = method.with_throws(throws),
        Err(error) => issues.push(SourceIssue::declaration(
            None,
            Some(member.ordinal()),
            error,
        )),
    }
    if has_annotations {
        let annotations = annotations::read(
            class,
            annotations::Owner::Method {
                member,
                parameters: method.param_types().len(),
                default_return_type: (class.access_flags() & 0x2200 == 0x2200
                    && method.can_have_annotation_default())
                .then(|| method.return_type()),
            },
            issues,
        );
        method.annotations = annotations.declaration;
        method.parameter_annotations = annotations.parameters;
        method.annotation_default = annotations.default;
    }
    let signature = metadata::method_signature(class, member, &method, generics, issues);
    method.parameter_names = metadata::parameter_names(class, member, &method, issues);
    Ok((method.with_signature(signature), identity))
}

fn constant_value(value: FieldValue<'_>, ty: &ArgType) -> Result<DexValue, SourceError> {
    Ok(match (value, ty) {
        (FieldValue::Integer(value), &ArgType::BOOLEAN) => DexValue::Boolean(value != 0),
        (FieldValue::Integer(value), &ArgType::BYTE) => DexValue::Byte(value as i8),
        (FieldValue::Integer(value), &ArgType::SHORT) => DexValue::Short(value as i16),
        (FieldValue::Integer(value), &ArgType::CHAR) => DexValue::Char(value as u16),
        (FieldValue::Integer(value), _) => DexValue::Int(value),
        (FieldValue::Long(value), _) => DexValue::Long(value),
        (FieldValue::Float(value), _) => DexValue::Float(f32::from_bits(value)),
        (FieldValue::Double(value), _) => DexValue::Double(f64::from_bits(value)),
        (FieldValue::String(value), _) => {
            DexValue::String(Utf16String::from_utf16(value.to_utf16()?))
        }
    })
}

fn declaration_text(value: JvmString<'_>) -> Result<String, SourceError> {
    if let Some(text) = value.utf8() {
        return Ok(text.to_owned());
    }
    char::decode_utf16(value.units())
        .collect::<Result<String, _>>()
        .map_err(|_| SourceError::UnrepresentableName)
}

pub(crate) fn class_type(name: JvmString<'_>) -> Result<ArgType, SourceError> {
    let name = declaration_text(name)?;
    if name.starts_with('[') {
        name.parse().map_err(malformed)
    } else {
        Ok(ArgType::object(&name))
    }
}

fn malformed(reason: impl fmt::Display) -> SourceError {
    SourceError::Malformed(reason.to_string())
}
