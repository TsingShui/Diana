use diana_decomp::input::{ClassView, DexString, FieldStorage, ProgramIndex};
use diana_decomp::{ClassDefId, FieldId, MethodId, StringId};

use super::images::LogicalDex;
use super::query::{QueryError, Row};

/// App-local order of the root images in one request. It is display metadata and
/// never decides original identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImageIndex(pub usize);

/// One `class_defs` definition with its original identity.
///
/// The descriptor stays borrowed: a class index is hundreds of thousands of rows,
/// and materializing its UTF-16 units per row was one allocation per class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassRow<'a> {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    pub descriptor: DexString<'a>,
    pub access_flags: u32,
}

/// One method actually defined by `class_data`; an external `method_ids`
/// reference is never a definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodRow<'a> {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    /// Original owner descriptor, needed for the canonical `at` locator.
    pub class_descriptor: DexString<'a>,
    pub method: MethodId,
    pub name: DexString<'a>,
    pub prototype: PrototypeRow<'a>,
    pub access_flags: u32,
}

/// A method's original prototype, kept as the descriptors it is made of.
///
/// Parameters borrow the logical index's resolved prototype facts. A referencing
/// method neither allocates a parameter vector nor resolves the same types again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrototypeRow<'a> {
    pub parameters: &'a [DexString<'a>],
    pub returns: DexString<'a>,
}

/// One field actually defined by `class_data`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRow<'a> {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    /// Original owner descriptor, needed for the canonical `at` locator.
    pub class_descriptor: DexString<'a>,
    pub field: FieldId,
    pub name: DexString<'a>,
    pub field_type: DexString<'a>,
    pub access_flags: u32,
    pub storage: FieldStorage,
}

/// One original `string_ids` value, independent of where it is used.
///
/// The value stays borrowed so a full string pool costs no per-row allocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringRow<'a> {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub string: StringId,
    pub value: DexString<'a>,
}

/// Definition kind restriction for `classes`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassKind {
    Class,
    Interface,
}

/// Direct hierarchy relations restricting `classes`. Every configured relation
/// must hold; none of them performs a transitive closure.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClassFilter {
    /// The definition's own superclass is this descriptor.
    pub extends: Option<Vec<u16>>,
    /// The definition is a class and lists this interface directly.
    pub implements: Option<Vec<u16>>,
    /// The definition is an interface and lists this interface directly.
    pub subinterface: Option<Vec<u16>>,
    pub kind: Option<ClassKind>,
}

impl ClassFilter {
    fn is_empty(&self) -> bool {
        self.extends.is_none()
            && self.implements.is_none()
            && self.subinterface.is_none()
            && self.kind.is_none()
    }

