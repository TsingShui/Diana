use super::code::read_code;
use super::declarations::{LocalDeclarations, Section, TableKey};
use super::header::{map_kind, SUPPORTED_MAP_KINDS};
use super::strings::{DexString, TypeShape};
use super::{
    checked_range, count_usize, file_offset, malformed, Cursor, DexContainer, DexHeader,
    InputError, InputProblem,
};

use crate::{
    ByteLength, ByteOffset, ClassDefId, ClassKey, FailureKind, FieldId, MethodId, ProtoId,
    RegisterWordCount, StringId, TypeId, Utf16Offset,
};

/// A single-image index borrowing original bytes. Variable metadata is resolved
/// on demand; it owns neither a decoded string pool nor method result buffers.
pub struct ProgramIndex<'a> {
    pub bytes: &'a [u8],
    dex: crate::DexId,
    class_facts: std::sync::Mutex<
        std::collections::HashMap<
            ClassDefId,
            Result<std::sync::Arc<crate::p0::ClassFacts>, InputError>,
        >,
    >,
    facts_build: std::sync::Mutex<()>,
    pub(super) declarations: std::sync::Arc<LocalDeclarations<'a>>,
    pub header: DexHeader,
    prototypes: std::sync::OnceLock<Vec<std::sync::OnceLock<PrototypeFacts<'a>>>>,
    prototype_build: std::sync::Mutex<()>,
}

impl<'a> ProgramIndex<'a> {
    /// Original physical bytes this index borrows.
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub(super) fn dex_id(&self) -> crate::DexId {
        self.dex
    }

    /// Number of original class definitions in this logical member.
    pub fn class_count(&self) -> u32 {
        self.header.class_defs.count
    }

    /// Binds the logical DEX identity used for original class keys.
    pub fn with_dex(mut self, dex: crate::DexId) -> Self {
        self.dex = dex;
        self
    }

    /// The current class's P0 facts, built once per logical class and shared by
    /// later declaration queries. Deterministic input failures are cached;
    /// resource failures are not.
    pub fn class_facts_cached(
        &self,
        class: ClassDefId,
    ) -> Result<std::sync::Arc<crate::p0::ClassFacts>, crate::RequestError> {
        {
            let cache = self.class_facts.lock().map_err(|_| {
                crate::RequestError::InternalInvariant("class-fact cache was poisoned")
            })?;
            if let Some(entry) = cache.get(&class) {
                return match entry {
                    Ok(facts) => Ok(std::sync::Arc::clone(facts)),
                    Err(error) => Err((*error).into()),
                };
            }
        }
        let _build = self.facts_build.lock().map_err(|_| {
            crate::RequestError::InternalInvariant("class-fact build lock was poisoned")
        })?;
        {
            let cache = self.class_facts.lock().map_err(|_| {
                crate::RequestError::InternalInvariant("class-fact cache was poisoned")
            })?;
            if let Some(entry) = cache.get(&class) {
                return match entry {
                    Ok(facts) => Ok(std::sync::Arc::clone(facts)),
                    Err(error) => Err((*error).into()),
                };
            }
        }
        let key = ClassKey {
            dex: self.dex,
            class_def: class,
        };
        let result = crate::p0::collect(self, key);
        let mut cache = self
            .class_facts
            .lock()
            .map_err(|_| crate::RequestError::InternalInvariant("class-fact cache was poisoned"))?;
        match result {
            Ok(facts) => {
                let facts = std::sync::Arc::new(facts);
                cache.insert(class, Ok(std::sync::Arc::clone(&facts)));
                Ok(facts)
            }
            Err(crate::RequestError::Input(error)) => {
                cache.insert(class, Err(error));
                Err(error.into())
            }
            Err(error) => Err(error),
        }
    }

    pub(super) fn class_type_id(&self, class: ClassDefId) -> Result<TypeId, InputError> {
        self.declarations.class_type_id(class)
    }
    fn class_location(&self, ty: TypeId) -> Result<Option<ClassDefId>, InputError> {
        self.declarations.class_location(ty)
    }

    pub fn open(bytes: &'a [u8]) -> Result<Self, InputError> {
        let container = DexContainer::open(bytes)?;
        Self::from_container(container, ByteOffset(0))
    }

    pub fn from_container(
        container: DexContainer<'a>,
        header_offset: ByteOffset,
    ) -> Result<Self, InputError> {
        let bytes = container.bytes;
        let header = super::header::read_member_header(container, header_offset)?;
        let mut cursor = Cursor::bounded(
            bytes,
            header.map_byte_range.start,
            header.map_byte_range.end,
        )?;
        let count = count_usize(cursor.read_u32()?, header.map_byte_range.start)?;
        let mut sections = [Section {
            kind: 0,
            count: 0,
            start_byte: ByteOffset(0),
            end_byte: ByteOffset(0),
        }; SUPPORTED_MAP_KINDS];
        if count > sections.len() {
            return Err(malformed(
                InputProblem::InvalidMap,
                header.map_byte_range.start,
            ));
        }
        for index in 0..count {
            let kind = cursor.read_u16()?;
            cursor.read_u16()?;
            let item_count = cursor.read_u32()?;
            let start_byte =
                file_offset(cursor.read_u32()?, ByteOffset(cursor.position_byte().0 - 4))?;
            if index > 0 {
                sections[index - 1].end_byte = start_byte;
            }
            sections[index] = Section {
                kind,
                count: item_count,
                start_byte,
                end_byte: header.data_byte_range.end,
            };
        }
        let mut by_kind = [None; SUPPORTED_MAP_KINDS];
        for section in &sections[..count] {
            let (bit, _) = map_kind(section.kind)
                .ok_or_else(|| malformed(InputProblem::InvalidMap, section.start_byte))?;
            by_kind[bit.trailing_zeros() as usize] = Some(*section);
        }
        Ok(Self {
            bytes,
            prototypes: std::sync::OnceLock::new(),
            prototype_build: std::sync::Mutex::new(()),
            dex: crate::DexId(0),
            class_facts: std::sync::Mutex::new(std::collections::HashMap::new()),
            facts_build: std::sync::Mutex::new(()),
            declarations: std::sync::Arc::new(LocalDeclarations::new(
                container,
                header.clone(),
                by_kind,
            )?),
            header,
        })
    }

