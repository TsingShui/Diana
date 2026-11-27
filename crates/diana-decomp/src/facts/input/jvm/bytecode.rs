//! Checked JVM bytecode and its original constant-pool operands.
//!
//! An invalid operand does not erase the instruction's known width. An
//! undecodable width leaves a gap through the end of this method's Code.

use std::iter::FusedIterator;
use std::ops::Range;

use super::{ClassError, ClassFile, Constant, JvmString, MemberReference, ReferenceKind};

const CODE_HEADER_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeProblem {
    TruncatedHeader,
    InvalidCodeLength(u32),
    TruncatedCode,
    TruncatedExceptionTable,
    TruncatedAttributes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("malformed JVM Code attribute at class-file byte {offset}: {problem:?}")]
pub struct CodeError {
    pub offset: usize,
    pub problem: CodeProblem,
}

/// The independently readable instruction bytes of one Code attribute.
#[derive(Clone, Copy, Debug)]
pub struct Code<'a> {
    bytes: &'a [u8],
    tail: &'a [u8],
    declared_length: u16,
    offset: usize,
    pub max_stack: u16,
    pub max_locals: u16,
}

impl<'a> Code<'a> {
    pub fn parse(attribute: &'a [u8]) -> Result<Self, CodeError> {
        Self::parse_at(attribute, 0)
    }

    /// `offset` is the position of the attribute payload in the class file.
    pub fn parse_at(attribute: &'a [u8], offset: usize) -> Result<Self, CodeError> {
        let header = attribute
            .first_chunk::<CODE_HEADER_SIZE>()
            .ok_or(CodeError {
                offset,
                problem: CodeProblem::TruncatedHeader,
            })?;
        let length = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
        let length = u16::try_from(length)
            .ok()
            .filter(|length| *length > 0)
            .ok_or(CodeError {
                offset: offset.saturating_add(4),
                problem: CodeProblem::InvalidCodeLength(length),
            })?;
        let offset = offset.saturating_add(CODE_HEADER_SIZE);
        let payload = &attribute[CODE_HEADER_SIZE..];
        let (bytes, tail) = payload.split_at(usize::from(length).min(payload.len()));
        Ok(Self {
            bytes,
            tail,
            declared_length: length,
            offset,
            max_stack: u16::from_be_bytes([header[0], header[1]]),
            max_locals: u16::from_be_bytes([header[2], header[3]]),
        })
    }

    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    pub fn class_file_offset(self) -> usize {
        self.offset
    }

    /// Bytecode interval that the Code envelope declares beyond the payload.
    pub fn missing_bytes(self) -> Option<Range<u16>> {
        let available = self.bytes.len() as u16;
        (available < self.declared_length).then_some(available..self.declared_length)
    }

