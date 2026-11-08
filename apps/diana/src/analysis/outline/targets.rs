//! Declaration-only class selection for cross-method navigation.
use std::collections::HashMap;

use super::{Issue, OutlineError, copied, push};
use crate::analysis::{
    apk::DexRoot,
    dex,
    query::{QueryError, Row},
    selector::ClassSelector,
};
use diana_decomp::RequestError;

pub(super) struct Targets {
    classes: HashMap<Vec<u16>, Vec<ClassSelector>>,
    complete: bool,
}
impl Targets {
    pub fn unavailable() -> Self {
        Self {
            classes: HashMap::new(),
            complete: false,
        }
    }
    pub fn read(roots: &[DexRoot<'_>], issues: &mut Vec<Issue>) -> Result<Self, OutlineError> {
        let mut result = Self {
            classes: HashMap::new(),
            complete: true,
        };
        let mut scratch = Vec::new();
        for (ordinal, root) in roots.iter().enumerate() {
            let scan = (|| -> Result<(), QueryError> {
                let entry = u32::try_from(root.entry().map_or(0, |r| r.index().0))
                    .map_err(|_| RequestError::PlatformCapacity)?;
                dex::visit_logical(root.bytes_into(&mut scratch)?, |logical, index| {
                    dex::classes(
                        &index,
                        dex::ImageIndex(ordinal),
                        logical,
                        None,
                        &dex::ClassFilter::default(),
                        &mut |row| {
                            let Row::Class(row) = row else {
                                return Ok(());
                            };
                            let owner = row.descriptor.units()?;
                            let descriptor = copied(&owner).map_err(QueryError::Request)?;
                            if !result.classes.contains_key(&owner) {
                                result
                                    .classes
                                    .try_reserve(1)
                                    .map_err(RequestError::Allocation)?;
                            }
                            let entries = result.classes.entry(owner).or_default();
                            entries.try_reserve(1).map_err(RequestError::Allocation)?;
                            entries.push(ClassSelector {
                                entry: Some(entry),
                                logical: Some(logical.0),
                                class_def: Some(row.class_def.0),
                                descriptor,
                            });
                            Ok(())
                        },
                    )
                })
            })();
            if let Err(error) = scan {
                result.complete = false;
                push(
                    issues,
                    Issue {
                        at: None,
                        detail: format!(
                            "declaration scan incomplete ({:?}): {error}",
                            error.kind()
                        ),
                    },
                )?;
            }
        }
        Ok(result)
    }
    pub fn resolve(&self, owner: &[u16]) -> Result<&ClassSelector, &'static str> {
        if !self.complete {
            return Err("target uniqueness unknown: declaration scan incomplete");
        }
        match self.classes.get(owner).map(Vec::as_slice).unwrap_or(&[]) {
            [] => Err("no owner definition in the input"),
            [class] => Ok(class),
            _ => Err("ambiguous owner: multiple original class definitions"),
        }
    }
}