    pub fn string(&self, string_idx: StringId) -> Result<DexString<'a>, InputError> {
        self.declarations.string(string_idx)
    }

    /// The pool entry named by a member identity, validated once per entry.
    fn member_name(&self, string_idx: StringId) -> Result<DexString<'a>, InputError> {
        self.declarations.member_name(string_idx)
    }

    pub fn class(&self, class_def_idx: ClassDefId) -> Result<ClassView<'_, 'a>, InputError> {
        let (record, descriptor) = self.declarations.checked_class(class_def_idx)?;
        let at = self.table(TableKey::Class(class_def_idx))?.position_byte();
        let class_data_offset = file_offset(record.class_data, ByteOffset(at.0 + 24))?;
        Ok(ClassView {
            index: self,
            class_def_idx,
            type_idx: record.type_id,
            access_flags: record.access_flags,
            descriptor,
            class_data_offset: (class_data_offset != ByteOffset(0)).then_some(class_data_offset),
        })
    }

    /// Number of original ``string_ids`` entries, not the count of strings that
    /// appear in code.
    pub fn string_ids_len(&self) -> u32 {
        self.header.string_ids.count
    }

    /// Number of original ``type_ids`` entries.
    pub fn type_ids_len(&self) -> u32 {
        self.header.type_ids.count
    }

    /// Number of original ``field_ids`` entries.
    pub fn field_ids_len(&self) -> u32 {
        self.header.field_ids.count
    }

    /// Number of original ``method_ids`` entries.
    pub fn method_ids_len(&self) -> u32 {
        self.header.method_ids.count
    }

    /// Visits original class_def identities independently. Malformed class
    /// metadata is delivered locally; resource failures stop the request.
    pub fn visit_classes(
        &self,
        mut visit: impl FnMut(
            ClassDefId,
            Result<ClassView<'_, 'a>, InputError>,
        ) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        for index in 0..self.header.class_defs.count {
            let class = local(self.class(ClassDefId(index)))?;
            visit(ClassDefId(index), class)?;
        }
        Ok(())
    }

    pub(super) fn item(
        &self,
        kind: u16,
        offset_byte: ByteOffset,
    ) -> Result<Cursor<'a>, InputError> {
        self.declarations.item(kind, offset_byte)
    }

    pub fn mapped_count(&self, kind: u16) -> u32 {
        map_kind(kind)
            .and_then(|(bit, _)| self.declarations.sections[bit.trailing_zeros() as usize])
            .map_or(0, |section| section.count)
    }
    pub(super) fn mapped_entry(
        &self,
        kind: u16,
        id: u32,
        stride: ByteLength,
    ) -> Result<Cursor<'a>, InputError> {
        let (bit, _) = map_kind(kind)
            .ok_or_else(|| malformed(InputProblem::InvalidReference, self.header.header_offset))?;
        let section = self.declarations.sections[bit.trailing_zeros() as usize]
            .ok_or_else(|| malformed(InputProblem::InvalidReference, self.header.header_offset))?;
        if id >= section.count {
            return Err(malformed(
                InputProblem::InvalidReference,
                section.start_byte,
            ));
        }
        let prefix = checked_range(
            ByteLength(section.end_byte.0),
            section.start_byte,
            count_usize(id, section.start_byte)?,
            stride,
        )?;
        let range = checked_range(ByteLength(section.end_byte.0), prefix.end, 1, stride)?;
        let cursor = self.item(kind, range.start)?;
        if cursor.remaining_byte().0 < stride.0 {
            return Err(malformed(InputProblem::Truncated, range.start));
        }

        Cursor::bounded(self.bytes, range.start, range.end)
    }

    fn table(&self, key: TableKey) -> Result<Cursor<'a>, InputError> {
        self.declarations.table(key)
    }
    pub fn type_descriptor(
        &self,
        type_idx: TypeId,
    ) -> Result<(DexString<'a>, TypeShape), InputError> {
        self.declarations.type_descriptor(type_idx)
    }

    pub fn prototype(&self, proto_idx: ProtoId) -> Result<Prototype<'_, 'a>, InputError> {
        if proto_idx.0 >= self.header.proto_ids.count {
            return Err(malformed(
                InputProblem::InvalidReference,
                self.header.proto_ids.offset_byte,
            ));
        }
        let prototypes = match self.prototypes.get() {
            Some(prototypes) => prototypes.as_slice(),
            None => self.initialize_prototypes()?,
        };
        let slot = prototypes
            .get(count_usize(proto_idx.0, self.header.proto_ids.offset_byte)?)
            .ok_or(InputError::InternalInvariant(
                "prototype cache lost a checked table index",
            ))?;
        let facts = match slot.get() {
            Some(facts) => facts,
            None => {
                // Only successful construction is published. A failed declaration or
                // allocation remains local to its caller, not a cached absence.
                let facts = self.read_prototype(proto_idx)?;
                let _ = slot.set(facts);
                slot.get().ok_or(InputError::InternalInvariant(
                    "prototype cache publication lost",
                ))?
            }
        };
        Ok(Prototype {
            shorty: facts.shorty,
            return_type: facts.return_type,
            return_type_idx: facts.return_type_idx,
            parameters: facts.parameters,
            parameter_descriptors: &facts.descriptors,
            parameter_words: facts.parameter_words,
        })
    }

    /// Slot storage is needed only by prototype consumers, not field/class scans.
    /// Serialize first construction so racing callers do not reserve duplicate
    /// vectors; failed reservation leaves the cell empty and remains retryable.
    fn initialize_prototypes(
        &self,
    ) -> Result<&[std::sync::OnceLock<PrototypeFacts<'a>>], InputError> {
        let _build = self
            .prototype_build
            .lock()
            .map_err(|_| InputError::InternalInvariant("prototype cache construction poisoned"))?;
        if self.prototypes.get().is_none() {
            let slots = super::declarations::cache_slots(
                self.header.proto_ids.count,
                self.header.proto_ids.offset_byte,
            )?;
            let _ = self.prototypes.set(slots);
        }
        self.prototypes
            .get()
            .map(Vec::as_slice)
            .ok_or(InputError::InternalInvariant(
                "prototype slot publication lost",
            ))
    }

    fn read_prototype(&self, proto_idx: ProtoId) -> Result<PrototypeFacts<'a>, InputError> {
        let mut record = self.table(TableKey::Proto(proto_idx))?;
        let shorty = self.string(StringId(record.read_u32()?))?;
        let return_type_idx = TypeId(record.read_u32()?);
        let (return_type, return_shape) = self.type_descriptor(return_type_idx)?;
        let parameters_offset =
            file_offset(record.read_u32()?, ByteOffset(record.position_byte().0 - 4))?;
        let parameters = if parameters_offset == ByteOffset(0) {
            &self.bytes[0..0]
        } else {
            let mut parameters = self.item(0x1001, parameters_offset)?;
            let count = parameters.read_u32()?;
            let range = parameters.skip(count_usize(count, parameters_offset)?, ByteLength(2))?;
            &self.bytes[range.as_range()]
        };
        if u64::from(shorty.utf16_len().0) != (parameters.len() / 2) as u64 + 1 {
            return Err(malformed(InputProblem::InvalidPrototype, parameters_offset));
        }
        let mut shorty_units = shorty.reader()?;
        if shorty_units.next()? != Some(u16::from(return_shape.shorty())) {
            return Err(malformed(InputProblem::InvalidPrototype, parameters_offset));
        }
        let mut descriptors = Vec::new();
        descriptors
            .try_reserve_exact(parameters.len() / 2)
            .map_err(|_| InputError::Allocation {
                offset_byte: parameters_offset,
            })?;
        let mut parameter_words = RegisterWordCount(0);
        for word in parameters.as_chunks::<2>().0 {
            let type_idx = TypeId(u32::from(u16::from_le_bytes(*word)));
            let (name, shape) = self.type_descriptor(type_idx)?;
            if shape.words() == 0 || shorty_units.next()? != Some(u16::from(shape.shorty())) {
                return Err(malformed(InputProblem::InvalidPrototype, parameters_offset));
            }
            parameter_words.0 += u64::from(shape.words());
            descriptors.push(name);
        }
        Ok(PrototypeFacts {
            shorty,
            return_type,
            return_type_idx,
            parameters,
            descriptors,
            parameter_words,
        })
    }

    pub(super) fn method_declaration(
        &self,
        method_idx: MethodId,
        owner: TypeId,
    ) -> Result<MethodDeclaration<'_, 'a>, InputError> {
        self.method_declaration_if(method_idx, owner, |_, _| Ok(true))?
            .ok_or(InputError::InternalInvariant(
                "unfiltered declaration was rejected",
            ))
    }

    fn method_declaration_if(
        &self,
        method_idx: MethodId,
        owner: TypeId,
        mut accept: impl FnMut(StringId, DexString<'a>) -> Result<bool, InputError>,
    ) -> Result<Option<MethodDeclaration<'_, 'a>>, InputError> {
        let (name_id, name, proto_idx) = self.method_header(method_idx, owner)?;
        if !accept(name_id, name)? {
            return Ok(None);
        }
        let prototype = self.prototype(proto_idx)?;
        Ok(Some(MethodDeclaration { name, prototype }))
    }

    fn method_header(
        &self,
        method_idx: MethodId,
        owner: TypeId,
    ) -> Result<(StringId, DexString<'a>, ProtoId), InputError> {
        let mut record = self.table(TableKey::Method(method_idx))?;
        if TypeId(u32::from(record.read_u16()?)) != owner {
            return Err(malformed(
                InputProblem::InvalidReference,
                ByteOffset(record.position_byte().0 - 2),
            ));
        }
        let proto_idx = ProtoId(u32::from(record.read_u16()?));
        let name_id = StringId(record.read_u32()?);
        let name = self.member_name(name_id)?;
        Ok((name_id, name, proto_idx))
    }

    pub(super) fn validate_field(
        &self,
        field_idx: FieldId,
        owner: TypeId,
    ) -> Result<FieldDeclaration<'a>, InputError> {
        let mut record = self.table(TableKey::Field(field_idx))?;
        if TypeId(u32::from(record.read_u16()?)) != owner {
            return Err(malformed(
                InputProblem::InvalidReference,
                ByteOffset(record.position_byte().0 - 2),
            ));
        }
        let field_type_idx = TypeId(u32::from(record.read_u16()?));
        let (field_type, shape) = self.type_descriptor(field_type_idx)?;
        let name = self.member_name(StringId(record.read_u32()?))?;
        if shape.words() == 0 {
            return Err(malformed(
                InputProblem::InvalidDescriptor,
                ByteOffset(record.position_byte().0 - 6),
            ));
        }
        Ok(FieldDeclaration {
            name,
            field_type,
            field_type_idx,
        })
    }
}

