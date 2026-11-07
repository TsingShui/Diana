use diana_decomp::input::{ClassView, MethodBody, ProgramIndex, Prototype, ReferenceFacts};
use diana_decomp::raw::LayoutKind;
use diana_decomp::{ClassDefId, MethodId};

use super::apk::DexRoot;
use super::dex::{self, ImageIndex};
pub use super::filter::TextFilter as TextMatch;
use super::filter::{self, FilterMatch, Relevance};
use super::images::LogicalDex;
use super::input::InputError;
use super::query::{ImageInfo, QueryError};
use super::request::RequestScope;

mod coordinate;
pub use coordinate::resolve_coordinate;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Relation {
    /// The original superclass reference.
    Extend,
    /// A reference in the original direct-interface list, including subinterfaces.
    Impl,
}

/// A reference predicate. Non-exact searches may omit owner/signature
/// constraints; a supplied prototype or field type remains exact.
#[derive(Clone, Debug)]
pub enum Target {
    String(TextMatch),
    Type(TextMatch),
    Method {
        owner: Option<TextMatch>,
        name: TextMatch,
        prototype: Option<Vec<u16>>,
        /// Presentation relevance carried by exact upstream caller expansion.
        relevance: Option<Relevance>,
    },
    Field {
        owner: Option<TextMatch>,
        name: TextMatch,
        field_type: Option<Vec<u16>>,
    },
    Hierarchy {
        relation: Relation,
        target: TextMatch,
    },
}

impl Target {
    pub fn is_ranked(&self) -> bool {
        match self {
            Self::String(predicate)
            | Self::Type(predicate)
            | Self::Hierarchy {
                target: predicate, ..
            } => predicate.is_fuzzy(),
            Self::Method {
                owner,
                name,
                relevance,
                ..
            } => {
                relevance.is_some()
                    || name.is_fuzzy()
                    || owner.as_ref().is_some_and(TextMatch::is_fuzzy)
            }
            Self::Field { owner, name, .. } => {
                name.is_fuzzy() || owner.as_ref().is_some_and(TextMatch::is_fuzzy)
            }
        }
    }
}

/// The original referenced value actually found at one site, never the pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Matched {
    String(Vec<u16>),
    Type(Vec<u16>),
    Method {
        owner: Vec<u16>,
        name: Vec<u16>,
        prototype: Vec<u16>,
    },
    Field {
        owner: Vec<u16>,
        name: Vec<u16>,
        field_type: Vec<u16>,
    },
}

