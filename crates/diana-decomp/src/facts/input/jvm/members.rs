use super::{error, ClassError, ClassFile, ClassProblem, JvmString, Reader};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MemberKind {
    Field,
    Method,
}

/// One field or method at its original position in the declaration table.
#[derive(Clone, Copy, Debug)]
pub struct Member<'a> {
    source: &'a [u8],
    kind: MemberKind,
    ordinal: u16,
    offset: usize,
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Attributes<'a>,
}

impl<'a> Member<'a> {
    pub fn ordinal(self) -> u16 {
        self.ordinal
    }

    pub fn offset(self) -> usize {
        self.offset
    }

    pub fn access_flags(self) -> u16 {
        self.access_flags
    }

    pub fn name_index(self) -> u16 {
        self.name_index
    }

    pub fn descriptor_index(self) -> u16 {
        self.descriptor_index
    }

    pub fn name(self, class: &ClassFile<'a>) -> Result<JvmString<'a>, ClassError> {
        self.verify(class)?;
        class
            .pool
            .utf8(self.name_index, self.offset + 2)?
            .member_name(self.kind == MemberKind::Method)
    }

    pub fn descriptor(self, class: &ClassFile<'a>) -> Result<JvmString<'a>, ClassError> {
        self.verify(class)?;
        class
            .pool
            .utf8(self.descriptor_index, self.offset + 4)?
            .descriptor(self.kind == MemberKind::Method)
    }

    pub fn attributes(self) -> Attributes<'a> {
        self.attributes
    }

    pub(super) fn kind(self) -> MemberKind {
        self.kind
    }

    pub(super) fn verify(self, class: &ClassFile<'a>) -> Result<(), ClassError> {
        if std::ptr::eq(self.source, class.bytes) {
            Ok(())
        } else {
            Err(error(self.offset, ClassProblem::ForeignMember))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Members<'a> {
    source: &'a [u8],
    kind: MemberKind,
    reader: Reader<'a>,
    count: u16,
    next: u16,
    pending_issue: Option<ClassError>,
}

impl<'a> Members<'a> {
    pub(super) fn new(
        source: &'a [u8],
        mut reader: Reader<'a>,
        kind: MemberKind,
    ) -> Result<Self, ClassError> {
        let count = reader.u16()?;
        Ok(Self {
            source,
            kind,
            reader,
            count,
            next: 0,
            pending_issue: None,
        })
    }

    pub fn declared_count(&self) -> u16 {
        self.count
    }

    pub(super) fn skip_remaining(mut self) -> Result<Reader<'a>, ClassError> {
        for _ in self.next..self.count {
            // Only attribute envelopes locate the following member.
            self.reader.take(6)?;
            let attributes = self.reader.u16()?;
            skip_attributes(&mut self.reader, attributes)?;
        }
        Ok(self.reader)
    }

    fn read_member(&mut self) -> Result<Member<'a>, ClassError> {
        let offset = self.reader.at;
        let access_flags = self.reader.u16()?;
        let name_index = self.reader.u16()?;
        let descriptor_index = self.reader.u16()?;
        let attributes = match self.reader.u16() {
            Ok(count) => {
                let start = self.reader.at;
                let mut envelope = self.reader;
                let checked = skip_attributes(&mut envelope, count);
                let reader = match checked {
                    Ok(()) => {
                        self.reader = envelope;
                        Reader::new(&self.source[start..envelope.at], start)
                    }
                    Err(issue) => {
                        self.pending_issue = Some(issue);
                        self.reader
                    }
                };
                Attributes {
                    reader,
                    count,
                    check_end: false,
                    pending_issue: None,
                }
            }
            Err(issue) => {
                self.pending_issue = Some(issue);
                Attributes {
                    reader: self.reader,
                    count: 0,
                    check_end: false,
                    pending_issue: Some(issue),
                }
            }
        };
        Ok(Member {
            source: self.source,
            kind: self.kind,
            ordinal: self.next,
            offset,
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl<'a> Iterator for Members<'a> {
    type Item = Result<Member<'a>, ClassError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Recover the preceding member's original declaration even when its
        // attribute envelope prevents locating any following declarations.
        if let Some(issue) = self.pending_issue.take() {
            self.next = self.count;
            return Some(Err(issue));
        }
        if self.next == self.count {
            return None;
        }
        let result = self.read_member();
        if result.is_ok() {
            self.next += 1;
        } else {
            self.next = self.count;
        }
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = usize::from(self.count - self.next);
        (
            0,
            Some(remaining + usize::from(remaining != 0 || self.pending_issue.is_some())),
        )
    }
}

impl std::iter::FusedIterator for Members<'_> {}

/// An attribute payload inside the original class file.
#[derive(Clone, Copy, Debug)]
pub struct Attribute<'a> {
    pub name_index: u16,
    pub payload: &'a [u8],
    pub offset: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Attributes<'a> {
    reader: Reader<'a>,
    count: u16,
    check_end: bool,
    pending_issue: Option<ClassError>,
}

impl<'a> Attributes<'a> {
    pub(super) fn class(mut reader: Reader<'a>) -> Result<Self, ClassError> {
        let count = reader.u16()?;
        Ok(Self {
            reader,
            count,
            check_end: true,
            pending_issue: None,
        })
    }

    pub(super) fn embedded(reader: &mut Reader<'a>) -> Result<Self, ClassError> {
        let count = reader.u16()?;
        let start = *reader;
        skip_attributes(reader, count)?;
        let span = reader.at - start.at;
        Ok(Self {
            reader: Reader::new(&start.rest[..span], start.at),
            count,
            check_end: false,
            pending_issue: None,
        })
    }

    fn read(&mut self) -> Result<Attribute<'a>, ClassError> {
        let name_index = self.reader.u16()?;
        let length = attribute_size(&mut self.reader)?;
        let offset = self.reader.at;
        let payload = self.reader.take(length)?;
        Ok(Attribute {
            name_index,
            payload,
            offset,
        })
    }
}

fn attribute_size(reader: &mut Reader<'_>) -> Result<usize, ClassError> {
    let offset = reader.at;
    usize::try_from(reader.u32()?).map_err(|_| error(offset, ClassProblem::PlatformCapacity))
}

fn skip_attributes(reader: &mut Reader<'_>, count: u16) -> Result<(), ClassError> {
    for _ in 0..count {
        reader.take(2)?;
        let length = attribute_size(reader)?;
        reader.take(length)?;
    }
    Ok(())
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<Attribute<'a>, ClassError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(issue) = self.pending_issue.take() {
            return Some(Err(issue));
        }
        if self.count == 0 {
            let check_end = std::mem::take(&mut self.check_end);
            return (check_end && !self.reader.rest.is_empty())
                .then(|| Err(error(self.reader.at, ClassProblem::TrailingBytes)));
        }
        let result = self.read();
        if result.is_ok() {
            self.count -= 1;
        } else {
            self.count = 0;
            self.check_end = false;
        }
        Some(result)
    }
}

