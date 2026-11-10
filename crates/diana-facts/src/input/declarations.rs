use super::class_locations::ClassLocations;
use super::header::{SUPPORTED_MAP_KINDS, map_kind};
use super::strings::{DexString, TypeShape};
use super::{
    Cursor, DexContainer, DexHeader, InputError, InputProblem, count_usize, file_offset, malformed,
};
use crate::{ByteOffset, ClassDefId, FieldId, MethodId, ProtoId, StringId, TypeId};

#[derive(Clone, Copy)]
pub(super) struct Section {
    pub(super) kind: u16,
    pub(super) count: u32,
    pub(super) start_byte: ByteOffset,
    pub(super) end_byte: ByteOffset,
}

pub struct LocalDeclarations<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) header: DexHeader,
    container: DexContainer<'a>,
    pub(super) sections: [Option<Section>; SUPPORTED_MAP_KINDS],
    locations: std::sync::OnceLock<ClassLocations>,
    build: std::sync::Mutex<()>,
    /// The descriptor shape established per `type_ids` index, `0` meaning not yet
    /// read. A prototype names the same handful of types for millions of members, so
    /// re-deriving and re-validating a descriptor per member access dominated listing
    /// scans. The stored value is a pure function of immutable bytes.
    shapes: Vec<std::sync::atomic::AtomicU8>,
    /// The resolved `string_data_item` per `string_ids` index: payload start, payload
    /// end and declared unit count, with start `0` meaning not yet read. One payload
    /// is reached once per member that refers to it, so re-reading the pool record,
    /// the length prefix and the terminator dominated listing scans.
    starts: Vec<std::sync::atomic::AtomicU32>,
    ends: Vec<std::sync::atomic::AtomicU32>,
    units: Vec<std::sync::atomic::AtomicU32>,
    /// The `string_ids` index named by each `type_ids` record, `0` meaning not yet
    /// read; a stored value is the index plus one.
    type_strings: Vec<std::sync::atomic::AtomicU32>,
    /// Which `string_ids` entries already passed member-name validation. A pool holds
    /// a name once but a listing reaches it once per member that uses it, and the
    /// check walks every unit. Only accepted names are recorded, so an error keeps
    /// being reported from the same entry that produced it.
    names: Vec<std::sync::atomic::AtomicU8>,
}
pub enum LocalParent {
    Missing,
    Root,
    Type(TypeId),
}
pub(super) struct ClassRecord {
    pub type_id: TypeId,
    pub access_flags: u32,
    pub superclass: u32,
    pub interfaces: u32,
    pub source_file: u32,
    pub annotations: u32,
    pub class_data: u32,
    pub values: u32,
}
impl<'a> LocalDeclarations<'a> {
    pub(super) fn new(
        container: DexContainer<'a>,
        header: DexHeader,
        sections: [Option<Section>; SUPPORTED_MAP_KINDS],
    ) -> Result<Self, InputError> {
        // Table extents were checked before construction. Cache storage is bounded
        // by those accepted counts; reservation failure remains a classified error.
        let shapes = cache_slots(header.type_ids.count, header.type_ids.offset_byte)?;
        let names = cache_slots(header.string_ids.count, header.string_ids.offset_byte)?;
        let starts = cache_slots(header.string_ids.count, header.string_ids.offset_byte)?;
        let ends = cache_slots(header.string_ids.count, header.string_ids.offset_byte)?;
        let units = cache_slots(header.string_ids.count, header.string_ids.offset_byte)?;
        let type_strings = cache_slots(header.type_ids.count, header.type_ids.offset_byte)?;
        Ok(Self {
            bytes: container.bytes,
            header,
            container,
            sections,
            locations: std::sync::OnceLock::new(),
            build: std::sync::Mutex::new(()),
            shapes,
            names,
            starts,
            ends,
            units,
            type_strings,
        })
    }
    pub fn class_count(&self) -> u32 {
        self.header.class_defs.count
    }
    pub(super) fn class_type_id(&self, class: ClassDefId) -> Result<TypeId, InputError> {
        let mut record = self.table(TableKey::Class(class))?;
        Ok(TypeId(record.read_u32()?))
    }
    pub(super) fn class_record(&self, class: ClassDefId) -> Result<ClassRecord, InputError> {
        let mut record = self.table(TableKey::Class(class))?;
        Ok(ClassRecord {
            type_id: TypeId(record.read_u32()?),
            access_flags: record.read_u32()?,
            superclass: record.read_u32()?,
            interfaces: record.read_u32()?,
            source_file: record.read_u32()?,
            annotations: record.read_u32()?,
            class_data: record.read_u32()?,
            values: record.read_u32()?,
        })
    }
    pub(super) fn checked_class(
        &self,
        class: ClassDefId,
    ) -> Result<(ClassRecord, DexString<'a>), InputError> {
        let record = self.class_record(class)?;
        let (name, shape) = self.type_descriptor(record.type_id)?;
        if !matches!(shape, TypeShape::Class) {
            return Err(malformed(
                InputProblem::InvalidDescriptor,
                self.table(TableKey::Class(class))?.position_byte(),
            ));
        }
        Ok((record, name))
    }
    pub(super) fn class_location(&self, ty: TypeId) -> Result<Option<ClassDefId>, InputError> {
        if self.locations.get().is_none() {
            let _lock = self
                .build
                .lock()
                .map_err(|_| InputError::InternalInvariant("input cache construction poisoned"))?;
            if let Some(locations) = self.locations.get() {
                return locations.get(ty, self);
            }
            let locations = ClassLocations::build(self)?;
            let _ = self.locations.set(locations);
        }
        self.locations
            .get()
            .ok_or(InputError::InternalInvariant(
                "class location cache was not initialized",
            ))?
            .get(ty, self)
    }
    pub fn parent(&self, ty: TypeId) -> Result<LocalParent, InputError> {
        let Some(class) = self.class_location(ty)? else {
            return Ok(LocalParent::Missing);
        };
        let (record, _) = self.checked_class(class)?;
        self.superclass(record.superclass)?;
        Ok(if record.superclass == u32::MAX {
            LocalParent::Root
        } else {
            LocalParent::Type(TypeId(record.superclass))
        })
    }
    pub(super) fn item(
        &self,
        kind: u16,
        offset_byte: ByteOffset,
    ) -> Result<Cursor<'a>, InputError> {
        let (bit, alignment) =
            map_kind(kind).ok_or_else(|| malformed(InputProblem::InvalidReference, offset_byte))?;
        let section = self.sections[bit.trailing_zeros() as usize]
            .ok_or_else(|| malformed(InputProblem::InvalidReference, offset_byte))?;
        if offset_byte < section.start_byte || offset_byte >= section.end_byte {
            return Err(malformed(InputProblem::InvalidReference, offset_byte));
        }
        if !offset_byte.0.is_multiple_of(alignment) {
            return Err(malformed(InputProblem::InvalidAlignment, offset_byte));
        }
        let mut end = self.container.item_end(offset_byte)?.min(section.end_byte);
        let link = self.header.link_byte_range;
        if offset_byte >= link.start && offset_byte < link.end {
            return Err(malformed(InputProblem::OverlappingSections, offset_byte));
        }
        if offset_byte < link.start {
            end = end.min(link.start);
        }
        Cursor::bounded(self.bytes, offset_byte, end)
    }

    /// One record of a fixed table.
    ///
    /// The table extent was validated when this reader was built (`offset + count ×
    /// stride` inside the member), so a record lies inside it by construction and
    /// only the index bound needs a check. Re-deriving both range bounds per access
    /// cost more than reading the record: a string pool is millions of accesses.
    pub(super) fn table(&self, key: TableKey) -> Result<Cursor<'a>, InputError> {
        let (table, index, stride) = match key {
            TableKey::String(id) => (self.header.string_ids, id.0, 4usize),
            TableKey::Type(id) => (self.header.type_ids, id.0, 4),
            TableKey::Proto(id) => (self.header.proto_ids, id.0, 12),
            TableKey::Field(id) => (self.header.field_ids, id.0, 8),
            TableKey::Method(id) => (self.header.method_ids, id.0, 8),
            TableKey::Class(id) => (self.header.class_defs, id.0, 32),
        };
        if index >= table.count {
            return Err(malformed(InputProblem::InvalidReference, table.offset_byte));
        }
        let index = count_usize(index, table.offset_byte)?;
        let start = ByteOffset(table.offset_byte.0 + index * stride);
        Cursor::bounded(self.bytes, start, ByteOffset(start.0 + stride))
    }

    pub(super) fn type_descriptor(
        &self,
        type_idx: TypeId,
    ) -> Result<(DexString<'a>, TypeShape), InputError> {
        let string_idx = match self.cached_type_string(type_idx) {
            Some(string_idx) => string_idx,
            None => {
                let mut record = self.table(TableKey::Type(type_idx))?;
                let string_idx = StringId(record.read_u32()?);
                self.remember_type_string(type_idx, string_idx);
                string_idx
            }
        };
        let descriptor = self.string(string_idx)?;
        let shape = match self.cached_shape(type_idx) {
            Some(shape) => shape,
            None => {
                let shape = descriptor.shape()?;
                self.remember_shape(type_idx, shape);
                shape
            }
        };
        Ok((descriptor, shape))
    }

    fn cached_type_string(&self, type_idx: TypeId) -> Option<StringId> {
        let slot = self.type_strings.get(usize::try_from(type_idx.0).ok()?)?;
        match slot.load(std::sync::atomic::Ordering::Relaxed) {
            0 => None,
            stored => Some(StringId(stored - 1)),
        }
    }

    fn remember_type_string(&self, type_idx: TypeId, string_idx: StringId) {
        let Some(slot) = usize::try_from(type_idx.0)
            .ok()
            .and_then(|index| self.type_strings.get(index))
        else {
            return;
        };
        slot.store(
            string_idx.0.wrapping_add(1),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    fn cached_shape(&self, type_idx: TypeId) -> Option<TypeShape> {
        let slot = self.shapes.get(usize::try_from(type_idx.0).ok()?)?;
        decode_shape(slot.load(std::sync::atomic::Ordering::Relaxed))
    }

    fn remember_shape(&self, type_idx: TypeId, shape: TypeShape) {
        let Some(slot) = usize::try_from(type_idx.0)
            .ok()
            .and_then(|index| self.shapes.get(index))
        else {
            return;
        };
        slot.store(encode_shape(shape), std::sync::atomic::Ordering::Relaxed);
    }

    /// The string at `string_idx`, validated as a member name at most once.
    ///
    /// `method_ids` and `field_ids` name their member through a pool entry, and the
    /// same entry is read again for every member that refers to it, so the unit walk
    /// of [`DexString::validate_name`] ran millions of times over a pool of a few
    /// hundred thousand names.
    pub(super) fn member_name(&self, string_idx: StringId) -> Result<DexString<'a>, InputError> {
        let name = self.string(string_idx)?;
        let Some(slot) = usize::try_from(string_idx.0)
            .ok()
            .and_then(|index| self.names.get(index))
        else {
            name.validate_name()?;
            return Ok(name);
        };
        if slot.load(std::sync::atomic::Ordering::Relaxed) != 0 {
            return Ok(name);
        }
        name.validate_name()?;
        slot.store(1, std::sync::atomic::Ordering::Relaxed);
        Ok(name)
    }

    pub(super) fn string(&self, string_idx: StringId) -> Result<DexString<'a>, InputError> {
        if let Some(string) = self.cached_string(string_idx) {
            return Ok(string);
        }
        let mut record = self.table(TableKey::String(string_idx))?;
        let offset = file_offset(record.read_u32()?, ByteOffset(record.position_byte().0 - 4))?;
        let string = DexString::read(self.bytes, self.item(0x2002, offset)?)?;
        self.remember_string(string_idx, string);
        Ok(string)
    }

    /// A previously resolved item, re-created from its recorded parts.
    ///
    /// The start is published last with release ordering, so seeing a non-zero start
    /// also establishes the end and unit count written before it.
    fn cached_string(&self, string_idx: StringId) -> Option<DexString<'a>> {
        let index = usize::try_from(string_idx.0).ok()?;
        let start = self
            .starts
            .get(index)?
            .load(std::sync::atomic::Ordering::Acquire);
        if start == 0 {
            return None;
        }
        let end = self
            .ends
            .get(index)?
            .load(std::sync::atomic::Ordering::Relaxed);
        let utf16_len = self
            .units
            .get(index)?
            .load(std::sync::atomic::Ordering::Relaxed);
        DexString::from_parts(self.bytes, start, end, utf16_len)
    }

    fn remember_string(&self, string_idx: StringId, string: DexString<'a>) {
        let Some(index) = usize::try_from(string_idx.0).ok() else {
            return;
        };
        let (Some(start_slot), Some(end_slot), Some(units)) = (
            self.starts.get(index),
            self.ends.get(index),
            self.units.get(index),
        ) else {
            return;
        };
        let Some((start, end, utf16_len)) = string.parts() else {
            return;
        };
        units.store(utf16_len, std::sync::atomic::Ordering::Relaxed);
        end_slot.store(end, std::sync::atomic::Ordering::Relaxed);
        start_slot.store(start, std::sync::atomic::Ordering::Release);
    }

    pub(super) fn superclass(&self, index: u32) -> Result<Option<DexString<'a>>, InputError> {
        if index == u32::MAX {
            return Ok(None);
        }
        let (name, shape) = self.type_descriptor(TypeId(index))?;
        if !matches!(shape, TypeShape::Class) {
            return Err(malformed(
                InputProblem::InvalidDescriptor,
                self.header.type_ids.offset_byte,
            ));
        }
        Ok(Some(name))
    }
}