/// Declaration relations have a class locator, not an invented method or offset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Site {
    Class,
    Instruction {
        method: MethodId,
        name: Vec<u16>,
        prototype: Vec<u16>,
        code_unit: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XrefRow {
    pub relevance: Option<Relevance>,
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    pub class_descriptor: Vec<u16>,
    pub site: Site,
    /// One for the queried target, two for a caller of a caller.
    pub depth: u32,
    /// Original mnemonic, or the direct declaration relation.
    pub use_: &'static str,
    pub matched: Matched,
}

impl XrefRow {
    /// Original caller key for deduplicating the next scan's targets, not rows.
    pub fn caller_key(&self) -> Option<(&[u16], &[u16], &[u16])> {
        match &self.site {
            Site::Instruction {
                name, prototype, ..
            } => Some((&self.class_descriptor, name, prototype)),
            Site::Class => None,
        }
    }

    /// Caller expansion always uses the complete original identity.
    pub fn caller(&self) -> Result<Option<Target>, QueryError> {
        let Some((owner, name, prototype)) = self.caller_key() else {
            return Ok(None);
        };
        Ok(Some(Target::Method {
            owner: Some(TextMatch::Exact(copy(owner)?)),
            name: TextMatch::Exact(copy(name)?),
            prototype: Some(copy(prototype)?),
            relevance: self.relevance,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct XrefPlan {
    /// One initial query, or all exact callers reached at the previous depth.
    pub targets: Vec<Target>,
    /// Exact class descriptor restricting the referencing definition.
    pub class: Option<Vec<u16>>,
    pub depth: u32,
    pub jobs: usize,
}

/// Prepared reference scan over the root images of one leased input.
pub struct Xref<'a> {
    roots: Vec<DexRoot<'a>>,
    images: Vec<ImageInfo>,
    plan: XrefPlan,
}

impl<'a> Xref<'a> {
    pub fn new(scope: &'a RequestScope, plan: XrefPlan) -> Result<Self, QueryError> {
        if plan.jobs == 0 {
            return Err(QueryError::InvalidJobs);
        }
        let roots = scope.roots()?;
        let mut images = Vec::new();
        images
            .try_reserve_exact(roots.len())
            .map_err(InputError::Allocation)?;
        for (ordinal, root) in roots.iter().enumerate() {
            images.push(ImageInfo {
                index: ImageIndex(ordinal),
                label: root.label().to_owned(),
                entry: root.entry().map(|entry| entry.index()),
            });
        }
        Ok(Self {
            roots,
            images,
            plan,
        })
    }

    pub fn images(&self) -> &[ImageInfo] {
        &self.images
    }

    /// Streams every matching site in completion order. Instruction sites are
    /// never merged; hierarchy queries read declarations without opening bodies.
    /// Reader, allocation and sink failures retain their classified errors.
    pub fn run(
        &self,
        sink: &mut impl FnMut(XrefRow) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        super::runner::run(
            self.roots.len(),
            self.plan.jobs,
            || (),
            |(), ordinal, emit| {
                let root = &self.roots[ordinal];
                dex::visit_logical(root.bytes()?, |logical, index| {
                    scan_logical(&index, ImageIndex(ordinal), logical, &self.plan, emit)
                })
            },
            sink,
        )
    }
}

fn scan_logical(
    index: &ProgramIndex<'_>,
    image: ImageIndex,
    logical: LogicalDex,
    plan: &XrefPlan,
    emit: &mut dyn FnMut(XrefRow) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut failure = None;
    let result = index.visit_classes(|class_def, class| {
        let result = class
            .map_err(QueryError::from)
            .and_then(|class| scan_class(index, &class, image, logical, class_def, plan, emit));
        if let Err(error) = result {
            failure = Some(error);
            return Err(diana_decomp::input::InputError::InternalInvariant(
                "xref scan stopped",
            ));
        }
        Ok(())
    });
    match failure {
        Some(error) => Err(error),
        None => result.map_err(QueryError::from),
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_class(
    index: &ProgramIndex<'_>,
    class: &ClassView<'_, '_>,
    image: ImageIndex,
    logical: LogicalDex,
    class_def: ClassDefId,
    plan: &XrefPlan,
    emit: &mut dyn FnMut(XrefRow) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if let Some(target) = plan.class.as_deref()
        && !class.descriptor.matches(target)?
    {
        return Ok(());
    }
    let class_descriptor = class.descriptor.units()?;
    if plan
        .targets
        .iter()
        .any(|target| matches!(target, Target::Hierarchy { .. }))
    {
        let hierarchy = class.hierarchy()?;
        for target in &plan.targets {
            let Target::Hierarchy { relation, target } = target else {
                continue;
            };
            let (references, use_) = match relation {
                Relation::Extend => (hierarchy.superclass.as_slice(), "extends"),
                Relation::Impl => (
                    hierarchy.interfaces.as_slice(),
                    if class.access_flags & 0x200 != 0 {
                        "extends-interface"
                    } else {
                        "implements"
                    },
                ),
            };
            for &reference in references {
                if let Some(selected) = target.evaluate(reference)? {
                    emit(XrefRow {
                        image,
                        logical,
                        class_def,
                        class_descriptor: copy(&class_descriptor)?,
                        site: Site::Class,
                        relevance: selected.relevance,
                        depth: plan.depth,
                        use_,
                        matched: Matched::Type(reference.units()?),
                    })?;
                }
            }
        }
    }
    if plan
        .targets
        .iter()
        .all(|target| matches!(target, Target::Hierarchy { .. }))
    {
        return Ok(());
    }
    let mut failure = None;
    let result = class.visit_methods(|method| {
        let result = (|| -> Result<(), QueryError> {
            let declaration = method.declaration?;
            let view = match method.body? {
                MethodBody::Code(view) => view,
                MethodBody::DeclarationOnly => return Ok(()),
            };
            let mut sites = Vec::new();
            view.visit_layouts(|code_unit, layout| {
                let LayoutKind::Instruction(spec) = layout.kind() else {
                    return Ok(());
                };
                for reference in layout.references().into_iter().flatten() {
                    sites
                        .try_reserve(1)
                        .map_err(diana_decomp::input::MethodError::Allocation)?;
                    sites.push((code_unit, spec.mnemonic, reference.target));
                }
                Ok(())
            })?;
            // Caller names/prototypes are needed only for actual matching sites.
            let mut caller = None;
            for (code_unit, mnemonic, reference) in sites {
                let Some((matched, selected)) =
                    matched(index.reference_facts(reference)?, &plan.targets)?
                else {
                    continue;
                };
                if caller.is_none() {
                    caller = Some((
                        declaration.name.units()?,
                        prototype_units(&declaration.prototype)?,
                    ));
                }
                if let Some((name, prototype)) = &caller {
                    emit(XrefRow {
                        image,
                        logical,
                        class_def,
                        class_descriptor: copy(&class_descriptor)?,
                        relevance: selected.relevance,
                        site: Site::Instruction {
                            method: method.method_idx,
                            name: copy(name)?,
                            prototype: copy(prototype)?,
                            code_unit: code_unit.0,
                        },
                        depth: plan.depth,
                        use_: mnemonic,
                        matched,
                    })?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            failure = Some(error);
            return Err(diana_decomp::input::InputError::InternalInvariant(
                "xref method scan stopped",
            ));
        }
        Ok(())
    });
    match failure {
        Some(error) => Err(error),
        None => result.map_err(QueryError::from),
    }
}

fn copy(units: &[u16]) -> Result<Vec<u16>, QueryError> {
    let mut owned = Vec::new();
    append(&mut owned, units)?;
    Ok(owned)
}

fn append(out: &mut Vec<u16>, units: &[u16]) -> Result<(), QueryError> {
    out.try_reserve(units.len())
        .map_err(InputError::Allocation)?;
    out.extend_from_slice(units);
    Ok(())
}

fn prototype_units(prototype: &Prototype<'_, '_>) -> Result<Vec<u16>, QueryError> {
    let mut units = Vec::new();
    append(&mut units, &[u16::from(b'(')])?;
    for parameter in prototype.parameter_descriptors() {
        append(&mut units, &parameter.units()?)?;
    }
    append(&mut units, &[u16::from(b')')])?;
    append(&mut units, &prototype.return_type.units()?)?;
    Ok(units)
}

fn matched(
    facts: ReferenceFacts<'_, '_>,
    targets: &[Target],
) -> Result<Option<(Matched, FilterMatch)>, QueryError> {
    for target in targets {
        let found = match (&facts, target) {
            (ReferenceFacts::String(value), Target::String(predicate)) => {
                let Some(selected) = predicate.evaluate(*value)? else {
                    continue;
                };
                Some((Matched::String(value.units()?), selected))
            }
            (ReferenceFacts::Type(value), Target::Type(predicate)) => {
                let Some(selected) = predicate.evaluate(*value)? else {
                    continue;
                };
                Some((Matched::Type(value.units()?), selected))
            }
            (
                ReferenceFacts::Method {
                    owner,
                    name,
                    prototype,
                },
                Target::Method {
                    owner: owner_match,
                    name: name_match,
                    prototype: signature,
                    relevance,
                },
            ) => {
                let Some(owner_quality) = filter::select(owner_match.as_ref(), *owner)? else {
                    continue;
                };
                let Some(name_quality) = name_match.evaluate(*name)? else {
                    continue;
                };
                if let Some(signature) = signature
                    && !prototype.matches(signature)?
                {
                    continue;
                }
                let mut selected = owner_quality.combine(name_quality)?;
                if let Some(relevance) = relevance {
                    selected.relevance = Some(*relevance);
                }
                Some((
                    Matched::Method {
                        owner: owner.units()?,
                        name: name.units()?,
                        prototype: prototype_units(prototype)?,
                    },
                    selected,
                ))
            }
            (
                ReferenceFacts::Field {
                    owner,
                    name,
                    field_type,
                },
                Target::Field {
                    owner: owner_match,
                    name: name_match,
                    field_type: signature,
                },
            ) => {
                let Some(owner_quality) = filter::select(owner_match.as_ref(), *owner)? else {
                    continue;
                };
                let Some(name_quality) = name_match.evaluate(*name)? else {
                    continue;
                };
                if let Some(signature) = signature
                    && !field_type.matches(signature)?
                {
                    continue;
                }
                Some((
                    Matched::Field {
                        owner: owner.units()?,
                        name: name.units()?,
                        field_type: field_type.units()?,
                    },
                    owner_quality.combine(name_quality)?,
                ))
            }
            _ => None,
        };
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

/// Convenience for an exact `type` target expressed as a descriptor or dotted name.
pub fn type_target(text: &str) -> Target {
    Target::Type(TextMatch::Exact(super::dex::class_descriptor(text)))
}