    pub fn instructions(self) -> Instructions<'a> {
        Instructions {
            bytes: self.bytes,
            declared_length: self.declared_length,
            position: 0,
        }
    }

    /// A malformed handler table does not prevent instruction inspection.
    pub fn exception_handlers(self) -> Result<ExceptionHandlers<'a>, CodeError> {
        let offset = self.offset.saturating_add(self.bytes.len());
        if self.missing_bytes().is_some() {
            return Err(CodeError {
                offset,
                problem: CodeProblem::TruncatedCode,
            });
        }
        let error = CodeError {
            offset,
            problem: CodeProblem::TruncatedExceptionTable,
        };
        let count = usize::from(read_u16(self.tail, 0).ok_or(error)?);
        let payload = &self.tail[2..];
        let (table, attributes, issue) = match count
            .checked_mul(8)
            .and_then(|length| payload.split_at_checked(length))
        {
            Some((table, attributes)) => (table, attributes, None),
            None => (
                payload,
                &[][..],
                Some(CodeError {
                    offset: offset.saturating_add(2 + payload.len() / 8 * 8),
                    ..error
                }),
            ),
        };
        Ok(ExceptionHandlers {
            rows: table.as_chunks::<8>().0.iter(),
            attributes,
            issue,
        })
    }

    /// Checked envelopes of the attributes nested within this Code.
    pub(crate) fn attributes(self) -> Result<super::Attributes<'a>, CodeError> {
        let bytes = self.attributes_bytes()?;
        let offset = self
            .offset
            .saturating_add(self.bytes.len() + self.tail.len() - bytes.len());
        super::Attributes::class(super::Reader::new(bytes, offset)).map_err(|error| CodeError {
            offset: error.offset,
            problem: CodeProblem::TruncatedAttributes,
        })
    }

    /// The nested attribute table, including its original count word.
    pub fn attributes_bytes(self) -> Result<&'a [u8], CodeError> {
        let handlers = self.exception_handlers()?;
        if let Some(error) = handlers.issue() {
            return Err(error);
        }
        let attributes = handlers.attributes;
        if attributes.len() < 2 {
            return Err(CodeError {
                offset: self
                    .offset
                    .saturating_add(self.bytes.len() + self.tail.len() - attributes.len()),
                problem: CodeProblem::TruncatedAttributes,
            });
        }
        Ok(attributes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExceptionHandler {
    pub start: u16,
    pub end: u16,
    pub target: u16,
    /// Zero in the input represents a catch-all handler.
    pub catch_type: Option<u16>,
}

impl ExceptionHandler {
    pub fn range_problem(self, code: Code<'_>) -> Option<OperandProblem> {
        let len = code.declared_length;
        if self.start >= self.end || self.end > len || self.target >= len {
            return Some(OperandProblem::ExceptionHandlerRange);
        }
        None
    }
}

pub struct ExceptionHandlers<'a> {
    rows: std::slice::Iter<'a, [u8; 8]>,
    attributes: &'a [u8],
    issue: Option<CodeError>,
}

impl ExceptionHandlers<'_> {
    /// The first missing row when only an ordered prefix is readable.
    pub fn issue(&self) -> Option<CodeError> {
        self.issue
    }
}

impl Iterator for ExceptionHandlers<'_> {
    type Item = ExceptionHandler;

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.rows.next()?;
        Some(ExceptionHandler {
            start: u16::from_be_bytes([row[0], row[1]]),
            end: u16::from_be_bytes([row[2], row[3]]),
            target: u16::from_be_bytes([row[4], row[5]]),
            catch_type: std::num::NonZeroU16::new(u16::from_be_bytes([row[6], row[7]]))
                .map(std::num::NonZeroU16::get),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rows.size_hint()
    }
}

impl ExactSizeIterator for ExceptionHandlers<'_> {}
impl FusedIterator for ExceptionHandlers<'_> {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeProblem {
    UndefinedOpcode(u8),
    TruncatedInstruction,
    InvalidWideOpcode(u8),
    InvalidSwitchCount,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("JVM bytecode at {range:?}: {problem:?}")]
pub struct DecodeError {
    /// The byte interval whose instruction layout could not be established.
    pub range: Range<u16>,
    pub problem: DecodeProblem,
}

impl DecodeError {
    pub fn kind(&self) -> crate::FailureKind {
        match self.problem {
            DecodeProblem::UndefinedOpcode(_) => crate::FailureKind::Unsupported,
            _ => crate::FailureKind::Malformed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperandProblem {
    InvokeInterfaceCount,
    InvokeReservedBytes,
    NewArrayType,
    MultiArrayDimensions,
    BranchOutsideCode,
    SwitchKeyOrder,
    ExceptionHandlerRange,
}

pub struct Instructions<'a> {
    bytes: &'a [u8],
    declared_length: u16,
    position: usize,
}

impl<'a> Iterator for Instructions<'a> {
    type Item = Result<Instruction<'a>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.position;
        let tail = self.bytes.get(offset..)?;
        let opcode = *tail.first()?;
        let width = match WIDTHS[usize::from(opcode)] {
            0 => variable_width(opcode, offset, tail),
            fixed => Ok(usize::from(fixed)),
        };
        let entry =
            width.and_then(|width| tail.get(..width).ok_or(DecodeProblem::TruncatedInstruction));
        Some(match entry {
            Ok(bytes) => {
                self.position += bytes.len();
                Ok(Instruction {
                    offset: offset as u16,
                    code_length: self.declared_length,
                    bytes,
                })
            }
            Err(problem) => {
                self.position = self.bytes.len();
                Err(DecodeError {
                    range: offset as u16..self.position as u16,
                    problem,
                })
            }
        })
    }
}

impl FusedIterator for Instructions<'_> {}

#[derive(Clone, Copy, Debug)]
pub struct Instruction<'a> {
    offset: u16,
    code_length: u16,
    bytes: &'a [u8],
}

