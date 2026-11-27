//! Package aggregation over the ordinary class query. No DEX parsing occurs here.
//!
//! Each physical root has an independent builder, so parallel class scans never
//! contend on the same tree. The caller merges package nodes after the scan; equal
//! class names remain distinct leaves with their original locators. Arena storage
//! avoids recursive construction/destruction for input-derived package depths.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use serde::Serialize;

use super::at;
use super::input::InputError;
use super::query::{JvmItem, JvmRow, Query, QueryError, QueryIssue, QueryKind, QueryPlan, Row};
use super::request::RequestScope;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PackageId(pub usize);

#[derive(Serialize)]
pub struct ClassLeaf {
    pub name: String,
    pub at: String,
    pub descriptor_units: Vec<u16>,
}

#[derive(Serialize)]
pub struct Package {
    pub parent: Option<PackageId>,
    pub name: String,
    pub name_units: Vec<u16>,
    /// Definitions in the entire subtree, including packages beyond `--level`.
    pub class_count: usize,
    pub children: Vec<PackageId>,
    pub classes: Vec<ClassLeaf>,
    #[serde(skip)]
    lookup: HashMap<Vec<u16>, PackageId>,
}

#[derive(Serialize)]
pub struct PackageTree {
    pub root: PackageId,
    pub packages: Vec<Package>,
}

/// Recovered tree and the original scan failure, if coverage is incomplete.
pub struct TreeResult {
    pub tree: PackageTree,
    pub issue: Option<QueryError>,
    pub issues: Vec<QueryIssue>,
}

struct Builder {
    tree: PackageTree,
    path: Vec<PackageId>,
}

impl Builder {
    fn new(prefix: &[u16]) -> Result<Self, QueryError> {
        let name_units = copy_units(prefix)?;
        let name = if prefix.is_empty() {
            ".".to_owned()
        } else {
            at::escape_units(prefix).replace('/', ".")
        };
        let mut packages = Vec::new();
        packages.try_reserve(1).map_err(InputError::Allocation)?;
        packages.push(Package {
            parent: None,
            name,
            name_units,
            class_count: 0,
            children: Vec::new(),
            classes: Vec::new(),
            lookup: HashMap::new(),
        });
        Ok(Self {
            tree: PackageTree {
                root: PackageId(0),
                packages,
            },
            path: Vec::new(),
        })
    }

    fn child(&mut self, parent: PackageId, name: &[u16]) -> Result<PackageId, QueryError> {
        if let Some(&child) = self.tree.packages[parent.0].lookup.get(name) {
            return Ok(child);
        }
        let name_units = copy_units(name)?;
        let key = copy_units(name)?;
        self.tree
            .packages
            .try_reserve(1)
            .map_err(InputError::Allocation)?;
        let owner = &mut self.tree.packages[parent.0];
        owner
            .children
            .try_reserve(1)
            .map_err(InputError::Allocation)?;
        owner
            .lookup
            .try_reserve(1)
            .map_err(InputError::Allocation)?;
        let child = PackageId(self.tree.packages.len());
        self.tree.packages.push(Package {
            parent: Some(parent),
            name: at::escape_units(name),
            name_units,
            class_count: 0,
            children: Vec::new(),
            classes: Vec::new(),
            lookup: HashMap::new(),
        });
        let owner = &mut self.tree.packages[parent.0];
        owner.children.push(child);
        owner.lookup.insert(key, child);
        Ok(child)
    }

    fn add(
        &mut self,
        units: Vec<u16>,
        locator: impl FnOnce(&[u16]) -> String,
        prefix: &[u16],
        level: Option<NonZeroUsize>,
        classes: bool,
    ) -> Result<(), QueryError> {
        // ClassRow comes from a checked class descriptor, not a synthesized name.
        let interior = units.get(1..units.len().saturating_sub(1)).ok_or(
            diana_decomp::input::InputError::InternalInvariant(
                "tree received a non-class descriptor",
            ),
        )?;
        let (package, name) = match interior.iter().rposition(|&unit| unit == u16::from(b'/')) {
            Some(at) => (&interior[..at], &interior[at + 1..]),
            None => (&interior[..0], interior),
        };
        let relative = if prefix.is_empty() {
            package
        } else if package == prefix {
            &package[..0]
        } else if let Some(tail) = package.strip_prefix(prefix)
            && let Some(tail) = tail.strip_prefix(&[u16::from(b'/')])
        {
            tail
        } else {
            return Ok(());
        };
        self.path.clear();
        self.path.try_reserve(1).map_err(InputError::Allocation)?;
        self.path.push(self.tree.root);
        let mut current = self.tree.root;
        let mut depth = 0;
        for segment in relative
            .split(|&unit| unit == u16::from(b'/'))
            .filter(|s| !s.is_empty())
        {
            depth += 1;
            if level.is_some_and(|level| depth > level.get()) {
                break;
            }
            current = self.child(current, segment)?;
            self.path.try_reserve(1).map_err(InputError::Allocation)?;
            self.path.push(current);
        }
        for id in &self.path {
            if self.tree.packages[id.0].class_count == usize::MAX {
                return Err(InputError::PlatformCapacity.into());
            }
        }
        // Reserve the leaf before accounting for this definition. A construction
        // failure cannot publish counts for a class whose leaf was lost.
        if classes && level.is_none_or(|level| depth <= level.get()) {
            let leaf = ClassLeaf {
                name: at::escape_units(name),
                at: locator(&units),
                descriptor_units: units,
            };
            let leaves = &mut self.tree.packages[current.0].classes;
            leaves.try_reserve(1).map_err(InputError::Allocation)?;
            leaves.push(leaf);
        }
        for id in &self.path {
            self.tree.packages[id.0].class_count += 1;
        }
        Ok(())
    }

