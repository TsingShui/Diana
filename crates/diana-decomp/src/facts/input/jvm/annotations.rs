//! Original declaration annotations and their checked element values.

use super::members::MemberKind;
use super::{
    error, Attribute, ClassError, ClassFile, ClassProblem, Constant, JvmString, Member, Reader,
};

const MAX_ANNOTATION_NESTING: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    RuntimeVisible,
    RuntimeInvisible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target<'a> {
    Declaration,
    Parameter {
        encoded_index: u8,
        descriptor_index: Option<u8>,
    },
    TypeUse {
        target_type: u8,
        target_info: &'a [u8],
        type_path: &'a [u8],
    },
}

#[derive(Debug, Default)]
pub struct DeclarationAnnotations<'a> {
    pub annotations: Vec<FoundAnnotation<'a>>,
    pub defaults: Vec<ElementValue<'a>>,
    pub issues: Vec<ClassError>,
    pub(crate) source_default: Option<usize>,
}

#[derive(Debug)]
pub struct FoundAnnotation<'a> {
    pub target: Target<'a>,
    pub annotation: Annotation<'a>,
}

#[derive(Debug)]
pub struct Annotation<'a> {
    pub visibility: Visibility,
    pub descriptor: Option<JvmString<'a>>,
    pub offset: usize,
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug)]
pub struct Element<'a> {
    pub name: Option<JvmString<'a>>,
    pub value: ElementValue<'a>,
}

#[derive(Debug)]
pub struct ElementValue<'a> {
    pub offset: usize,
    pub value: Value<'a>,
}