impl<'a> Instruction<'a> {
    pub fn offset(self) -> u16 {
        self.offset
    }

    pub fn opcode(self) -> u8 {
        self.bytes[0]
    }

    pub fn mnemonic(self) -> &'static str {
        opcode_mnemonic(self.opcode())
    }

    pub fn may_throw(self) -> bool {
        matches!(self.opcode(), 0x12..=0x14 | 0x2e..=0x35 | 0x4f..=0x56
            | 0x6c | 0x6d | 0x70 | 0x71 | 0xb2..=0xc3 | 0xc5)
    }

    pub fn modified_mnemonic(self) -> Option<&'static str> {
        self.wide_opcode()
            .map(|opcode| MNEMONICS[usize::from(opcode)])
    }

    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// The modified opcode for a `wide` instruction.
    pub fn wide_opcode(self) -> Option<u8> {
        (self.opcode() == 0xc4).then(|| self.bytes[1])
    }

    /// The original constant-pool operand, independent of its resolution.
    pub fn constant_index(self) -> Option<u16> {
        match self.opcode() {
            0x12 => Some(u16::from(self.bytes[1])),
            0x13..=0x14 | 0xb2..=0xbb | 0xbd | 0xc0 | 0xc1 | 0xc5 => {
                Some(u16::from_be_bytes([self.bytes[1], self.bytes[2]]))
            }
            _ => None,
        }
    }

    /// Explicit non-constant-pool operands, signed where specified by JVM.
    pub fn operands(self) -> Operands<'a> {
        let opcode = self.opcode();
        match opcode {
            0x10 => Operands::Immediate(i32::from(self.bytes[1] as i8)),
            0x11 => Operands::Immediate(i32::from(i16::from_be_bytes([
                self.bytes[1],
                self.bytes[2],
            ]))),
            0x15..=0x19 | 0x36..=0x3a | 0xa9 => Operands::Local(u16::from(self.bytes[1])),
            0x84 => Operands::Increment {
                local: u16::from(self.bytes[1]),
                value: i16::from(self.bytes[2] as i8),
            },
            0xbc => Operands::ArrayType(self.bytes[1]),
            0xc4 => {
                let local = u16::from_be_bytes([self.bytes[2], self.bytes[3]]);
                if self.bytes[1] == 0x84 {
                    Operands::Increment {
                        local,
                        value: i16::from_be_bytes([self.bytes[4], self.bytes[5]]),
                    }
                } else {
                    Operands::Local(local)
                }
            }
            _ => {
                if let Some(relative) = self.relative_branch() {
                    Operands::Branch(relative)
                } else if let Some(switch) = self.switch() {
                    Operands::Switch(switch)
                } else if let Some(index) = self.constant_index() {
                    let extra = match opcode {
                        0xb9 => Some(ConstantOperand::InterfaceSlots(self.bytes[3])),
                        0xc5 => Some(ConstantOperand::Dimensions(self.bytes[3])),
                        _ => None,
                    };
                    Operands::Constant { index, extra }
                } else {
                    Operands::None
                }
            }
        }
    }

    pub fn relative_branch(self) -> Option<i32> {
        match self.opcode() {
            0x99..=0xa8 | 0xc6..=0xc7 => Some(i32::from(i16::from_be_bytes([
                self.bytes[1],
                self.bytes[2],
            ]))),
            0xc8..=0xc9 => read_i32(self.bytes, 1),
            _ => None,
        }
    }

    /// Resolve an offset in the method's byte coordinate space. An enclosing
    /// control-flow producer can additionally check instruction membership.
    pub fn target(self, relative: i32) -> Result<u16, OperandProblem> {
        let absolute = i64::from(self.offset) + i64::from(relative);
        if !(0..i64::from(self.code_length)).contains(&absolute) {
            return Err(OperandProblem::BranchOutsideCode);
        }
        Ok(absolute as u16)
    }

    pub fn switch(self) -> Option<Switch<'a>> {
        let is_table = match self.opcode() {
            0xaa => true,
            0xab => false,
            _ => return None,
        };
        let header = &self.bytes[switch_header(usize::from(self.offset))..];
        let default = read_i32(header, 0)?;
        if is_table {
            Some(Switch {
                default,
                low: Some(read_i32(header, 4)?),
                entries: &header[12..],
            })
        } else {
            Some(Switch {
                default,
                low: None,
                entries: &header[8..],
            })
        }
    }

    /// Shape checks confined to this instruction. The physical bytes and
    /// constant index remain independently available after an error.
    pub fn operand_problem(self) -> Option<OperandProblem> {
        match self.opcode() {
            0xb9 if self.bytes[3] == 0 => return Some(OperandProblem::InvokeInterfaceCount),
            0xb9 if self.bytes[4] != 0 => return Some(OperandProblem::InvokeReservedBytes),
            0xba if self.bytes[3] != 0 || self.bytes[4] != 0 => {
                return Some(OperandProblem::InvokeReservedBytes);
            }
            0xbc if !(4..=11).contains(&self.bytes[1]) => {
                return Some(OperandProblem::NewArrayType);
            }
            0xc5 if self.bytes[3] == 0 => return Some(OperandProblem::MultiArrayDimensions),
            _ => {}
        }
        if let Some(relative) = self.relative_branch() {
            return self.target(relative).err();
        }
        if let Some(switch) = self.switch() {
            let mut prior = None;
            for (key, relative) in switch.cases() {
                if prior.is_some_and(|prior| prior >= key) {
                    return Some(OperandProblem::SwitchKeyOrder);
                }
                if let Err(problem) = self.target(relative) {
                    return Some(problem);
                }
                prior = Some(key);
            }
            return self.target(switch.default).err();
        }
        None
    }

    /// Resolve the operand using only the input class's original pool.
    pub fn reference(self, class: &ClassFile<'a>) -> Result<Option<Reference<'a>>, ReferenceError> {
        let Some(index) = self.constant_index() else {
            return Ok(None);
        };
        let opcode = self.opcode();
        let constant = class.constant(index)?;
        let unexpected = || ReferenceError::UnexpectedConstant { index, opcode };
        let reference = match (opcode, constant) {
            (0xb2..=0xb5, Constant::FieldRef { .. })
            | (0xb6..=0xb8, Constant::MethodRef { .. })
            | (0xb7..=0xb9, Constant::InterfaceMethodRef { .. }) => {
                Reference::Member(class.member_reference(index)?)
            }
            (0xbb | 0xbd | 0xc0 | 0xc1 | 0xc5, Constant::Class { .. }) => {
                Reference::Class(class.class_name(index)?)
            }
            (
                0xba,
                Constant::InvokeDynamic {
                    bootstrap_index,
                    name_and_type_index,
                },
            ) => dynamic_reference(class, index, true, bootstrap_index, name_and_type_index)?,
            (0x12..=0x14, Constant::Dynamic { .. }) => {
                let reference = loadable_reference(class, index, constant)?;
                if let Some(Reference::Dynamic { descriptor, .. }) = reference {
                    let wide = matches!(descriptor.bytes(), b"J" | b"D");
                    if (opcode == 0x14) != wide {
                        return Err(unexpected());
                    }
                }
                return Ok(reference);
            }
            (
                0x12..=0x13,
                Constant::Class { .. }
                | Constant::String { .. }
                | Constant::MethodHandle { .. }
                | Constant::MethodType { .. }
                | Constant::Integer(_)
                | Constant::Float(_),
            )
            | (0x14, Constant::Long(_) | Constant::Double(_)) => {
                return loadable_reference(class, index, constant);
            }
            _ => return Err(unexpected()),
        };
        Ok(Some(reference))
    }
}

