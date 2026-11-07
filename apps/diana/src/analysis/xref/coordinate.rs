//! Resolve canonical coordinates through shared checked declaration readers.
//! Coordinates select an original value, not a restriction on referencing images.

use diana_decomp::StringId;

use super::{Relation, Target, TextMatch};
use crate::analysis::images::{ImageSet, resolve_class};
use crate::analysis::request::RequestScope;
use crate::analysis::selector::{Selector, SelectorError};

/// Validates physical/logical/index/name correspondence and member membership
/// without opening method bodies. Searches use the resolved complete value across
/// the input: numeric DEX IDs themselves are not cross-image reference identities.
pub fn resolve_coordinate(
    scope: &RequestScope,
    kind: &str,
    selector: &Selector,
) -> Result<Target, SelectorError> {
    let class_selector = match (kind, selector) {
        ("string", Selector::String(selector)) => {
            let images = ImageSet::open(scope, selector.entry)?;
            let mut found = None;
            for image in images.images() {
                if selector.logical != Some(image.logical.0) {
                    continue;
                }
                let index = images.program(image)?;
                if selector.string >= index.string_ids_len() {
                    return Err(SelectorError::InvalidIndex);
                }
                let value = index.string(StringId(selector.string))?.units()?;
                if found.replace(value).is_some() {
                    return Err(SelectorError::Ambiguous);
                }
            }
            return found
                .map(|value| Target::String(TextMatch::Exact(value)))
                .ok_or(SelectorError::NotFound);
        }
        ("type" | "impl" | "extend", Selector::Class(selector)) => selector,
        ("method", Selector::Method(selector)) => &selector.class,
        ("field", Selector::Field(selector)) => &selector.class,
        _ => {
            return Err(SelectorError::Syntax(
                "coordinate kind does not match xref kind",
            ));
        }
    };
    let images = ImageSet::open(scope, class_selector.entry)?;
    let selected = resolve_class(&images, class_selector)?;
    let index = images.program(selected.image)?;
    let class = index.class(selected.class_def)?;
    match selector {
        Selector::Class(_) => {
            let target = TextMatch::Exact(selected.descriptor);
            Ok(match kind {
                "impl" => Target::Hierarchy {
                    relation: Relation::Impl,
                    target,
                },
                "extend" => Target::Hierarchy {
                    relation: Relation::Extend,
                    target,
                },
                _ => Target::Type(target),
            })
        }
        Selector::Method(selector) => {
            let mut found = false;
            let mut name_filter =
                |_, name: diana_decomp::input::DexString<'_>| name.matches(&selector.name);
            class.visit_method_headers(Some(&mut name_filter), |id, _, declaration| {
                if Some(id.0) != selector.method {
                    return Ok(());
                }
                let declaration = declaration?;
                found = declaration.name.matches(&selector.name)?
                    && declaration.prototype.matches(&selector.prototype)?;
                Ok(())
            })?;
            if !found {
                return Err(SelectorError::InvalidIndex);
            }
            Ok(Target::Method {
                owner: Some(TextMatch::Exact(selected.descriptor)),
                name: TextMatch::Exact(selector.name.clone()),
                prototype: Some(selector.prototype.clone()),
                relevance: None,
            })
        }
        Selector::Field(selector) => {
            let mut found = false;
            class.visit_fields(|field| {
                if Some(field.field_idx.0) != selector.field {
                    return Ok(());
                }
                let declaration = field.declaration?;
                found = declaration.name.matches(&selector.name)?
                    && declaration.field_type.matches(&selector.field_type)?;
                Ok(())
            })?;
            if !found {
                return Err(SelectorError::InvalidIndex);
            }
            Ok(Target::Field {
                owner: Some(TextMatch::Exact(selected.descriptor)),
                name: TextMatch::Exact(selector.name.clone()),
                field_type: Some(selector.field_type.clone()),
            })
        }
        Selector::String(_) => Err(SelectorError::Syntax("expected a declaration coordinate")),
    }
}