/// Fallible storage for per-index caches. Counts refer to already bounded tables.
pub(super) fn cache_slots<T: Default>(count: u32, at: ByteOffset) -> Result<Vec<T>, InputError> {
    let count = count_usize(count, at)?;
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(count)
        .map_err(|_| InputError::Allocation { offset_byte: at })?;
    slots.resize_with(count, T::default);
    Ok(slots)
}

pub(super) enum TableKey {
    String(StringId),
    Type(TypeId),
    Proto(ProtoId),
    Field(FieldId),
    Method(MethodId),
    Class(ClassDefId),
}

/// Encodes a shape into one byte: `0` is "not read yet", `1..=3` the non-primitive
/// shapes and anything above is the primitive tag itself.
fn encode_shape(shape: TypeShape) -> u8 {
    match shape {
        TypeShape::Void => 1,
        TypeShape::Class => 2,
        TypeShape::Array => 3,
        TypeShape::Primitive(primitive) => primitive as u8,
    }
}

fn decode_shape(byte: u8) -> Option<TypeShape> {
    match byte {
        0 => None,
        1 => Some(TypeShape::Void),
        2 => Some(TypeShape::Class),
        3 => Some(TypeShape::Array),
        tag => Some(TypeShape::Primitive(
            super::strings::PrimitiveType::from_tag(u16::from(tag))?,
        )),
    }
}