/// Resolve a loadable constant supplied as an original bootstrap argument.
pub fn bootstrap_argument_reference<'a>(
    class: &ClassFile<'a>,
    index: u16,
) -> Result<Option<Reference<'a>>, ReferenceError> {
    loadable_reference(class, index, class.constant(index)?)
}

fn loadable_reference<'a>(
    class: &ClassFile<'a>,
    index: u16,
    constant: Constant<'a>,
) -> Result<Option<Reference<'a>>, ReferenceError> {
    let reference = match constant {
        Constant::Class { .. } => Reference::Class(class.class_name(index)?),
        Constant::String { .. } => Reference::String(class.string_constant(index)?),
        Constant::MethodHandle {
            kind,
            reference_index,
        } => Reference::MethodHandle {
            kind,
            reference_index,
            member: method_handle(class, kind, reference_index)?,
        },
        Constant::MethodType { descriptor_index } => {
            Reference::MethodType(class.utf8(descriptor_index)?.descriptor(true)?)
        }
        Constant::Dynamic {
            bootstrap_index,
            name_and_type_index,
        } => dynamic_reference(class, index, false, bootstrap_index, name_and_type_index)?,
        Constant::Integer(_) | Constant::Float(_) | Constant::Long(_) | Constant::Double(_) => {
            return Ok(None);
        }
        _ => return Err(ReferenceError::InvalidLoadableConstant(index)),
    };
    Ok(Some(reference))
}

