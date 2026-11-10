//! Request assembly: method facts, hierarchy closure and nested classes.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use crate::input::{DexContainer, InputError, ProgramIndex};
use crate::p0::{ClassFacts, Fact};
use crate::p1::DecodedMethod;
use crate::{DexId, MethodId, RequestError};

use super::{DexImage, MethodSelector};

pub(super) struct IndexedImage<'a> {
    pub(super) image: DexImage<'a>,
    pub(super) index: ProgramIndex<'a>,
    pub(super) dex: DexId,
    classes_by_descriptor: OnceLock<HashMap<String, crate::ClassDefId>>,
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

    pub(super) fn classes_by_descriptor(&self) -> &HashMap<String, crate::ClassDefId> {
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
                classes.insert(descriptor.into_owned(), key);
            }
            classes
        })
    }
}

pub(super) struct MethodFacts<'a> {
    pub(super) method: MethodId,
    pub(super) decoded: Result<DecodedMethod<'a>, String>,
}

/// One complete P0/P1 delivery for a backend class request.
pub(super) struct BackendInput<'a> {
    pub(super) class: Arc<ClassFacts>,
    pub(super) related: Vec<Arc<ClassFacts>>,
    pub(super) methods: Vec<MethodFacts<'a>>,
    pub(super) nested: Vec<BackendInput<'a>>,
}

pub(super) fn backend_input<'a>(
    image: &IndexedImage<'a>,
    class_def: crate::ClassDefId,
    selector: Option<&MethodSelector>,
    include_nested: bool,
) -> Result<BackendInput<'a>, RequestError> {
    let facts = image.index.class_facts_cached(class_def)?;
    let mut methods = Vec::new();
    for entry in facts.methods() {
        let id = entry.method();
        if let Some(selector) = selector {
            let selected = facts.resolve_method(&selector.name, &selector.prototype)?;
            if selected != id {
                continue;
            }
        }
        let source = match image.index.method_source(&facts, id) {
            Ok(source) => source,
            Err(error) => {
                methods.push(MethodFacts {
                    method: id,
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
                    method: id,
                    decoded: Err(error.to_string()),
                });
                continue;
            }
        };
        match decoded.decoded {
            Some(decoded) => methods.push(MethodFacts {
                method: id,
                decoded: Ok(decoded),
            }),
            None => methods.push(MethodFacts {
                method: id,
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
        let mut wanted = Vec::new();
        if let Fact::Present(super_class) = current.superclass() {
            wanted.push(super_class);
        }
        if let Fact::Present(interfaces) = current.interfaces() {
            for interface in interfaces {
                if let Fact::Present(descriptor) = &interface.descriptor {
                    wanted.push(descriptor);
                }
            }
        }
        for descriptor in wanted {
            let Ok(text) = String::from_utf16(descriptor.units()) else {
                continue;
            };
            let Some(&key) = classes.get(&text) else {
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