impl std::iter::FusedIterator for Attributes<'_> {}

#[derive(Clone, Copy, Debug)]
pub struct BootstrapMethod<'a> {
    pub ordinal: u16,
    pub method_ref: u16,
    pub offset: usize,
    arguments: &'a [u8],
}

impl<'a> BootstrapMethod<'a> {
    pub fn arguments(self) -> impl ExactSizeIterator<Item = u16> + 'a {
        self.arguments
            .as_chunks::<2>()
            .0
            .iter()
            .copied()
            .map(u16::from_be_bytes)
    }
}

#[derive(Clone, Debug)]
pub struct BootstrapMethods<'a> {
    reader: Reader<'a>,
    count: u16,
    next: u16,
    check_end: bool,
}

impl<'a> BootstrapMethods<'a> {
    pub(super) fn new(attribute: Attribute<'a>) -> Result<Self, ClassError> {
        let mut reader = Reader::new(attribute.payload, attribute.offset);
        let count = reader.u16()?;
        Ok(Self {
            reader,
            count,
            next: 0,
            check_end: true,
        })
    }

    pub(super) fn offset(&self) -> usize {
        self.reader.at
    }

    fn read(&mut self) -> Result<BootstrapMethod<'a>, ClassError> {
        let offset = self.reader.at;
        let method_ref = self.reader.u16()?;
        let count = usize::from(self.reader.u16()?);
        Ok(BootstrapMethod {
            ordinal: self.next,
            method_ref,
            offset,
            arguments: self.reader.take(count * 2)?,
        })
    }
}

impl<'a> Iterator for BootstrapMethods<'a> {
    type Item = Result<BootstrapMethod<'a>, ClassError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.count {
            let check_end = std::mem::take(&mut self.check_end);
            return (check_end && !self.reader.rest.is_empty())
                .then(|| Err(error(self.reader.at, ClassProblem::TrailingBytes)));
        }
        let result = self.read();
        if result.is_ok() {
            self.next += 1;
        } else {
            self.next = self.count;
            self.check_end = false;
        }
        Some(result)
    }
}

impl std::iter::FusedIterator for BootstrapMethods<'_> {}