#[derive(Debug)]
pub enum Value<'a> {
    Unavailable,
    Byte(i8),
    Char(u16),
    Double(u64),
    Float(u32),
    Int(i32),
    Long(i64),
    Short(i16),
    Boolean(bool),
    String(JvmString<'a>),
    Enum {
        descriptor: Option<JvmString<'a>>,
        name: Option<JvmString<'a>>,
    },
    Class(JvmString<'a>),
    Annotation(Box<Annotation<'a>>),
    Array {
        values: Vec<ElementValue<'a>>,
        complete: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ParameterTable<'a> {
    rows: &'a [[u8; 4]],
    issue: Option<ClassError>,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameter {
    pub name_index: u16,
    pub access_flags: u16,
}

impl<'a> ParameterTable<'a> {
    pub fn issue(self) -> Option<ClassError> {
        self.issue
    }

    pub fn entries(self) -> impl Iterator<Item = Parameter> + 'a {
        self.rows.iter().map(|&[nhi, nlo, fhi, flo]| Parameter {
            name_index: u16::from_be_bytes([nhi, nlo]),
            access_flags: u16::from_be_bytes([fhi, flo]),
        })
    }
}

impl<'a> ClassFile<'a> {
    /// Reads the optional standard method-parameter table. An incomplete
    /// suffix leaves its checked prefix available, but cannot align omissions.
    pub fn method_parameters(
        &self,
        member: Member<'a>,
        expected: usize,
    ) -> Result<Option<ParameterTable<'a>>, ClassError> {
        member.verify(self)?;
        if member.kind() != MemberKind::Method {
            return Ok(None);
        }
        let Some(attribute) = self.find_attribute(member.attributes(), b"MethodParameters")? else {
            return Ok(None);
        };
        let (&count, body) = attribute
            .payload
            .split_first()
            .ok_or_else(|| error(attribute.offset, ClassProblem::Truncated))?;
        if usize::from(count) != expected {
            return Err(error(
                attribute.offset,
                ClassProblem::InvalidMethodParametersCount {
                    encoded: count,
                    descriptor: expected,
                },
            ));
        }
        let rows = body.as_chunks::<4>().0;
        let issue = (body.len() != usize::from(count) * 4).then(|| {
            error(
                attribute.offset,
                ClassProblem::InvalidMethodParametersLength,
            )
        });
        Ok(Some(ParameterTable {
            rows: &rows[..rows.len().min(usize::from(count))],
            issue,
        }))
    }

    /// A class can contain these attributes only if its pool contains one of
    /// their original UTF-8 name constants. Queries use this as a prefilter.
    pub fn may_have_annotations(&self) -> bool {
        self.pool
            .contains_utf8(|bytes| AttributeKind::of(bytes).is_some())
    }

    /// Returns established annotations and local issues. Unavailable indices
    /// leave their checked siblings readable. An unknown layout ends only the
    /// current bounded attribute, retaining the independently checked prefix.
    pub fn annotation_facts(
        &self,
        member: Option<Member<'a>>,
    ) -> Result<DeclarationAnnotations<'a>, ClassError> {
        let attributes = match member {
            Some(member) => {
                member.verify(self)?;
                member.attributes()
            }
            None => self.attributes()?,
        };
        let mut decoder = Decoder {
            class: self,
            result: DeclarationAnnotations::default(),
        };
        let mut seen = [false; 7];
        for attribute in attributes {
            let attribute = match attribute {
                Ok(attribute) => attribute,
                Err(error) => {
                    decoder.issue(error)?;
                    break;
                }
            };
            let name = match self
                .pool
                .utf8_bytes(attribute.name_index, attribute.offset - 6)
            {
                Ok((name, _)) => name,
                Err(error) => {
                    decoder.issue(error)?;
                    continue;
                }
            };
            let Some(kind) = AttributeKind::of(name) else {
                continue;
            };
            let duplicate = std::mem::replace(&mut seen[kind as usize], true);
            if duplicate {
                decoder.issue(error(attribute.offset, ClassProblem::DuplicateAttribute))?;
            }
            let issue_count = decoder.result.issues.len();
            let default_index = decoder.result.defaults.len();
            let complete = match decoder.read(member, kind, attribute) {
                Ok(complete) => complete,
                Err(error) => {
                    decoder.issue(error)?;
                    false
                }
            };
            if matches!(kind, AttributeKind::Default) {
                // Checked prefixes remain available to raw metadata queries.
                // Source may only consume one complete, unambiguous value.
                decoder.result.source_default = (!duplicate
                    && complete
                    && decoder.result.issues.len() == issue_count
                    && decoder.result.defaults.len() == default_index + 1)
                    .then_some(default_index);
            }
        }
        Ok(decoder.result)
    }
}

#[repr(usize)]
#[derive(Clone, Copy)]
enum AttributeKind {
    Visible,
    Invisible,
    VisibleParameters,
    InvisibleParameters,
    VisibleTypes,
    InvisibleTypes,
    Default,
}

impl AttributeKind {
    fn of(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"RuntimeVisibleAnnotations" => Self::Visible,
            b"RuntimeInvisibleAnnotations" => Self::Invisible,
            b"RuntimeVisibleParameterAnnotations" => Self::VisibleParameters,
            b"RuntimeInvisibleParameterAnnotations" => Self::InvisibleParameters,
            b"RuntimeVisibleTypeAnnotations" => Self::VisibleTypes,
            b"RuntimeInvisibleTypeAnnotations" => Self::InvisibleTypes,
            b"AnnotationDefault" => Self::Default,
            _ => return None,
        })
    }

    fn visibility(self) -> Visibility {
        match self {
            Self::Invisible | Self::InvisibleParameters | Self::InvisibleTypes => {
                Visibility::RuntimeInvisible
            }
            _ => Visibility::RuntimeVisible,
        }
    }
}

struct Decoder<'a, 'class> {
    class: &'class ClassFile<'a>,
    result: DeclarationAnnotations<'a>,
}

