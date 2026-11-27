//! Borrowed facts from one JVM ClassFile. Parsing establishes the header,
//! constant-pool bounds and declaration directory boundaries on demand.

pub mod annotations;
pub mod bytecode;
pub mod constants;
mod members;
mod pool;
mod record;
mod text;

use crate::FailureKind;
use thiserror::Error;

pub use members::{Attribute, Attributes, BootstrapMethod, BootstrapMethods, Member, Members};
pub use pool::{Constant, MemberReference, NameAndType, ReferenceKind};
pub use record::{RecordComponent, RecordComponents};
pub use text::{JvmString, Utf16Units};

use members::MemberKind;
use pool::{Pool, PoolOffsets};

const MAGIC: [u8; 4] = [0xca, 0xfe, 0xba, 0xbe];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassProblem {
    Truncated,
    InvalidMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    InvalidConstantCount,
    UnknownConstantTag(u8),
    InvalidConstantIndex(u16),
    UnexpectedConstantKind { index: u16, tag: u8 },
    InvalidModifiedUtf8,
    InvalidName,
    InvalidDescriptor,
    DuplicateAttribute,
    UnknownAnnotationTag(u8),
    InvalidAnnotationValue,
    InvalidFieldConstant,
    InvalidAnnotationTarget(u8),
    InvalidAnnotationPath,
    AnnotationNesting,
    UnalignedAnnotationParameters { encoded: u8, descriptor: usize },
    InvalidMethodParametersCount { encoded: u8, descriptor: usize },
    InvalidMethodParametersLength,
    InvalidAttributeLength,
    InvalidBootstrapIndex(u16),
    TrailingBytes,
    ForeignMember,
    PlatformCapacity,
    Allocation,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("JVM class file at byte {offset}: {problem:?}")]
pub struct ClassError {
    pub offset: usize,
    pub problem: ClassProblem,
}

impl ClassError {
    pub fn kind(self) -> FailureKind {
        match self.problem {
            ClassProblem::UnsupportedVersion { .. } | ClassProblem::UnknownConstantTag(_) => {
                FailureKind::Unsupported
            }
            ClassProblem::PlatformCapacity
            | ClassProblem::Allocation
            | ClassProblem::AnnotationNesting => FailureKind::ResourceExhausted,
            ClassProblem::ForeignMember => FailureKind::InternalInvariant,
            _ => FailureKind::Malformed,
        }
    }
}

fn error(offset: usize, problem: ClassProblem) -> ClassError {
    ClassError { offset, problem }
}

/// Per-worker storage for original constant-pool offsets.
#[derive(Debug, Default)]
pub struct ClassScratch {
    offsets: Vec<u32>,
}

/// One physical class file, independent of its archive name and entry position.
///
/// Header reads establish the extent of each constant-pool slot. Name, member
/// and attribute queries then validate the original facts they consume.
#[derive(Debug)]
pub struct ClassFile<'a> {
    bytes: &'a [u8],
    pool: Pool<'a>,
    major: u16,
    minor: u16,
    declaration_offset: usize,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces_offset: usize,
}