impl<'a> ProgramIndex<'a> {
    pub fn type_name(&self, id: TypeId) -> Result<DexString<'a>, InputError> {
        Ok(self.type_descriptor(id)?.0)
    }

    pub fn local_field_flags(&self, field_id: FieldId) -> Result<Option<u32>, InputError> {
        let mut record = self.table(TableKey::Field(field_id))?;
        let owner = TypeId(u32::from(record.read_u16()?));
        let Some(class_id) = self.class_location(owner)? else {
            return Ok(None);
        };
        let mut found = None;
        self.class(class_id)?.visit_members_with(
            MemberSelection::Fields,
            |entry| -> Result<(), InputError> {
                let MemberEntry::Field(field) = entry else {
                    return Err(InputError::InternalInvariant(
                        "local field lookup acquired another member",
                    ));
                };
                if field.field_idx == field_id {
                    field.declaration?;
                    if found.is_some() {
                        return Err(malformed(
                            InputProblem::InvalidClassData,
                            self.header.class_defs.offset_byte,
                        ));
                    }
                    found = Some(field.access_flags);
                }
                Ok(())
            },
        )?;
        Ok(found)
    }
    pub fn local_method(&self, method_id: MethodId) -> Result<Option<LocalMethod<'a>>, InputError> {
        let mut method = self.table(TableKey::Method(method_id))?;
        let owner = TypeId(u32::from(method.read_u16()?));
        let Some(class_id) = self.class_location(owner)? else {
            return Ok(None);
        };
        let mut found = None;
        self.class(class_id)?.visit_members_with(
            MemberSelection::AllMembers,
            |entry| -> Result<(), InputError> {
                if let MemberEntry::MethodHeader {
                    method_idx,
                    access_flags,
                    declaration,
                    ..
                } = entry
                {
                    declaration?;
                    if method_idx == method_id {
                        let class = self.class(class_id)?;
                        let context = self.class_context(class_id)?;
                        found = Some(LocalMethod {
                            access_flags,
                            owner_access_flags: class.access_flags,
                            parent: context.superclass?,
                            plain_scope: self.no_class_annotations(context.annotations_offset)?
                                && context.interfaces?.is_empty(),
                        });
                    }
                }
                Ok(())
            },
        )?;
        Ok(found)
    }

    pub fn source_file(&self, id: ClassDefId) -> Result<Option<DexString<'a>>, InputError> {
        match self.declarations.class_record(id)?.source_file {
            u32::MAX => Ok(None),
            id => self.string(StringId(id)).map(Some),
        }
    }

    pub(super) fn class_annotations_offset(
        &self,
        id: ClassDefId,
    ) -> Result<ByteOffset, InputError> {
        let record = self.declarations.class_record(id)?;
        let at = self.table(TableKey::Class(id))?.position_byte();
        file_offset(record.annotations, ByteOffset(at.0 + 20))
    }

    pub fn class_context(&self, id: ClassDefId) -> Result<ClassContext<'a>, InputError> {
        let record = self.declarations.class_record(id)?;
        let super_idx = record.superclass;
        let interfaces_raw = record.interfaces;
        let annotations_raw = record.annotations;
        let values_raw = record.values;
        let superclass = local(self.superclass(super_idx))?;
        let interfaces = local(self.interfaces(interfaces_raw))?;
        let values_offset = local(self.values_offset(values_raw))?;
        Ok(ClassContext {
            superclass_type: (super_idx != u32::MAX).then_some(TypeId(super_idx)),
            superclass,
            interfaces,
            values_offset,
            annotations_offset: file_offset(annotations_raw, self.header.class_defs.offset_byte)?,
        })
    }

    fn superclass(&self, index: u32) -> Result<Option<DexString<'a>>, InputError> {
        self.declarations.superclass(index)
    }

    fn interfaces(&self, raw: u32) -> Result<&'a [u8], InputError> {
        let offset = file_offset(raw, self.header.class_defs.offset_byte)?;
        if offset == ByteOffset(0) {
            return Ok(&self.bytes[0..0]);
        }
        let mut list = self.item(0x1001, offset)?;
        let count = list.read_u32()?;
        let range = list.skip(count_usize(count, offset)?, ByteLength(2))?;

        let bytes = &self.bytes[range.as_range()];
        for word in bytes.as_chunks::<2>().0 {
            let (_, shape) = self.type_descriptor(TypeId(u32::from(u16::from_le_bytes(*word))))?;
            if !matches!(shape, TypeShape::Class) {
                return Err(malformed(InputProblem::InvalidDescriptor, offset));
            }
        }
        Ok(bytes)
    }

    fn values_offset(&self, raw: u32) -> Result<Option<ByteOffset>, InputError> {
        let offset = file_offset(raw, self.header.class_defs.offset_byte)?;
        if offset == ByteOffset(0) {
            return Ok(None);
        }
        self.item(0x2005, offset)?;
        Ok(Some(offset))
    }

    pub fn method_reference(&self, id: MethodId) -> Result<MethodReference<'_, 'a>, InputError> {
        let mut record = self.table(TableKey::Method(id))?;
        let owner_idx = TypeId(u32::from(record.read_u16()?));
        let (owner, shape) = self.type_descriptor(owner_idx)?;
        if !matches!(shape, TypeShape::Class | TypeShape::Array) {
            return Err(malformed(
                InputProblem::InvalidDescriptor,
                record.position_byte(),
            ));
        }
        Ok(MethodReference {
            owner,
            owner_type: owner_idx,
            declaration: self.method_declaration(id, owner_idx)?,
        })
    }

    pub fn field_reference(&self, id: FieldId) -> Result<FieldReference<'a>, InputError> {
        let mut record = self.table(TableKey::Field(id))?;
        let owner_idx = TypeId(u32::from(record.read_u16()?));
        let (owner, shape) = self.type_descriptor(owner_idx)?;
        if !matches!(shape, TypeShape::Class) {
            return Err(malformed(
                InputProblem::InvalidDescriptor,
                record.position_byte(),
            ));
        }
        Ok(FieldReference {
            owner,
            owner_type: owner_idx,
            declaration: self.validate_field(id, owner_idx)?,
        })
    }
}