impl<'a> Decoder<'a, '_> {
    fn read(
        &mut self,
        member: Option<Member<'a>>,
        kind: AttributeKind,
        attribute: Attribute<'a>,
    ) -> Result<bool, ClassError> {
        let mut reader = Reader::new(attribute.payload, attribute.offset);
        let visibility = kind.visibility();
        let complete = match kind {
            AttributeKind::VisibleParameters | AttributeKind::InvisibleParameters => {
                let count = reader.u8()?;
                let alignment = self.parameter_alignment(member, count, attribute.offset)?;
                let mut complete = true;
                for encoded_index in 0..count {
                    let count = reader.u16()?;
                    let target = Target::Parameter {
                        encoded_index,
                        descriptor_index: alignment.get(usize::from(encoded_index)).copied(),
                    };
                    if !self
                        .read_annotations(&mut reader, count, visibility, |_| Ok((target, None)))?
                    {
                        complete = false;
                        break;
                    }
                }
                complete
            }
            AttributeKind::VisibleTypes | AttributeKind::InvisibleTypes => {
                let count = reader.u16()?;
                self.read_annotations(&mut reader, count, visibility, |reader| {
                    type_target(reader, member)
                })?
            }
            AttributeKind::Default => {
                let (value, complete) = self.value(&mut reader, visibility, 0)?;
                push(&mut self.result.defaults, value, attribute.offset)?;
                complete
            }
            _ => {
                let count = reader.u16()?;
                self.read_annotations(&mut reader, count, visibility, |_| {
                    Ok((Target::Declaration, None))
                })?
            }
        };
        if complete && !reader.rest.is_empty() {
            return Err(error(reader.at, ClassProblem::TrailingBytes));
        }
        Ok(complete)
    }

    fn read_annotations(
        &mut self,
        reader: &mut Reader<'a>,
        count: u16,
        visibility: Visibility,
        target: impl Fn(&mut Reader<'a>) -> Result<(Target<'a>, Option<ClassError>), ClassError>,
    ) -> Result<bool, ClassError> {
        for _ in 0..count {
            let (target, issue) = target(reader)?;
            if let Some(issue) = issue {
                self.issue(issue)?;
            }
            let (annotation, complete) = self.annotation(reader, visibility, 0)?;
            let at = annotation.offset;
            push(
                &mut self.result.annotations,
                FoundAnnotation { target, annotation },
                at,
            )?;
            if !complete {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn parameter_alignment(
        &mut self,
        member: Option<Member<'a>>,
        count: u8,
        offset: usize,
    ) -> Result<Vec<u8>, ClassError> {
        let Some(member) = member.filter(|member| member.kind() == MemberKind::Method) else {
            self.issue(error(offset, ClassProblem::InvalidAnnotationValue))?;
            return Ok(Vec::new());
        };
        let descriptor = match self.keep(member.descriptor(self.class))? {
            Some(descriptor) => descriptor,
            None => return Ok(Vec::new()),
        };
        let expected = descriptor
            .parameter_count()
            .ok_or_else(|| error(descriptor.offset(), ClassProblem::InvalidDescriptor))?;
        let mut alignment = Vec::new();
        alignment
            .try_reserve_exact(expected)
            .map_err(|_| error(offset, ClassProblem::Allocation))?;
        if usize::from(count) == expected {
            alignment.extend(0..count);
            return Ok(alignment);
        }
        if usize::from(count) < expected {
            let table = self.class.method_parameters(member, expected);
            if let Some(Some(table)) = self.keep(table)? {
                if let Some(error) = table.issue() {
                    self.issue(error)?;
                } else {
                    for (index, entry) in table.entries().enumerate() {
                        // Only names and flags supplied by the method can
                        // establish which omitted slots are not source parameters.
                        if entry.access_flags & (0x1000 | 0x8000) == 0 {
                            alignment.push(index as u8);
                        }
                    }
                    if alignment.len() == usize::from(count) {
                        return Ok(alignment);
                    }
                }
            }
        }
        self.issue(error(
            offset,
            ClassProblem::UnalignedAnnotationParameters {
                encoded: count,
                descriptor: expected,
            },
        ))?;
        Ok(Vec::new())
    }

    fn annotation(
        &mut self,
        reader: &mut Reader<'a>,
        visibility: Visibility,
        depth: usize,
    ) -> Result<(Annotation<'a>, bool), ClassError> {
        if depth > MAX_ANNOTATION_NESTING {
            return Err(error(reader.at, ClassProblem::AnnotationNesting));
        }
        let offset = reader.at;
        let index = reader.u16()?;
        let descriptor = self.keep(self.object_descriptor(index, offset))?;
        let mut annotation = Annotation {
            visibility,
            descriptor,
            offset,
            elements: Vec::new(),
        };
        let count = match self.keep(reader.u16())? {
            Some(count) => count,
            None => return Ok((annotation, false)),
        };
        for _ in 0..count {
            let at = reader.at;
            let index = match self.keep(reader.u16())? {
                Some(index) => index,
                None => return Ok((annotation, false)),
            };
            let name = self.keep(self.element_name(index, at))?;
            let parsed = self.value(reader, visibility, depth + 1);
            let (value, complete) = match self.keep(parsed)? {
                Some(parsed) => parsed,
                None => return Ok((annotation, false)),
            };
            push(&mut annotation.elements, Element { name, value }, at)?;
            if !complete {
                return Ok((annotation, false));
            }
        }
        Ok((annotation, true))
    }

    fn value(
        &mut self,
        reader: &mut Reader<'a>,
        visibility: Visibility,
        depth: usize,
    ) -> Result<(ElementValue<'a>, bool), ClassError> {
        if depth > MAX_ANNOTATION_NESTING {
            return Err(error(reader.at, ClassProblem::AnnotationNesting));
        }
        let offset = reader.at;
        let tag = reader.u8()?;
        let (value, complete) = match tag {
            b'@' => {
                let (annotation, complete) = self.annotation(reader, visibility, depth + 1)?;
                (Value::Annotation(Box::new(annotation)), complete)
            }
            b'[' => {
                let count = reader.u16()?;
                let mut values = Vec::new();
                let mut complete = true;
                for _ in 0..count {
                    let current = self.value(reader, visibility, depth + 1);
                    let Some((value, readable)) = self.keep(current)? else {
                        complete = false;
                        break;
                    };
                    let at = value.offset;
                    push(&mut values, value, at)?;
                    if !readable {
                        complete = false;
                        break;
                    }
                }
                (Value::Array { values, complete }, complete)
            }
            b'e' => {
                let at = reader.at;
                let index = reader.u16()?;
                let descriptor = self.keep(self.object_descriptor(index, at))?;
                let at = reader.at;
                let index = reader.u16()?;
                let name = self.keep(
                    self.class
                        .pool
                        .utf8(index, at)
                        .and_then(|name| name.member_name(false)),
                )?;
                (Value::Enum { descriptor, name }, true)
            }
            b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' | b'c' => {
                let at = reader.at;
                let index = reader.u16()?;
                let value = self
                    .keep(self.constant(tag, index, at))?
                    .unwrap_or(Value::Unavailable);
                (value, true)
            }
            _ => return Err(error(offset, ClassProblem::UnknownAnnotationTag(tag))),
        };
        Ok((ElementValue { offset, value }, complete))
    }

    fn constant(&self, tag: u8, index: u16, at: usize) -> Result<Value<'a>, ClassError> {
        let bad = || error(at, ClassProblem::InvalidAnnotationValue);
        match tag {
            b's' => return self.class.pool.utf8(index, at).map(Value::String),
            b'c' => {
                let descriptor = self.class.pool.utf8(index, at)?;
                return if descriptor.bytes() == b"V" {
                    Ok(Value::Class(descriptor))
                } else {
                    descriptor.descriptor(false).map(Value::Class)
                };
            }
            _ => {}
        }
        Ok(match (tag, self.class.pool.constant(index, at)?) {
            (b'B', Constant::Integer(value)) => {
                Value::Byte(i8::try_from(value).map_err(|_| bad())?)
            }
            (b'C', Constant::Integer(value)) => {
                Value::Char(u16::try_from(value).map_err(|_| bad())?)
            }
            (b'D', Constant::Double(value)) => Value::Double(value),
            (b'F', Constant::Float(value)) => Value::Float(value),
            (b'I', Constant::Integer(value)) => Value::Int(value),
            (b'J', Constant::Long(value)) => Value::Long(value),
            (b'S', Constant::Integer(value)) => {
                Value::Short(i16::try_from(value).map_err(|_| bad())?)
            }
            (b'Z', Constant::Integer(value @ (0 | 1))) => Value::Boolean(value != 0),
            _ => return Err(bad()),
        })
    }

    fn object_descriptor(&self, index: u16, at: usize) -> Result<JvmString<'a>, ClassError> {
        let descriptor = self.class.pool.utf8(index, at)?.descriptor(false)?;
        if descriptor.bytes().starts_with(b"L") {
            Ok(descriptor)
        } else {
            Err(error(at, ClassProblem::InvalidDescriptor))
        }
    }

    fn element_name(&self, index: u16, at: usize) -> Result<JvmString<'a>, ClassError> {
        let name = self.class.pool.utf8(index, at)?.member_name(true)?;
        if name.bytes().starts_with(b"<") {
            Err(error(at, ClassProblem::InvalidName))
        } else {
            Ok(name)
        }
    }

    fn keep<T>(&mut self, value: Result<T, ClassError>) -> Result<Option<T>, ClassError> {
        match value {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                self.issue(error)?;
                Ok(None)
            }
        }
    }

    fn issue(&mut self, error: ClassError) -> Result<(), ClassError> {
        push(&mut self.result.issues, error, error.offset)
    }
}