    fn merge(&mut self, other: PackageTree) -> Result<(), QueryError> {
        let mut mapped = Vec::new();
        mapped
            .try_reserve_exact(other.packages.len())
            .map_err(InputError::Allocation)?;
        // Parents precede children in the arena. No descriptor is resolved again.
        for mut node in other.packages {
            let target = match node.parent {
                None => self.tree.root,
                Some(parent) => self.child(mapped[parent.0], &node.name_units)?,
            };
            let into = &mut self.tree.packages[target.0];
            into.class_count = into
                .class_count
                .checked_add(node.class_count)
                .ok_or(InputError::PlatformCapacity)?;
            into.classes
                .try_reserve(node.classes.len())
                .map_err(InputError::Allocation)?;
            into.classes.append(&mut node.classes);
            mapped.push(target);
        }
        Ok(())
    }
}

fn copy_units(units: &[u16]) -> Result<Vec<u16>, QueryError> {
    let mut copied = Vec::new();
    copied
        .try_reserve_exact(units.len())
        .map_err(InputError::Allocation)?;
    copied.extend_from_slice(units);
    Ok(copied)
}

/// Builds the selected package hierarchy from the shared class query. `prefix`
/// is a slash-separated package path, without leading/trailing separators.
/// A depth is an explicit structural selection; descendant counts remain complete.
pub fn build(
    scope: &RequestScope,
    prefix: &[u16],
    level: Option<NonZeroUsize>,
    classes: bool,
    jobs: NonZeroUsize,
) -> Result<TreeResult, QueryError> {
    let query = Query::new(
        scope,
        QueryPlan {
            kind: QueryKind::Classes,
            filter: None,
            class: None,
            class_filter: Default::default(),
            jobs: jobs.get(),
            limit: 0,
        },
    )?;
    let mut shards = Vec::new();
    shards
        .try_reserve_exact(query.shard_count())
        .map_err(InputError::Allocation)?;
    for _ in 0..query.shard_count() {
        shards.push(Mutex::new(Builder::new(prefix)?));
    }
    let outcome = query.run_detailed(
        &|row, _, _| match row {
            Row::Class(row) => {
                let image = query
                    .images()
                    .get(row.image.0)
                    .ok_or(QueryError::WorkerLost)?;
                let units = row.descriptor.units()?;
                let mut builder = shards[row.image.0]
                    .lock()
                    .map_err(|_| QueryError::WorkerLost)?;
                builder.add(
                    units,
                    |units| {
                        at::class(
                            image.entry.map(|e| e.0),
                            row.logical.0,
                            row.class_def.0,
                            units,
                        )
                    },
                    prefix,
                    level,
                    classes,
                )
            }
            Row::Jvm(JvmRow {
                item: JvmItem::Class(name),
                location,
                shard,
                ..
            }) => {
                let units = super::query::jvm::descriptor(name)?;
                let mut builder = shards[shard].lock().map_err(|_| QueryError::WorkerLost)?;
                builder.add(units, |_| location.to_string(), prefix, level, classes)
            }
            _ => Err(diana_decomp::input::InputError::InternalInvariant(
                "tree query delivered a non-class",
            )
            .into()),
        },
        &mut |_| Ok(()),
    );
    let (issue, issues) = match outcome {
        Ok(outcome) => (None, outcome.issues),
        Err(issue) => (Some(issue), Vec::new()),
    };
    let mut combined = Builder::new(prefix)?;
    for shard in shards {
        let builder = shard.into_inner().map_err(|_| {
            diana_decomp::input::InputError::InternalInvariant("tree builder poisoned")
        })?;
        combined.merge(builder.tree)?;
    }
    Ok(TreeResult {
        tree: combined.tree,
        issue,
        issues,
    })
}