/// Original declared type relations of one class definition.
#[derive(Debug)]
pub struct ClassHierarchy<'a> {
    pub superclass: Option<DexString<'a>>,
    pub interfaces: Vec<DexString<'a>>,
}

/// Resolved facts of one original operand reference. Only the four xref target
/// families and prototypes are resolved; a call-site or method-handle slot keeps
/// its unsupported form rather than being expanded into members.
pub enum ReferenceFacts<'i, 'a> {
    String(DexString<'a>),
    Type(DexString<'a>),
    Method {
        owner: DexString<'a>,
        name: DexString<'a>,
        prototype: Prototype<'i, 'a>,
    },
    Field {
        owner: DexString<'a>,
        name: DexString<'a>,
        field_type: DexString<'a>,
    },
    Proto(Prototype<'i, 'a>),
    Unsupported,
}

impl std::fmt::Debug for ReferenceFacts<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::String(_) => "ReferenceFacts::String",
            Self::Type(_) => "ReferenceFacts::Type",
            Self::Method { .. } => "ReferenceFacts::Method",
            Self::Field { .. } => "ReferenceFacts::Field",
            Self::Proto(_) => "ReferenceFacts::Proto",
            Self::Unsupported => "ReferenceFacts::Unsupported",
        })
    }
}

impl<'a> ProgramIndex<'a> {
    /// Resolves one original operand reference to displayable facts. This is a
    /// read-only table query; it starts no analysis and returns `Unsupported` for
    /// reference families outside the xref targets.
    pub fn reference_facts(
        &self,
        reference: crate::raw::OperandReference,
    ) -> Result<ReferenceFacts<'_, 'a>, InputError> {
        use crate::raw::OperandReference;
        Ok(match reference {
            OperandReference::String(id) => ReferenceFacts::String(self.string(id)?),
            OperandReference::Type(id) => ReferenceFacts::Type(self.type_descriptor(id)?.0),
            OperandReference::Method(id) => {
                let method = self.method_reference(id)?;
                ReferenceFacts::Method {
                    owner: method.owner,
                    name: method.declaration.name,
                    prototype: method.declaration.prototype,
                }
            }
            OperandReference::Field(id) => {
                let field = self.field_reference(id)?;
                ReferenceFacts::Field {
                    owner: field.owner,
                    name: field.declaration.name,
                    field_type: field.declaration.field_type,
                }
            }
            OperandReference::Proto(id) => ReferenceFacts::Proto(self.prototype(id)?),
            OperandReference::CallSite(_) | OperandReference::MethodHandle(_) => {
                ReferenceFacts::Unsupported
            }
        })
    }
}

