use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use diana_decomp::input::ProgramIndex;
use diana_decomp::raw::OperandReference;
use diana_decomp::{ClassDefId, MethodId};

use super::declarations::InputLocation;
use super::{FlatSite, Matched, QueryError, RowView, Site, XrefPlan, matched, prototype_units};
use crate::analysis::{dex::ImageIndex, filter::Relevance, images::LogicalDex, input::InputError};

#[derive(Clone, Copy, Debug)]
struct ContextId(u32);

#[derive(Clone, Copy, Debug)]
struct ReferenceId(u32);

/// A code site owns only coordinates into its batch's shared identity tables.
#[derive(Clone, Copy, Debug)]
struct Hit {
    context: ContextId,
    reference: ReferenceId,
    code_unit: u32,
}

#[derive(Clone, Debug)]
struct Reference {
    matched: Matched,
    relevance: Option<Relevance>,
    use_: String,
}

/// Hits retain the first occurrence of each referenced target in each method.
/// Identity tables are owned so workers can release their input scratch buffers.
#[derive(Clone, Debug)]
pub struct Batch {
    image: ImageIndex,
    logical: LogicalDex,
    class_def: ClassDefId,
    descriptor: Vec<u16>,
    depth: u32,
    contexts: Vec<Site>,
    references: Vec<Reference>,
    hits: Vec<Hit>,
}

impl Batch {
    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    pub fn row(&self, index: usize) -> RowView<'_> {
        let hit = self.hits[index];
        let reference = &self.references[hit.reference.0 as usize];
        RowView {
            image: self.image,
            logical: self.logical,
            class_def: self.class_def,
            class_descriptor: &self.descriptor,
            site: &self.contexts[hit.context.0 as usize],
            code_unit: Some(hit.code_unit),
            depth: self.depth,
            relevance: reference.relevance,
            use_: &reference.use_,
            matched: &reference.matched,
        }
    }
}

pub(super) struct Builder<'a> {
    descriptor: diana_decomp::input::DexString<'a>,
    batch: Batch,
    resolved: HashMap<(u8, u32, &'static str), Range<usize>>,
}

impl<'a> Builder<'a> {
    pub fn new(location: &InputLocation<'a>, depth: u32) -> Self {
        Self {
            descriptor: location.descriptor,
            batch: Batch {
                image: location.image,
                logical: location.logical,
                class_def: location.class_def,
                descriptor: Vec::new(),
                depth,
                contexts: Vec::new(),
                references: Vec::new(),
                hits: Vec::new(),
            },
            resolved: HashMap::new(),
        }
    }

    pub fn finish(self) -> Batch {
        self.batch
    }

    pub fn method(
        &mut self,
        index: &ProgramIndex<'_>,
        method: MethodId,
        sites: &[FlatSite],
        plan: &XrefPlan,
    ) -> Result<(), QueryError> {
        let mut context = None;
        // Store indices rather than cloning potentially large target identities.
        // The full identity comparison also handles hash collisions.
        let mut seen: HashMap<u64, Vec<usize>> = HashMap::new();
        for site in sites {
            let (kind, id) = reference_key(site.reference);
            let key = (kind, id, site.mnemonic);
            let references = match self.resolved.get(&key) {
                Some(range) => range.clone(),
                None => {
                    let start = self.batch.references.len();
                    let direct_proto = matches!(site.reference, OperandReference::Proto(_));
                    index.visit_reference_leaves::<QueryError>(site.reference, |path, facts| {
                        if !plan.typed_uses && !direct_proto && !path.is_empty() {
                            return Ok(());
                        }
                        let Ok(facts) = facts else {
                            return Ok(());
                        };
                        if let Some((matched, selected)) = matched(facts, &plan.targets)? {
                            self.batch
                                .references
                                .try_reserve(1)
                                .map_err(InputError::Allocation)?;
                            self.batch.references.push(Reference {
                                matched,
                                relevance: selected.relevance,
                                use_: if path.is_empty() {
                                    site.mnemonic.to_owned()
                                } else {
                                    format!("{} → {path}", site.mnemonic)
                                },
                            });
                        }
                        Ok(())
                    })?;
                    let range = start..self.batch.references.len();
                    self.resolved
                        .try_reserve(1)
                        .map_err(InputError::Allocation)?;
                    self.resolved.insert(key, range.clone());
                    range
                }
            };
            if references.is_empty() {
                continue;
            }
            for reference in references {
                let mut hasher = DefaultHasher::new();
                self.batch.references[reference].matched.hash(&mut hasher);
                let hash = hasher.finish();
                if seen.get(&hash).is_some_and(|indices| {
                    indices.iter().any(|&index| {
                        self.batch.references[index].matched
                            == self.batch.references[reference].matched
                    })
                }) {
                    continue;
                }
                seen.try_reserve(1).map_err(InputError::Allocation)?;
                let indices = seen.entry(hash).or_default();
                indices.try_reserve(1).map_err(InputError::Allocation)?;
                indices.push(reference);
                let context = match context {
                    Some(id) => id,
                    None => {
                        let reference = index.method_reference(method)?;
                        if self.batch.contexts.is_empty() {
                            self.batch.descriptor = self.descriptor.units()?;
                        }
                        let id = ContextId(
                            u32::try_from(self.batch.contexts.len())
                                .map_err(|_| InputError::PlatformCapacity)?,
                        );
                        self.batch
                            .contexts
                            .try_reserve(1)
                            .map_err(InputError::Allocation)?;
                        self.batch.contexts.push(Site::Method {
                            method,
                            name: reference.declaration.name.units()?,
                            prototype: prototype_units(&reference.declaration.prototype)?,
                        });
                        context = Some(id);
                        id
                    }
                };
                self.batch
                    .hits
                    .try_reserve(1)
                    .map_err(InputError::Allocation)?;
                self.batch.hits.push(Hit {
                    context,
                    reference: ReferenceId(
                        u32::try_from(reference).map_err(|_| InputError::PlatformCapacity)?,
                    ),
                    code_unit: site.code_unit,
                });
            }
        }
        Ok(())
    }
}

fn reference_key(reference: OperandReference) -> (u8, u32) {
    match reference {
        OperandReference::String(id) => (0, id.0),
        OperandReference::Type(id) => (1, id.0),
        OperandReference::Field(id) => (2, id.0),
        OperandReference::Method(id) => (3, id.0),
        OperandReference::Proto(id) => (4, id.0),
        OperandReference::CallSite(id) => (5, id.0),
        OperandReference::MethodHandle(id) => (6, id.0),
    }
}