fn dynamic_reference<'a>(
    class: &ClassFile<'a>,
    index: u16,
    invoke: bool,
    bootstrap_index: u16,
    name_and_type_index: u16,
) -> Result<Reference<'a>, ReferenceError> {
    let member = class.name_and_type(name_and_type_index)?;
    if member.descriptor.bytes().starts_with(b"(") != invoke {
        return Err(ReferenceError::InvalidDynamicDescriptor(index));
    }
    Ok(Reference::Dynamic {
        invoke,
        bootstrap_index,
        name: member.name,
        descriptor: member.descriptor,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum Operands<'a> {
    None,
    Immediate(i32),
    Local(u16),
    Increment {
        local: u16,
        value: i16,
    },
    ArrayType(u8),
    Branch(i32),
    Switch(Switch<'a>),
    Constant {
        index: u16,
        extra: Option<ConstantOperand>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ConstantOperand {
    InterfaceSlots(u8),
    Dimensions(u8),
}

#[derive(Debug)]
pub enum Reference<'a> {
    Class(JvmString<'a>),
    String(JvmString<'a>),
    Member(MemberReference<'a>),
    Dynamic {
        invoke: bool,
        bootstrap_index: u16,
        name: JvmString<'a>,
        descriptor: JvmString<'a>,
    },
    MethodHandle {
        kind: u8,
        reference_index: u16,
        member: MemberReference<'a>,
    },
    MethodType(JvmString<'a>),
}

/// Resolve a method-handle's original reference after checking its kind.
pub fn method_handle<'a>(
    class: &ClassFile<'a>,
    kind: u8,
    reference_index: u16,
) -> Result<MemberReference<'a>, ReferenceError> {
    let member = class.member_reference(reference_index)?;
    let constructor = member.name.bytes() == b"<init>";
    let ordinary = !constructor && member.name.bytes() != b"<clinit>";
    let valid = match kind {
        1..=4 => member.kind == ReferenceKind::Field,
        5 => member.kind == ReferenceKind::Method && ordinary,
        6..=7 => {
            ordinary
                && (member.kind == ReferenceKind::Method
                    || member.kind == ReferenceKind::InterfaceMethod && class.major_version() >= 52)
        }
        8 => member.kind == ReferenceKind::Method && constructor,
        9 => member.kind == ReferenceKind::InterfaceMethod && ordinary,
        _ => false,
    };
    if valid {
        Ok(member)
    } else {
        Err(ReferenceError::InvalidMethodHandle {
            kind,
            index: reference_index,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReferenceError {
    #[error(transparent)]
    Constant(#[from] ClassError),
    #[error("unexpected constant-pool entry #{index} for opcode 0x{opcode:02x}")]
    UnexpectedConstant { index: u16, opcode: u8 },
    #[error("constant-pool entry #{0} is not a loadable constant")]
    InvalidLoadableConstant(u16),
    #[error("invalid dynamic descriptor for constant-pool entry #{0}")]
    InvalidDynamicDescriptor(u16),
    #[error("invalid method-handle kind {kind} for reference #{index}")]
    InvalidMethodHandle { kind: u8, index: u16 },
}

impl ReferenceError {
    pub fn kind(&self) -> crate::FailureKind {
        match self {
            Self::Constant(error) => error.kind(),
            Self::UnexpectedConstant { .. }
            | Self::InvalidLoadableConstant(_)
            | Self::InvalidDynamicDescriptor(_)
            | Self::InvalidMethodHandle { .. } => crate::FailureKind::Malformed,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Switch<'a> {
    pub default: i32,
    low: Option<i32>,
    entries: &'a [u8],
}

impl<'a> Switch<'a> {
    /// Original key and relative jump offset, in instruction order.
    pub fn cases(self) -> impl ExactSizeIterator<Item = (i32, i32)> + 'a {
        let width = if self.low.is_some() { 4 } else { 8 };
        self.entries
            .chunks_exact(width)
            .enumerate()
            .map(move |(index, row)| {
                let first = i32::from_be_bytes([row[0], row[1], row[2], row[3]]);
                match self.low {
                    Some(low) => (low + index as i32, first),
                    None => (first, i32::from_be_bytes([row[4], row[5], row[6], row[7]])),
                }
            })
    }
}

/// Mnemonic in the JVM opcode namespace.
pub fn opcode_mnemonic(opcode: u8) -> &'static str {
    MNEMONICS
        .get(usize::from(opcode))
        .copied()
        .unwrap_or("unknown")
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(*bytes.get(offset..)?.first_chunk()?))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_be_bytes(*bytes.get(offset..)?.first_chunk()?))
}

fn switch_header(offset: usize) -> usize {
    4 - offset % 4
}

fn variable_width(opcode: u8, offset: usize, tail: &[u8]) -> Result<usize, DecodeProblem> {
    let truncated = DecodeProblem::TruncatedInstruction;
    let invalid_count = DecodeProblem::InvalidSwitchCount;
    match opcode {
        0xc4 => match *tail.get(1).ok_or(truncated)? {
            0x15..=0x19 | 0x36..=0x3a | 0xa9 => Ok(4),
            0x84 => Ok(6),
            modified => Err(DecodeProblem::InvalidWideOpcode(modified)),
        },
        0xaa | 0xab => {
            let header = switch_header(offset);
            let data = tail.get(header..).ok_or(truncated)?;
            let first = read_i32(data, 4).ok_or(truncated)?;
            let (header_size, count, entry_size) = if opcode == 0xaa {
                let high = read_i32(data, 8).ok_or(truncated)?;
                if high < first {
                    return Err(invalid_count);
                }
                (12, i64::from(high) - i64::from(first) + 1, 4)
            } else {
                (8, i64::from(first), 8)
            };
            let bytes = usize::try_from(count)
                .ok()
                .and_then(|count| count.checked_mul(entry_size))
                .and_then(|bytes| bytes.checked_add(header + header_size))
                .ok_or(invalid_count)?;
            Ok(bytes)
        }
        _ => Err(DecodeProblem::UndefinedOpcode(opcode)),
    }
}

const WIDTHS: [u8; 256] = {
    let mut table = [0; 256];
    let mut opcode = 0;
    while opcode <= 0xc9 {
        table[opcode] = match opcode {
            0x10 | 0x12 | 0x15..=0x19 | 0x36..=0x3a | 0xa9 | 0xbc => 2,
            0x11
            | 0x13..=0x14
            | 0x84
            | 0x99..=0xa8
            | 0xb2..=0xb8
            | 0xbb
            | 0xbd
            | 0xc0..=0xc1
            | 0xc6..=0xc7 => 3,
            0xc5 => 4,
            0xb9..=0xba | 0xc8..=0xc9 => 5,
            0xaa..=0xab | 0xc4 => 0,
            _ => 1,
        };
        opcode += 1;
    }
    table
};

#[rustfmt::skip]
const MNEMONICS: [&str; 202] = [
    "nop", "aconst_null", "iconst_m1", "iconst_0", "iconst_1", "iconst_2", "iconst_3", "iconst_4",
    "iconst_5", "lconst_0", "lconst_1", "fconst_0", "fconst_1", "fconst_2", "dconst_0", "dconst_1",
    "bipush", "sipush", "ldc", "ldc_w", "ldc2_w", "iload", "lload", "fload",
    "dload", "aload", "iload_0", "iload_1", "iload_2", "iload_3", "lload_0", "lload_1",
    "lload_2", "lload_3", "fload_0", "fload_1", "fload_2", "fload_3", "dload_0", "dload_1",
    "dload_2", "dload_3", "aload_0", "aload_1", "aload_2", "aload_3", "iaload", "laload",
    "faload", "daload", "aaload", "baload", "caload", "saload", "istore", "lstore",
    "fstore", "dstore", "astore", "istore_0", "istore_1", "istore_2", "istore_3", "lstore_0",
    "lstore_1", "lstore_2", "lstore_3", "fstore_0", "fstore_1", "fstore_2", "fstore_3", "dstore_0",
    "dstore_1", "dstore_2", "dstore_3", "astore_0", "astore_1", "astore_2", "astore_3", "iastore",
    "lastore", "fastore", "dastore", "aastore", "bastore", "castore", "sastore", "pop",
    "pop2", "dup", "dup_x1", "dup_x2", "dup2", "dup2_x1", "dup2_x2", "swap",
    "iadd", "ladd", "fadd", "dadd", "isub", "lsub", "fsub", "dsub",
    "imul", "lmul", "fmul", "dmul", "idiv", "ldiv", "fdiv", "ddiv",
    "irem", "lrem", "frem", "drem", "ineg", "lneg", "fneg", "dneg",
    "ishl", "lshl", "ishr", "lshr", "iushr", "lushr", "iand", "land",
    "ior", "lor", "ixor", "lxor", "iinc", "i2l", "i2f", "i2d",
    "l2i", "l2f", "l2d", "f2i", "f2l", "f2d", "d2i", "d2l",
    "d2f", "i2b", "i2c", "i2s", "lcmp", "fcmpl", "fcmpg", "dcmpl",
    "dcmpg", "ifeq", "ifne", "iflt", "ifge", "ifgt", "ifle", "if_icmpeq",
    "if_icmpne", "if_icmplt", "if_icmpge", "if_icmpgt", "if_icmple", "if_acmpeq", "if_acmpne", "goto",
    "jsr", "ret", "tableswitch", "lookupswitch", "ireturn", "lreturn", "freturn", "dreturn",
    "areturn", "return", "getstatic", "putstatic", "getfield", "putfield", "invokevirtual", "invokespecial",
    "invokestatic", "invokeinterface", "invokedynamic", "new", "newarray", "anewarray", "arraylength", "athrow",
    "checkcast", "instanceof", "monitorenter", "monitorexit", "wide", "multianewarray", "ifnull", "ifnonnull",
    "goto_w", "jsr_w",
];

#[cfg(test)]
#[path = "../../../../tests/internal/jvm_bytecode.rs"]
mod tests;