pub struct ClassContext<'a> {
    pub superclass_type: Option<TypeId>,
    pub superclass: Result<Option<DexString<'a>>, InputError>,
    pub interfaces: Result<&'a [u8], InputError>,
    pub values_offset: Result<Option<ByteOffset>, InputError>,
    pub annotations_offset: ByteOffset,
}

#[allow(dead_code)]
pub struct LocalMethod<'a> {
    pub access_flags: u32,
    pub owner_access_flags: u32,
    pub parent: Option<DexString<'a>>,
    pub plain_scope: bool,
}
pub struct MethodReference<'i, 'a> {
    pub owner: DexString<'a>,
    pub owner_type: TypeId,
    pub declaration: MethodDeclaration<'i, 'a>,
}

pub struct FieldReference<'a> {
    pub owner: DexString<'a>,
    pub owner_type: TypeId,
    pub declaration: FieldDeclaration<'a>,
}

pub struct ClassView<'i, 'a> {
    index: &'i ProgramIndex<'a>,
    pub class_def_idx: ClassDefId,
    pub type_idx: TypeId,
    pub access_flags: u32,
    pub descriptor: DexString<'a>,
    pub(super) class_data_offset: Option<ByteOffset>,
}

pub struct MethodDeclaration<'i, 'a> {
    pub name: DexString<'a>,
    pub prototype: Prototype<'i, 'a>,
}

/// Established prototype facts, owned by the logical index rather than by each
/// referencing method. Descriptors borrow the original bytes; no output is cached.
struct PrototypeFacts<'a> {
    shorty: DexString<'a>,
    return_type: DexString<'a>,
    return_type_idx: TypeId,
    parameters: &'a [u8],
    descriptors: Vec<DexString<'a>>,
    parameter_words: RegisterWordCount,
}

pub struct Prototype<'i, 'a> {
    parameter_descriptors: &'i [DexString<'a>],
    pub shorty: DexString<'a>,
    pub return_type: DexString<'a>,
    pub return_type_idx: TypeId,
    pub(super) parameters: &'a [u8],
    pub parameter_words: RegisterWordCount,
}