    fn admits(&self, class: &ClassView<'_, '_>) -> Result<bool, diana_decomp::input::InputError> {
        let interface = class.access_flags & 0x200 != 0;
        if let Some(kind) = self.kind
            && interface != (kind == ClassKind::Interface)
        {
            return Ok(false);
        }
        if self.is_empty() {
            return Ok(true);
        }
        let hierarchy = class.hierarchy()?;
        if let Some(target) = &self.extends
            && !matches!(&hierarchy.superclass, Some(parent) if parent.matches(target)?)
        {
            return Ok(false);
        }
        if let Some(target) = &self.implements {
            if interface {
                return Ok(false);
            }
            let mut listed = false;
            for name in &hierarchy.interfaces {
                listed |= name.matches(target)?;
            }
            if !listed {
                return Ok(false);
            }
        }
        if let Some(target) = &self.subinterface {
            if !interface {
                return Ok(false);
            }
            let mut listed = false;
            for name in &hierarchy.interfaces {
                listed |= name.matches(target)?;
            }
            if !listed {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Visits each checked logical header without changing physical-root worker
/// scheduling. The original container bytes remain the offset coordinate space.
pub(super) fn visit_logical(
    bytes: &[u8],
    mut visit: impl FnMut(LogicalDex, ProgramIndex<'_>) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let container = diana_decomp::input::DexContainer::open(bytes)?;
    for (ordinal, offset) in container.logical_headers()?.into_iter().enumerate() {
        let logical = LogicalDex(u32::try_from(ordinal).map_err(|_| {
            diana_decomp::input::InputError::PlatformCapacity {
                offset_byte: offset,
            }
        })?);
        visit(logical, container.program(offset)?)?;
    }
    Ok(())
}

pub(super) fn classes(
    index: &ProgramIndex<'_>,
    image: ImageIndex,
    logical: LogicalDex,
    filter: Option<&[u16]>,
    relation: &ClassFilter,
    emit: &mut dyn FnMut(Row) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut failure: Option<QueryError> = None;
    let result = {
        let mut forward = |row: Row| -> Result<(), diana_decomp::input::InputError> {
            if let Err(error) = emit(row) {
                failure = Some(error);
                return Err(diana_decomp::input::InputError::InternalInvariant(
                    "query sink stopped",
                ));
            }
            Ok(())
        };
        index.visit_classes(|class_def, class| {
            let class = class?;
            // A filter is the only consumer that needs UTF-16 units, so the common
            // unfiltered index pays no per-class allocation.
            if let Some(needle) = filter
                && !contains(&class.descriptor.units()?, needle)
            {
                return Ok(());
            }
            if !relation.admits(&class)? {
                return Ok(());
            }
            forward(Row::Class(ClassRow {
                image,
                logical,
                class_def,
                descriptor: class.descriptor,
                access_flags: class.access_flags,
            }))?;
            Ok(())
        })
    };
    match failure {
        Some(error) => Err(error),
        None => result.map_err(QueryError::from),
    }
}

pub(super) fn methods<'a>(
    index: &ProgramIndex<'a>,
    image: ImageIndex,
    logical: LogicalDex,
    class_filter: Option<&[u16]>,
    filter: Option<&[u16]>,
    emit: &mut dyn FnMut(Row) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut matching_names = Vec::new();
    if filter.is_some() {
        let count = usize::try_from(index.string_ids_len())
            .map_err(|_| super::input::InputError::PlatformCapacity)?;
        matching_names
            .try_reserve_exact(count)
            .map_err(super::input::InputError::Allocation)?;
        matching_names.resize(count, None);
    }
    let mut failure: Option<QueryError> = None;
    let result = {
        let mut forward = |row: Row| -> Result<(), diana_decomp::input::InputError> {
            if let Err(error) = emit(row) {
                failure = Some(error);
                return Err(diana_decomp::input::InputError::InternalInvariant(
                    "query sink stopped",
                ));
            }
            Ok(())
        };
        index.visit_classes(|class_def, class| {
            let class = class?;
            if let Some(target) = class_filter
                && !class.descriptor.matches(target)?
            {
                return Ok(());
            }
            // A name appears once in the pool but can be used by many definitions.
            // Cache only completed matches, scoped to this logical index/request.
            let mut accept = |id: StringId,
                              name: DexString<'a>|
             -> Result<bool, diana_decomp::input::InputError> {
                let Some(needle) = filter else {
                    return Ok(true);
                };
                let id = usize::try_from(id.0).map_err(|_| {
                    diana_decomp::input::InputError::PlatformCapacity {
                        offset_byte: diana_decomp::ByteOffset(0),
                    }
                })?;
                let slot = matching_names.get_mut(id).ok_or(
                    diana_decomp::input::InputError::InternalInvariant(
                        "method name cache lost an original string ID",
                    ),
                )?;
                if let Some(matched) = *slot {
                    return Ok(matched);
                }
                let matched = name.contains_ignore_case(needle)?;
                *slot = Some(matched);
                Ok(matched)
            };
            let name_filter = filter.map(|_| {
                &mut accept
                    as &mut dyn FnMut(
                        StringId,
                        DexString<'a>,
                    )
                        -> Result<bool, diana_decomp::input::InputError>
            });
            class.visit_method_headers(name_filter, |method_idx, access_flags, declaration| {
                let declaration = declaration?;
                let parameters = declaration.prototype.parameter_descriptors();
                forward(Row::Method(MethodRow {
                    image,
                    logical,
                    class_def,
                    class_descriptor: class.descriptor,
                    method: method_idx,
                    name: declaration.name,
                    prototype: PrototypeRow {
                        parameters,
                        returns: declaration.prototype.return_type,
                    },
                    access_flags,
                }))
            })
        })
    };
    match failure {
        Some(error) => Err(error),
        None => result.map_err(QueryError::from),
    }
}

pub(super) fn fields(
    index: &ProgramIndex<'_>,
    image: ImageIndex,
    logical: LogicalDex,
    class_filter: Option<&[u16]>,
    filter: Option<&[u16]>,
    emit: &mut dyn FnMut(Row) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut failure: Option<QueryError> = None;
    let result = {
        let mut forward = |row: Row| -> Result<(), diana_decomp::input::InputError> {
            if let Err(error) = emit(row) {
                failure = Some(error);
                return Err(diana_decomp::input::InputError::InternalInvariant(
                    "query sink stopped",
                ));
            }
            Ok(())
        };
        index.visit_classes(|class_def, class| {
            let class = class?;
            if let Some(target) = class_filter
                && !class.descriptor.matches(target)?
            {
                return Ok(());
            }
            class.visit_fields(|field| {
                let declaration = field.declaration?;
                if let Some(needle) = filter
                    && !declaration.name.contains_ignore_case(needle)?
                {
                    return Ok(());
                }
                forward(Row::Field(FieldRow {
                    image,
                    logical,
                    class_def,
                    class_descriptor: class.descriptor,
                    field: field.field_idx,
                    name: declaration.name,
                    field_type: declaration.field_type,
                    access_flags: field.access_flags,
                    storage: field.storage,
                }))
            })
        })
    };
    match failure {
        Some(error) => Err(error),
        None => result.map_err(QueryError::from),
    }
}

pub(super) fn strings(
    index: &ProgramIndex<'_>,
    image: ImageIndex,
    logical: LogicalDex,
    filter: Option<&[u16]>,
    emit: &mut dyn FnMut(Row) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    for ordinal in 0..index.string_ids_len() {
        let string = StringId(ordinal);
        let value = index.string(string)?;
        if let Some(needle) = filter
            && !value.contains_ignore_case(needle)?
        {
            continue;
        }
        emit(Row::String(StringRow {
            image,
            logical,
            string,
            value,
        }))?;
    }
    Ok(())
}

/// Converts a class name in dotted form to an original descriptor. A value that is
/// already a descriptor is kept, so a nested `$` name is not rewritten.
pub fn class_descriptor(text: &str) -> Vec<u16> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.first() == Some(&u16::from(b'L')) && units.last() == Some(&u16::from(b';')) {
        return units;
    }
    let mut descriptor = Vec::with_capacity(units.len() + 2);
    descriptor.push(u16::from(b'L'));
    for unit in units {
        descriptor.push(if unit == u16::from(b'.') {
            u16::from(b'/')
        } else {
            unit
        });
    }
    descriptor.push(u16::from(b';'));
    descriptor
}

/// Literal UTF-16 substring match over the primary column text.
pub fn contains(haystack: &[u16], needle: &[u16]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