fn type_target<'a>(
    reader: &mut Reader<'a>,
    member: Option<Member<'a>>,
) -> Result<(Target<'a>, Option<ClassError>), ClassError> {
    let at = reader.at;
    let target_type = reader.u8()?;
    let original = reader.rest;
    let length = match target_type {
        0x00 | 0x01 | 0x16 => 1,
        0x10..=0x12 | 0x17 | 0x42..=0x46 => 2,
        0x13..=0x15 => 0,
        0x40 | 0x41 => usize::from(reader.u16()?) * 6,
        0x47..=0x4b => 3,
        _ => {
            return Err(error(
                at,
                ClassProblem::InvalidAnnotationTarget(target_type),
            ))
        }
    };
    reader.take(length)?;
    let target_info = &original[..original.len() - reader.rest.len()];
    let path_at = reader.at;
    let path_length = usize::from(reader.u8()?);
    let type_path = reader.take(path_length * 2)?;
    let valid_scope = match member.map(Member::kind) {
        None => matches!(target_type, 0x00 | 0x10 | 0x11),
        Some(MemberKind::Field) => target_type == 0x13,
        Some(MemberKind::Method) => matches!(target_type, 0x01 | 0x12 | 0x14..=0x17),
    };
    let invalid_path = type_path
        .as_chunks::<2>()
        .0
        .iter()
        .any(|&[kind, argument]| kind > 3 || (kind != 3 && argument != 0));
    let issue = if !valid_scope {
        Some(error(
            at,
            ClassProblem::InvalidAnnotationTarget(target_type),
        ))
    } else if invalid_path {
        Some(error(path_at, ClassProblem::InvalidAnnotationPath))
    } else {
        None
    };
    Ok((
        Target::TypeUse {
            target_type,
            target_info,
            type_path,
        },
        issue,
    ))
}

fn push<T>(values: &mut Vec<T>, value: T, at: usize) -> Result<(), ClassError> {
    values
        .try_reserve(1)
        .map_err(|_| error(at, ClassProblem::Allocation))?;
    values.push(value);
    Ok(())
}