impl<'i, 'a> Prototype<'i, 'a> {
    /// Original parameter descriptors, resolved once in the logical index.
    pub fn parameter_descriptors(&self) -> &'i [DexString<'a>] {
        self.parameter_descriptors
    }

    pub fn visit_parameter_type_ids<E: From<InputError>>(
        &self,
        mut visit: impl FnMut(TypeId, DexString<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        for (word, &name) in self
            .parameters
            .as_chunks::<2>()
            .0
            .iter()
            .zip(self.parameter_descriptors)
        {
            let id = TypeId(u32::from(u16::from_le_bytes(*word)));
            visit(id, name)?;
        }
        Ok(())
    }
    pub fn visit_parameter_types<E: From<InputError>>(
        &self,
        mut visit: impl FnMut(DexString<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        self.visit_parameter_type_ids(|_, name| visit(name))
    }

    /// Visits the original parameter descriptors in declaration order.
    pub fn visit_parameters(
        &self,
        visit: impl FnMut(DexString<'a>) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        self.visit_parameter_types(visit)
    }

    /// Compares a complete DEX descriptor, including the return type.
    pub fn matches(&self, descriptor: &[u16]) -> Result<bool, InputError> {
        if descriptor.first() != Some(&u16::from(b'(')) {
            return Ok(false);
        }
        let mut position = Utf16Offset(1);
        for name in self.parameter_descriptors {
            let Some(next) = name.matches_prefix(descriptor, position)? else {
                return Ok(false);
            };
            position = next;
        }
        if descriptor.get(position.0) != Some(&u16::from(b')')) {
            return Ok(false);
        }
        Ok(self
            .return_type
            .matches_prefix(descriptor, Utf16Offset(position.0 + 1))?
            == Some(Utf16Offset(descriptor.len())))
    }
}

pub struct FieldDeclaration<'a> {
    pub name: DexString<'a>,
    pub field_type: DexString<'a>,
    pub field_type_idx: TypeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldStorage {
    Static { ordinal: u32 },
    Instance,
}

/// Original class-data method group. It comes from the encoded group, never from
/// `access_flags`; `ordinal` is the member's position inside that group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodGroup {
    Direct { ordinal: u32 },
    Virtual { ordinal: u32 },
}

/// The four original `class_data` group counts in encoded order. They describe
/// what the header declares, not how many members were read successfully.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClassDataShape {
    pub static_fields: u32,
    pub instance_fields: u32,
    pub direct_methods: u32,
    pub virtual_methods: u32,
}

impl ClassDataShape {
    pub fn declared_members(&self) -> u64 {
        u64::from(self.static_fields)
            + u64::from(self.instance_fields)
            + u64::from(self.direct_methods)
            + u64::from(self.virtual_methods)
    }
}

pub struct FieldEntry<'a> {
    pub field_idx: FieldId,
    pub access_flags: u32,
    pub storage: FieldStorage,
    /// Position inside the original static or instance group.
    pub ordinal: u32,
    pub declaration: Result<FieldDeclaration<'a>, InputError>,
}

pub enum MemberSelection<'f, 'a> {
    AllMethods,
    /// Method declarations only; field records establish cursor/identity bounds
    /// but do not resolve unrelated field names or types.
    MethodHeaders(Option<&'f mut dyn FnMut(StringId, DexString<'a>) -> Result<bool, InputError>>),
    /// Every field and method header in original group order, without reading code.
    AllMembers,
    Fields,
    /// Method identities and code offsets only: no declaration, no code read.
    MethodCodes,
}

pub enum MemberEntry<'i, 'a> {
    Field(FieldEntry<'a>),
    Method(MethodEntry<'i, 'a>),
    MethodHeader {
        method_idx: MethodId,
        access_flags: u32,
        group: MethodGroup,
        code_offset: Option<ByteOffset>,
        declaration: Result<MethodDeclaration<'i, 'a>, InputError>,
    },
    MethodCode {
        method_idx: MethodId,
        access_flags: u32,
        code_offset: Option<ByteOffset>,
    },
}

pub struct MethodEntry<'i, 'a> {
    pub method_idx: MethodId,
    pub access_flags: u32,
    pub declaration: Result<MethodDeclaration<'i, 'a>, InputError>,
    pub body: Result<super::MethodBody<'a>, InputError>,
}

impl<'i, 'a> ClassView<'i, 'a> {
    /// Read-only original type relations of this definition: the declared
    /// superclass and the interface list in their original order. It describes
    /// direct edges only and performs no closure or hierarchy analysis.
    pub fn hierarchy(&self) -> Result<ClassHierarchy<'a>, InputError> {
        let context = self.index.class_context(self.class_def_idx)?;
        let superclass = context.superclass?;
        let mut interfaces = Vec::new();
        for word in context.interfaces?.as_chunks::<2>().0 {
            let id = TypeId(u32::from(u16::from_le_bytes(*word)));
            interfaces
                .try_reserve(1)
                .map_err(|_| InputError::Allocation {
                    offset_byte: self.index.header.class_defs.offset_byte,
                })?;
            interfaces.push(self.index.type_descriptor(id)?.0);
        }
        Ok(ClassHierarchy {
            superclass,
            interfaces,
        })
    }

