//! Original, ordered component declarations from the optional Record attribute.

use super::{error, Attribute, Attributes, ClassError, ClassFile, ClassProblem, JvmString, Reader};

#[derive(Clone, Copy, Debug)]
pub struct RecordComponent<'a> {
    pub ordinal: u16,
    pub offset: usize,
    pub name: JvmString<'a>,
    pub descriptor: JvmString<'a>,
    pub attributes: Attributes<'a>,
}

pub struct RecordComponents<'a, 'c> {
    class: &'c ClassFile<'a>,
    reader: Reader<'a>,
    count: u16,
    ordinal: u16,
    check_end: bool,
}

impl<'a, 'c> RecordComponents<'a, 'c> {
    pub(super) fn new(
        class: &'c ClassFile<'a>,
        attribute: Attribute<'a>,
    ) -> Result<Self, ClassError> {
        let mut reader = Reader::new(attribute.payload, attribute.offset);
        let count = reader.u16()?;
        Ok(Self {
            class,
            reader,
            count,
            ordinal: 0,
            check_end: true,
        })
    }

    pub fn declared_count(&self) -> u16 {
        self.count
    }

    fn next_component(&mut self) -> Result<RecordComponent<'a>, ClassError> {
        let offset = self.reader.at;
        let name = self.reader.u16()?;
        let descriptor = self.reader.u16()?;
        // Advance to the next independently decodable record before resolving
        // the name and descriptor; an invalid pool reference is local.
        let attributes = Attributes::embedded(&mut self.reader)?;
        self.ordinal += 1;
        Ok(RecordComponent {
            ordinal: self.ordinal - 1,
            offset,
            name: self.class.pool.utf8(name, offset)?.member_name(false)?,
            descriptor: self
                .class
                .pool
                .utf8(descriptor, offset + 2)?
                .descriptor(false)?,
            attributes,
        })
    }
}

impl<'a> Iterator for RecordComponents<'a, '_> {
    type Item = Result<RecordComponent<'a>, ClassError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ordinal == self.count {
            let check = std::mem::take(&mut self.check_end);
            return (check && !self.reader.rest.is_empty())
                .then(|| Err(error(self.reader.at, ClassProblem::TrailingBytes)));
        }
        let before = self.ordinal;
        let result = self.next_component();
        if self.ordinal == before {
            self.ordinal = self.count;
            self.check_end = false;
        }
        Some(result)
    }
}

impl std::iter::FusedIterator for RecordComponents<'_, '_> {}
