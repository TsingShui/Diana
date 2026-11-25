//! Request assembly: method facts, hierarchy closure and nested classes.

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use crate::input::{DexContainer, InputError, ProgramIndex};
use crate::p0::{ClassFacts, Fact};
use crate::p1::MethodIr;
use crate::{DexId, RequestError};

use super::{DexImage, MethodSelector};

pub(super) struct IndexedImage<'a> {
    pub(super) image: DexImage<'a>,
    pub(super) index: ProgramIndex<'a>,
    pub(super) dex: DexId,
    classes_by_descriptor: OnceLock<HashMap<Cow<'a, str>, crate::ClassDefId>>,
}

impl<'a> IndexedImage<'a> {
    pub(super) fn open(image: DexImage<'a>, dex: DexId) -> Result<Self, InputError> {
        let container = DexContainer::open(image.bytes)?;
        let index = container.program(image.header_offset)?.with_dex(dex);
        Ok(Self {
            image,
            index,
            dex,
            classes_by_descriptor: OnceLock::new(),
        })
    }

    pub(super) fn classes_by_descriptor(&self) -> &HashMap<Cow<'a, str>, crate::ClassDefId> {
        self.classes_by_descriptor.get_or_init(|| {
            let mut classes = HashMap::new();
            for ordinal in 0..self.index.class_count() {
                let key = crate::ClassDefId(ordinal);
                let Ok(view) = self.index.class(key) else {
                    continue;
                };
                let Ok(descriptor) = view.descriptor.utf8_lossy() else {
                    continue;
                };
                classes.insert(descriptor, key);
            }
            classes
        })
    }
}

pub(super) struct MethodFacts {
    pub(super) ordinal: usize,
    pub(super) decoded: Result<MethodIr, String>,
}

/// One complete P0/P1 delivery for a backend class request.
pub(super) struct BackendInput {
    pub(super) class: Arc<ClassFacts>,
    pub(super) related: Vec<Arc<ClassFacts>>,
    pub(super) methods: Vec<MethodFacts>,
    pub(super) nested: Vec<BackendInput>,
}

pub(super) fn backend_input<'a>(
    image: &IndexedImage<'a>,
    class_def: crate::ClassDefId,
    selector: Option<&MethodSelector>,
    include_nested: bool,
) -> Result<BackendInput, RequestError> {
    let facts = image.index.class_facts_cached(class_def)?;
    backend_input_from_facts(image, facts, selector, include_nested)
}

pub(super) fn backend_input_from_facts<'a>(
    image: &IndexedImage<'a>,
    facts: Arc<ClassFacts>,
    selector: Option<&MethodSelector>,
    include_nested: bool,
) -> Result<BackendInput, RequestError> {
    let class_def = facts.class_def();
    let mut methods = Vec::new();
    let selected = selector
        .filter(|_| !facts.methods().is_empty())
        .map(|selector| facts.resolve_method(&selector.name, &selector.prototype))
        .transpose()?;
    for (ordinal, entry) in facts.methods().iter().enumerate() {
        let id = entry.method();
        if selected.is_some_and(|selected| selected != id) {
            continue;
        }
        let source = match image.index.method_source_by_ordinal(&facts, ordinal) {
            Ok(source) => source,
            Err(error) => {
                methods.push(MethodFacts {
                    ordinal,
                    decoded: Err(error.to_string()),
                });
                continue;
            }
        };
        if !matches!(source.raw(), crate::input::RawPrintable::Method(_)) {
            continue;
        }
        let decoded = match image.index.decode_method_source(&source) {
            Ok(decoded) => decoded,
            Err(error) => {
                methods.push(MethodFacts {
                    ordinal,
                    decoded: Err(error.to_string()),
                });
                continue;
            }
        };
        match decoded.decoded {
            Some(decoded) => methods.push(MethodFacts {
                ordinal,
                decoded: decoded
                    .into_ir()
                    .map_err(|reason| crate::DexError::Backend(reason).to_string()),
            }),
            None => methods.push(MethodFacts {
                ordinal,
                decoded: Err(decoded
                    .error
                    .map_or_else(|| "method decoding stopped".to_owned(), |e| e.to_string())),
            }),
        }
    }

    let classes = image.classes_by_descriptor();
    let mut related = Vec::new();
    let mut visited = BTreeSet::new();
    visited.insert(class_def);
    let mut frontier = vec![facts.clone()];
    while let Some(current) = frontier.pop() {
        let super_class = match current.superclass() {
            Fact::Present(name) => Some(name),
            _ => None,
        };
        let interfaces = match current.interfaces() {
            Fact::Present(interfaces) => interfaces.as_slice(),
            _ => &[],
        };
        let interfaces = interfaces
            .iter()
            .filter_map(|interface| match &interface.descriptor {
                Fact::Present(name) => Some(name),
                _ => None,
            });
        for descriptor in super_class.into_iter().chain(interfaces) {
            let Ok(text) = String::from_utf16(descriptor.units()) else {
                continue;
            };
            let Some(&key) = classes.get(text.as_str()) else {
                continue;
            };
            if !visited.insert(key) {
                continue;
            }
            let Ok(other) = image.index.class_facts_cached(key) else {
                continue;
            };
            frontier.push(other.clone());
            related.push(other);
        }
    }

    let mut nested = Vec::new();
    if include_nested {
        let Ok(current) = String::from_utf16(facts.descriptor().units()) else {
            return Err(RequestError::InternalInvariant(
                "class descriptor is not UTF-16",
            ));
        };
        let base = current.strip_suffix(';').unwrap_or(&current);
        let prefix = format!("{base}$");
        let mut children = Vec::new();
        for (descriptor, key) in classes {
            if *key == class_def {
                continue;
            }
            let Some(rest) = descriptor.strip_prefix(&prefix) else {
                continue;
            };
            let name = rest.strip_suffix(';').unwrap_or(rest);
            if name.contains('$') {
                continue;
            }
            children.push(*key);
        }
        children.sort_unstable();
        for key in children {
            nested.push(backend_input(image, key, None, true)?);
        }
    }

    Ok(BackendInput {
        class: facts,
        related,
        methods,
        nested,
    })
}