    /// Visits original field definitions only, in class-data order. External
    /// ``field_ids`` references are not definitions. A boundary error returns
    /// after the delivered prefix; callers retain that prefix.
    pub fn visit_fields(
        &self,
        mut visit: impl FnMut(FieldEntry<'a>) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        self.visit_members_with(MemberSelection::Fields, |entry| match entry {
            MemberEntry::Field(field) => visit(field),
            _ => Err(InputError::InternalInvariant(
                "field traversal delivered a method",
            )),
        })
    }

    /// Visits each method this class defines without reading its code.
    ///
    /// A listing needs the original index, access flags and declaration of every
    /// method; reading each `code_item` to answer that costs more than the listing
    /// itself on a large APK, so callers that do not analyze bodies use this
    /// traversal instead of [`Self::visit_methods`]. An optional name predicate
    /// sees the original string ID and checked name before prototype resolution.
    /// Only accepted names acquire prototypes; name/identity failures remain local
    /// declaration errors. The predicate does not change class-data coverage checks.
    pub fn visit_method_headers(
        &self,
        name_filter: Option<&mut dyn FnMut(StringId, DexString<'a>) -> Result<bool, InputError>>,
        mut visit: impl FnMut(
            crate::MethodId,
            u32,
            Result<&MethodDeclaration<'i, 'a>, InputError>,
        ) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        self.visit_members_with(
            MemberSelection::MethodHeaders(name_filter),
            |entry| match entry {
                MemberEntry::Field(_) => Err(InputError::InternalInvariant(
                    "method-header traversal delivered a field",
                )),
                MemberEntry::MethodHeader {
                    method_idx,
                    access_flags,
                    declaration,
                    ..
                } => match declaration {
                    Ok(declaration) => visit(method_idx, access_flags, Ok(&declaration)),
                    Err(error) => visit(method_idx, access_flags, Err(error)),
                },
                MemberEntry::Method(_) | MemberEntry::MethodCode { .. } => {
                    Err(InputError::InternalInvariant(
                        "code visitation reached a header-only selection",
                    ))
                }
            },
        )
    }

    /// Per-member failures retain their original IDs. A class-data boundary
    /// error returns after the delivered prefix; callers retain that prefix.
    pub fn visit_methods(
        &self,
        mut visit: impl FnMut(MethodEntry<'i, 'a>) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        self.visit_members_with(MemberSelection::AllMethods, |entry| match entry {
            MemberEntry::Method(method) => visit(method),
            MemberEntry::Field(_) => Ok(()),
            MemberEntry::MethodHeader { .. } | MemberEntry::MethodCode { .. } => Err(
                InputError::InternalInvariant("header-only selection reached code visitation"),
            ),
        })
    }

    /// Visits each method identity and code offset without resolving its
    /// declaration. Reference scans use this to skip bodies before paying for
    /// names and prototypes.
    pub fn visit_method_codes(
        &self,
        mut visit: impl FnMut(crate::MethodId, u32, Option<ByteOffset>) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        self.visit_members_with(MemberSelection::MethodCodes, |entry| match entry {
            MemberEntry::MethodCode {
                method_idx,
                access_flags,
                code_offset,
            } => visit(method_idx, access_flags, code_offset),
            _ => Err(InputError::InternalInvariant(
                "method-code traversal delivered another entry",
            )),
        })
    }

    /// Enumerates this definition's original members and reports how far the
    /// class-data traversal reached. A record whose index/identity cannot be
    /// established stops coverage without discarding the delivered prefix or any
    /// sibling declaration; per-declaration read failures stay in each entry.
    pub fn walk_members_with<E: From<InputError>>(
        &self,
        mut selection: MemberSelection<'_, 'a>,
        mut visit: impl FnMut(MemberEntry<'i, 'a>) -> Result<(), E>,
    ) -> Result<ClassDataWalk, E> {
        let Some(offset) = self.class_data_offset else {
            return Ok(ClassDataWalk::default());
        };
        let mut cursor = self.index.item(0x2000, offset)?;
        let mut counts = [0u32; 4];
        for count in &mut counts {
            *count = cursor.read_uleb32()?;
        }
        let minimum = u64::from(counts[0]) * 2
            + u64::from(counts[1]) * 2
            + u64::from(counts[2]) * 3
            + u64::from(counts[3]) * 3;
        if minimum > cursor.remaining_byte().0 as u64 {
            return Err(malformed(InputProblem::Truncated, cursor.position_byte()).into());
        }
        let mut traversal_error = None;
        let mut terminated_at = None;
        let mut delivered = [0u32; 4];
        'directory: for (groups, kind) in [
            ([counts[0], counts[1]], MemberKind::Field),
            ([counts[2], counts[3]], MemberKind::Method),
        ] {
            if matches!(selection, MemberSelection::Fields) && matches!(kind, MemberKind::Method) {
                break;
            }
            if groups == [0, 0] {
                continue;
            }
            let table_count = match kind {
                MemberKind::Field => self.index.header.field_ids.count,
                MemberKind::Method => self.index.header.method_ids.count,
            };
            let mut prior = PriorState::Comparable(PriorGroup {
                cursor: self.index.item(0x2000, cursor.position_byte())?,
                remaining: groups[0],
                next: None,
                index: 0,
                kind,
            });
            for (group, count) in groups.into_iter().enumerate() {
                let mut previous = Some(0u32);
                for item in 0..count {
                    let record = cursor.position_byte();
                    let header = (|| -> Result<_, InputError> {
                        let delta = cursor.read_uleb32()?;
                        let access_flags = cursor.read_uleb32()?;
                        let member = match kind {
                            MemberKind::Field => EncodedMember::Field,
                            MemberKind::Method => {
                                let offset = file_offset(cursor.read_uleb32()?, record)?;
                                EncodedMember::Method {
                                    code_offset: (offset != ByteOffset(0)).then_some(offset),
                                }
                            }
                        };
                        Ok((delta, access_flags, member))
                    })();
                    let (delta, access_flags, member) = match header {
                        Ok(v) => v,
                        Err(e)
                            if matches!(
                                e.kind(),
                                crate::FailureKind::Malformed | crate::FailureKind::Unsupported
                            ) =>
                        {
                            traversal_error.get_or_insert(e);
                            terminated_at.get_or_insert(record);
                            // No next record boundary is established. Keep every
                            // delivered member, but never scan beyond this suffix.
                            break 'directory;
                        }
                        Err(e) => return Err(e.into()),
                    };
                    let Some(index) = previous.and_then(|value| value.checked_add(delta)) else {
                        traversal_error
                            .get_or_insert(malformed(InputProblem::RangeOverflow, record));
                        terminated_at.get_or_insert(record);
                        previous = None;
                        if group == 0 {
                            prior = PriorState::Invalid;
                        }
                        continue;
                    };
                    previous = Some(index);
                    let invalid_index = index >= table_count || (item != 0 && delta == 0);
                    let duplicate = if !invalid_index && group == 1 {
                        match &mut prior {
                            PriorState::Comparable(prior) => prior.contains(index)?,
                            PriorState::Invalid => false,
                        }
                    } else {
                        false
                    };
                    if invalid_index || duplicate {
                        // The record extent is known, but this member has no usable
                        // identity. Coverage stays incomplete; the delivered prefix
                        // and every independently valid sibling remain available.
                        traversal_error
                            .get_or_insert(malformed(InputProblem::InvalidClassData, record));
                        terminated_at.get_or_insert(record);
                        if group == 0 {
                            prior = PriorState::Invalid;
                        }
                        continue;
                    }
                    match member {
                        EncodedMember::Field => {
                            if matches!(
                                selection,
                                MemberSelection::AllMethods
                                    | MemberSelection::MethodHeaders(_)
                                    | MemberSelection::MethodCodes
                            ) {
                                continue;
                            }
                            let declaration =
                                local(self.index.validate_field(FieldId(index), self.type_idx))?;
                            let group_slot = match kind {
                                MemberKind::Field => group,
                                MemberKind::Method => group + 2,
                            };
                            delivered[group_slot] = delivered[group_slot].saturating_add(1);
                            visit(MemberEntry::Field(FieldEntry {
                                field_idx: FieldId(index),
                                access_flags,
                                storage: if group == 0 {
                                    FieldStorage::Static { ordinal: item }
                                } else {
                                    FieldStorage::Instance
                                },
                                ordinal: item,
                                declaration,
                            }))?;
                        }
                        EncodedMember::Method { code_offset } => {
                            if matches!(selection, MemberSelection::MethodCodes) {
                                let group_slot = match kind {
                                    MemberKind::Field => group,
                                    MemberKind::Method => group + 2,
                                };
                                delivered[group_slot] = delivered[group_slot].saturating_add(1);
                                visit(MemberEntry::MethodCode {
                                    method_idx: MethodId(index),
                                    access_flags,
                                    code_offset,
                                })?;
                                continue;
                            }
                            let declaration = match &mut selection {
                                MemberSelection::MethodHeaders(Some(accept)) => {
                                    local(self.index.method_declaration_if(
                                        MethodId(index),
                                        self.type_idx,
                                        &mut **accept,
                                    ))?
                                }
                                _ => local(
                                    self.index
                                        .method_declaration(MethodId(index), self.type_idx)
                                        .map(Some),
                                )?,
                            };
                            let declaration = match declaration {
                                Ok(Some(declaration)) => Ok(declaration),
                                Ok(None) => continue,
                                Err(error) => Err(error),
                            };
                            if matches!(
                                selection,
                                MemberSelection::AllMembers | MemberSelection::MethodHeaders(_)
                            ) {
                                let group_slot = match kind {
                                    MemberKind::Field => group,
                                    MemberKind::Method => group + 2,
                                };
                                delivered[group_slot] = delivered[group_slot].saturating_add(1);
                                visit(MemberEntry::MethodHeader {
                                    method_idx: MethodId(index),
                                    access_flags,
                                    group: if group == 0 {
                                        MethodGroup::Direct { ordinal: item }
                                    } else {
                                        MethodGroup::Virtual { ordinal: item }
                                    },
                                    code_offset,
                                    declaration,
                                })?;
                                continue;
                            }
                            let words = declaration.as_ref().ok().map(|declaration| {
                                RegisterWordCount(
                                    declaration.prototype.parameter_words.0
                                        + u64::from(
                                            access_flags & crate::substrate::flags::STATIC == 0,
                                        ),
                                )
                            });
                            let body = local(read_code(
                                self.index,
                                code_offset,
                                MethodId(index),
                                access_flags,
                                words,
                            ))?;
                            visit(MemberEntry::Method(MethodEntry {
                                method_idx: MethodId(index),
                                access_flags,
                                declaration,
                                body,
                            }))?;
                        }
                    }
                }
            }
        }
        Ok(ClassDataWalk {
            shape: ClassDataShape {
                static_fields: counts[0],
                instance_fields: counts[1],
                direct_methods: counts[2],
                virtual_methods: counts[3],
            },
            delivered,
            terminated_at,
            issue: traversal_error,
        })
    }

    pub fn visit_members_with<E: From<InputError>>(
        &self,
        selection: MemberSelection<'_, 'a>,
        visit: impl FnMut(MemberEntry<'i, 'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        let walk = self.walk_members_with(selection, visit)?;
        match walk.issue {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}

/// How far one original class-data traversal reached, together with the declared
/// four-group counts. `issue` reports unusable coverage, never a declaration read.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClassDataWalk {
    pub shape: ClassDataShape,
    /// Members actually delivered in each original group, including entries whose
    /// own declaration read failed. A prefix shorter than `shape` is explicit
    /// coverage loss, not an empty group.
    pub delivered: [u32; 4],
    /// First original class-data byte whose record could not be located.
    pub terminated_at: Option<ByteOffset>,
    pub issue: Option<InputError>,
}

#[derive(Clone, Copy)]
enum MemberKind {
    Field,
    Method,
}

enum EncodedMember {
    Field,
    Method { code_offset: Option<ByteOffset> },
}

enum PriorState<'a> {
    Comparable(PriorGroup<'a>),
    Invalid,
}

struct PriorGroup<'a> {
    cursor: Cursor<'a>,
    remaining: u32,
    next: Option<u32>,
    index: u32,
    kind: MemberKind,
}

impl PriorGroup<'_> {
    // Sorted group replay detects cross-group duplicates in linear time.
    fn contains(&mut self, target: u32) -> Result<bool, InputError> {
        loop {
            if let Some(next) = self.next {
                if next >= target {
                    return Ok(next == target);
                }
                self.next = None;
            }
            if self.remaining == 0 {
                return Ok(false);
            }
            self.index = self
                .index
                .checked_add(self.cursor.read_uleb32()?)
                .ok_or_else(|| {
                    malformed(InputProblem::RangeOverflow, self.cursor.position_byte())
                })?;
            self.cursor.read_uleb32()?;
            if matches!(self.kind, MemberKind::Method) {
                self.cursor.read_uleb32()?;
            }
            self.remaining -= 1;
            self.next = Some(self.index);
        }
    }
}

pub(super) fn local<T>(result: Result<T, InputError>) -> Result<Result<T, InputError>, InputError> {
    match result {
        Err(error)
            if !matches!(
                error.kind(),
                FailureKind::Malformed | FailureKind::Unsupported
            ) =>
        {
            Err(error)
        }
        result => Ok(result),
    }
}