impl<'a> ClassFile<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ClassError> {
        let mut offsets = Vec::new();
        let (reader, major, minor) = Self::read_header(bytes, &mut offsets)?;
        Self::after_pool(bytes, reader, major, minor, PoolOffsets::Owned(offsets))
    }

    pub fn parse_in(bytes: &'a [u8], scratch: &'a mut ClassScratch) -> Result<Self, ClassError> {
        let (reader, major, minor) = Self::read_header(bytes, &mut scratch.offsets)?;
        Self::after_pool(
            bytes,
            reader,
            major,
            minor,
            PoolOffsets::Borrowed(&scratch.offsets),
        )
    }

    fn read_header(
        bytes: &'a [u8],
        offsets: &mut Vec<u32>,
    ) -> Result<(Reader<'a>, u16, u16), ClassError> {
        let mut reader = Reader::new(bytes, 0);
        if reader.array::<4>()? != MAGIC {
            return Err(error(0, ClassProblem::InvalidMagic));
        }
        let minor = reader.u16()?;
        let major = reader.u16()?;
        if major < 45 || (major >= 56 && !matches!(minor, 0 | u16::MAX)) {
            return Err(error(4, ClassProblem::UnsupportedVersion { major, minor }));
        }
        Pool::scan(&mut reader, offsets)?;
        Ok((reader, major, minor))
    }

    fn after_pool(
        bytes: &'a [u8],
        mut reader: Reader<'a>,
        major: u16,
        minor: u16,
        offsets: PoolOffsets<'a>,
    ) -> Result<Self, ClassError> {
        let declaration_offset = reader.at;
        let access_flags = reader.u16()?;
        let this_class = reader.u16()?;
        let super_class = reader.u16()?;
        Ok(Self {
            bytes,
            pool: Pool { bytes, offsets },
            major,
            minor,
            declaration_offset,
            access_flags,
            this_class,
            super_class,
            interfaces_offset: reader.at,
        })
    }

    pub fn encoded_len(&self) -> usize {
        self.bytes.len()
    }

    pub fn major_version(&self) -> u16 {
        self.major
    }

    pub fn minor_version(&self) -> u16 {
        self.minor
    }

    pub fn access_flags(&self) -> u16 {
        self.access_flags
    }

    /// The original internal binary name, for example `java/lang/Object`.
    pub fn name(&self) -> Result<JvmString<'a>, ClassError> {
        self.pool
            .binary_name(self.this_class, self.declaration_offset + 2)
    }

    pub fn super_name(&self) -> Result<Option<JvmString<'a>>, ClassError> {
        (self.super_class != 0)
            .then(|| {
                self.pool
                    .binary_name(self.super_class, self.declaration_offset + 4)
            })
            .transpose()
    }

    fn interface_bytes(&self) -> Result<(&'a [u8], Reader<'a>), ClassError> {
        let mut reader = Reader::new(
            &self.bytes[self.interfaces_offset..],
            self.interfaces_offset,
        );
        let count = usize::from(reader.u16()?);
        let interfaces = reader.take(count * 2)?;
        Ok((interfaces, reader))
    }

    pub fn interfaces(&self) -> impl Iterator<Item = Result<JvmString<'a>, ClassError>> + '_ {
        let first = self.interfaces_offset + 2;
        let (bytes, issue) = match self.interface_bytes() {
            Ok((bytes, _)) => (bytes, None),
            Err(issue) => (&[][..], Some(Err(issue))),
        };
        issue
            .into_iter()
            .chain(bytes.as_chunks::<2>().0.iter().copied().enumerate().map(
                move |(index, pair)| {
                    self.pool
                        .binary_name(u16::from_be_bytes(pair), first + index * 2)
                },
            ))
    }

    pub fn fields(&self) -> Result<Members<'a>, ClassError> {
        let (_, reader) = self.interface_bytes()?;
        Members::new(self.bytes, reader, MemberKind::Field)
    }

    pub fn methods(&self) -> Result<Members<'a>, ClassError> {
        let reader = self.fields()?.skip_remaining()?;
        Members::new(self.bytes, reader, MemberKind::Method)
    }

    pub fn attributes(&self) -> Result<Attributes<'a>, ClassError> {
        let reader = self.methods()?.skip_remaining()?;
        Attributes::class(reader)
    }

    pub fn constant_pool_count(&self) -> u16 {
        // The original constant_pool_count is u16, including the reserved zero slot.
        self.pool.offsets.as_slice().len() as u16
    }

    /// Tags and original pool indices, skipping unusable double-width slots.
    pub fn constant_tags(&self) -> impl Iterator<Item = (u16, u8)> + '_ {
        self.pool.tags()
    }

    pub fn constant(&self, index: u16) -> Result<Constant<'a>, ClassError> {
        self.pool.constant(index, 8)
    }

    pub fn utf8(&self, index: u16) -> Result<JvmString<'a>, ClassError> {
        self.pool.utf8(index, 8)
    }

    /// Reference class names can also identify array types.
    pub fn class_name(&self, index: u16) -> Result<JvmString<'a>, ClassError> {
        self.pool.class_name(index, 8)
    }

    pub fn string_constant(&self, index: u16) -> Result<JvmString<'a>, ClassError> {
        self.pool.string_constant(index, 8)
    }

    pub fn strings(&self) -> impl Iterator<Item = Result<(u16, JvmString<'a>), ClassError>> + '_ {
        self.pool.strings()
    }

    pub fn name_and_type(&self, index: u16) -> Result<NameAndType<'a>, ClassError> {
        self.pool.name_and_type(index, 8)
    }

    pub fn member_reference(&self, index: u16) -> Result<MemberReference<'a>, ClassError> {
        self.pool.member_reference(index)
    }

    /// The original generic signature, before source erasure or scope inference.
    pub fn signature(
        &self,
        attributes: Attributes<'a>,
    ) -> Result<Option<JvmString<'a>>, ClassError> {
        self.find_attribute(attributes, b"Signature")?
            .map(|attribute| {
                let index = <[u8; 2]>::try_from(attribute.payload)
                    .map_err(|_| error(attribute.offset, ClassProblem::InvalidAttributeLength))?;
                self.utf8(u16::from_be_bytes(index))
            })
            .transpose()
    }

    /// The method's original Exceptions entries in order. A bad entry does
    /// not obscure the names of the other declared exceptions.
    pub fn declared_exceptions<'c>(
        &'c self,
        member: Member<'a>,
    ) -> Result<impl Iterator<Item = Result<JvmString<'a>, ClassError>> + 'c, ClassError> {
        member.verify(self)?;
        let rows = self
            .find_attribute(member.attributes(), b"Exceptions")?
            .map(|attribute| {
                let invalid = || error(attribute.offset, ClassProblem::InvalidAttributeLength);
                let (count, rest) = attribute
                    .payload
                    .split_first_chunk::<2>()
                    .ok_or_else(invalid)?;
                let (rows, tail) = rest.as_chunks::<2>();
                if !tail.is_empty() || rows.len() != usize::from(u16::from_be_bytes(*count)) {
                    return Err(invalid());
                }
                Ok(rows)
            })
            .transpose()?
            .unwrap_or_default();
        Ok(rows
            .iter()
            .map(|index| self.class_name(u16::from_be_bytes(*index))?.binary_name()))
    }

    pub fn code_attribute(&self, member: Member<'a>) -> Result<Option<Attribute<'a>>, ClassError> {
        member.verify(self)?;
        if member.kind() != MemberKind::Method {
            return Ok(None);
        }
        self.find_attribute(member.attributes(), b"Code")
    }

    /// Returns the original record component sequence, when it is declared.
    pub fn record_components(&self) -> Result<Option<RecordComponents<'a, '_>>, ClassError> {
        self.find_attribute(self.attributes()?, b"Record")?
            .map(|attribute| RecordComponents::new(self, attribute))
            .transpose()
    }

    pub fn bootstrap_methods(&self) -> Result<Option<BootstrapMethods<'a>>, ClassError> {
        self.find_attribute(self.attributes()?, b"BootstrapMethods")?
            .map(BootstrapMethods::new)
            .transpose()
    }

    pub fn bootstrap_method(&self, index: u16) -> Result<BootstrapMethod<'a>, ClassError> {
        let mut methods = self.bootstrap_methods()?.ok_or_else(|| {
            error(
                self.declaration_offset,
                ClassProblem::InvalidBootstrapIndex(index),
            )
        })?;
        let offset = methods.offset();
        // Iterator::nth would discard an error establishing an earlier row boundary.
        for method in &mut methods {
            let method = method?;
            if method.ordinal == index {
                return Ok(method);
            }
        }
        Err(error(offset, ClassProblem::InvalidBootstrapIndex(index)))
    }

    pub(crate) fn find_attribute(
        &self,
        attributes: Attributes<'a>,
        name: &[u8],
    ) -> Result<Option<Attribute<'a>>, ClassError> {
        let mut found = None;
        let mut first_unavailable = None;
        for attribute in attributes {
            let attribute = attribute?;
            match self
                .pool
                .utf8_bytes(attribute.name_index, attribute.offset - 6)
            {
                Ok((bytes, _)) if bytes == name => {
                    if found.replace(attribute).is_some() {
                        return Err(error(attribute.offset, ClassProblem::DuplicateAttribute));
                    }
                }
                Ok(_) => {}
                Err(issue) => {
                    first_unavailable.get_or_insert(issue);
                }
            }
        }
        if found.is_none() {
            if let Some(issue) = first_unavailable {
                return Err(issue);
            }
        }
        Ok(found)
    }
}

#[derive(Clone, Copy, Debug)]
struct Reader<'a> {
    rest: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(rest: &'a [u8], at: usize) -> Self {
        Self { rest, at }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ClassError> {
        let Some((head, rest)) = self.rest.split_at_checked(count) else {
            return Err(error(self.at, ClassProblem::Truncated));
        };
        // Every reader is placed within an original, already bounded slice.
        self.at += count;
        self.rest = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ClassError> {
        let Some((&head, rest)) = self.rest.split_first_chunk() else {
            return Err(error(self.at, ClassProblem::Truncated));
        };
        self.at += N;
        self.rest = rest;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, ClassError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, ClassError> {
        self.array().map(u16::from_be_bytes)
    }

    fn u32(&mut self) -> Result<u32, ClassError> {
        self.array().map(u32::from_be_bytes)
    }
}

pub(crate) mod lower;
