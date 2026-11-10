//! Thin Engine facade over the original DEX facts and the recovery backend.

use std::sync::Arc;

use crate::input::{DexContainer, InputError, InputProblem, ProgramIndex};
use crate::p0::{ClassFacts, Fact};
use crate::p9::{RenderOutcome, Rendered};
use crate::{ByteOffset, ClassKey, DexId, ImageFact, ImageOrigin, MethodId, RequestError};

#[derive(Clone, Copy)]
pub struct DexImage<'a> {
    pub bytes: &'a [u8],
    pub header_offset: ByteOffset,
    pub origin: ImageOrigin,
}

pub struct MethodSelector {
    pub name: Vec<u16>,
    pub prototype: Vec<u16>,
}

pub struct Request {
    pub class: ClassKey,
    pub method: Option<MethodSelector>,
}

struct IndexedImage<'a> {
    image: DexImage<'a>,
    index: ProgramIndex<'a>,
    dex: DexId,
}

pub struct Engine<'a> {
    images: Vec<IndexedImage<'a>>,
}

pub struct ClassSource {
    pub class: ClassKey,
    pub text: String,
    pub methods: Vec<MethodSource>,
}

pub struct MethodSource {
    pub method: MethodId,
    pub name: Vec<u16>,
    pub prototype: Vec<u16>,
    pub failure: Option<String>,
}

pub struct ClassAnalysis {
    pub image: ImageFact,
    pub facts: Arc<ClassFacts>,
    pub rendered: RenderOutcome,
}

pub struct SingleMethodAnalysis {
    pub rendered: RenderOutcome,
}

impl<'a> Engine<'a> {
    pub fn with_method_jobs(self, _jobs: usize) -> Self {
        self
    }

    pub fn with_outline(self) -> Self {
        self
    }

    pub fn open(images: Vec<DexImage<'a>>) -> Result<Self, InputError> {
        if images.is_empty() {
            return Err(InputError::Malformed {
                code: InputProblem::InvalidReference,
                offset_byte: ByteOffset(0),
            });
        }
        let mut indexed = Vec::new();
        indexed
            .try_reserve_exact(images.len())
            .map_err(|_| InputError::Allocation {
                offset_byte: ByteOffset(0),
            })?;
        for image in images {
            let dex =
                DexId(
                    u32::try_from(indexed.len()).map_err(|_| InputError::PlatformCapacity {
                        offset_byte: image.header_offset,
                    })?,
                );
            let container = DexContainer::open(image.bytes)?;
            let index = container.program(image.header_offset)?.with_dex(dex);
            indexed.push(IndexedImage { image, index, dex });
        }
        Ok(Self { images: indexed })
    }

    pub fn focus(image: DexImage<'a>, dex: DexId) -> Result<Self, InputError> {
        let container = DexContainer::open(image.bytes)?;
        let index = container.program(image.header_offset)?.with_dex(dex);
        let mut images = Vec::new();
        images
            .try_reserve_exact(1)
            .map_err(|_| InputError::Allocation {
                offset_byte: image.header_offset,
            })?;
        images.push(IndexedImage { image, index, dex });
        Ok(Self { images })
    }

    fn image(&self, dex: DexId) -> Option<&IndexedImage<'a>> {
        self.images.iter().find(|image| image.dex == dex)
    }

    pub fn class_facts(&self, class: ClassKey) -> Result<Arc<ClassFacts>, RequestError> {
        let image = self.image(class.dex).ok_or(RequestError::UnknownDex)?;
        image.index.class_facts_cached(class.class_def)
    }

    /// Recover one class from P0/P1 facts through the backend.
    pub fn class_source(&self, request: Request) -> Result<ClassSource, RequestError> {
        let image = self
            .image(request.class.dex)
            .ok_or(RequestError::UnknownDex)?;
        let facts = image.index.class_facts_cached(request.class.class_def)?;
        let mut node = crate::backend::class_node(&facts).ok_or(
            RequestError::InternalInvariant("class node assembly failed"),
        )?;
        let mut methods = Vec::new();
        let mut inputs = Vec::new();
        for entry in facts.methods() {
            let id = entry.method();
            if let Some(selector) = &request.method {
                let selected = facts.resolve_method(&selector.name, &selector.prototype)?;
                if selected != id {
                    continue;
                }
            }
            let Ok(source) = image.index.method_source(&facts, id) else {
                continue;
            };
            let crate::input::RawPrintable::Method(view) = source.raw() else {
                continue;
            };
            let Ok(decoded) = image.index.decode_method_source(&source) else {
                continue;
            };
            let Some(decoded) = decoded.decoded else {
                continue;
            };
            let Some(method) = node
                .methods()
                .iter()
                .find(|method| method.id == id.0)
                .cloned()
            else {
                continue;
            };
            methods.push(MethodSource {
                method: id,
                name: method.info.name.encode_utf16().collect(),
                prototype: method.info.descriptor().encode_utf16().collect(),
                failure: None,
            });
            let code =
                crate::backend::method_code(view, crate::backend::tries(&image.index, &decoded));
            if let Some(target) = node
                .methods_mut()
                .iter_mut()
                .find(|method| method.id == id.0)
            {
                target.code = Some(code.clone());
            }
            let mut method = method;
            method.code = Some(code.clone());
            inputs.push(crate::MethodInput {
                method,
                code,
                symbols: crate::backend::symbols(&image.index, &decoded),
            });
        }

        let declaration_copy = node.clone();
        let mut related = Vec::new();
        let mut wanted = Vec::new();
        if let Fact::Present(super_class) = facts.superclass() {
            if let Ok(text) = String::from_utf16(super_class.units()) {
                wanted.push(text);
            }
        }
        if let Fact::Present(interfaces) = facts.interfaces() {
            for interface in interfaces {
                if let Fact::Present(descriptor) = &interface.descriptor {
                    if let Ok(text) = String::from_utf16(descriptor.units()) {
                        wanted.push(text);
                    }
                }
            }
        }
        for descriptor in wanted {
            for ordinal in 0..image.index.class_count() {
                let key = crate::ClassDefId(ordinal);
                let Ok(other) = image.index.class_facts_cached(key) else {
                    continue;
                };
                let Ok(text) = String::from_utf16(other.descriptor().units()) else {
                    continue;
                };
                if text == descriptor {
                    if let Some(node) = crate::backend::class_node(&other) {
                        related.push(node);
                    }
                    break;
                }
            }
        }

        let mut declarations: Vec<&crate::ClassNode> = vec![&declaration_copy];
        declarations.extend(related.iter());
        let output = crate::decompile_class(
            &declarations,
            crate::ClassInput {
                class: node,
                methods: inputs,
            },
            &crate::MethodOptions::default(),
        )
        .map_err(|error| {
            RequestError::InternalInvariant(Box::leak(
                format!("backend failed: {error}").into_boxed_str(),
            ))
        })?;
        for outcome in &output.methods {
            if let Some(failure) = &outcome.failure {
                if let Some(method) = methods.iter_mut().find(|method| {
                    String::from_utf16(&method.name).ok().as_deref() == Some(outcome.name.as_str())
                }) {
                    method.failure = Some(failure.clone());
                }
            }
        }
        Ok(ClassSource {
            class: request.class,
            text: output.source,
            methods,
        })
    }

    pub fn analyze(&self, request: Request) -> Result<ClassAnalysis, RequestError> {
        let class = request.class;
        let image = self.image(class.dex).ok_or(RequestError::UnknownDex)?;
        let facts = image.index.class_facts_cached(class.class_def)?;
        let source = self.class_source(request)?;
        let rendered = RenderOutcome::Ready(Rendered::external(class, source.text, facts.clone()));
        Ok(ClassAnalysis {
            image: ImageFact {
                dex: class.dex,
                origin: image.image.origin,
                header_offset: image.image.header_offset,
            },
            facts,
            rendered,
        })
    }

    pub fn analyze_method(
        &self,
        class: ClassKey,
        method: MethodId,
    ) -> Result<SingleMethodAnalysis, RequestError> {
        let facts = self.class_facts(class)?;
        let entry = facts
            .method_entry(method)
            .ok_or(RequestError::InternalInvariant("selected method is absent"))?;
        let Fact::Present(declaration) = entry.declaration() else {
            return Err(RequestError::InternalInvariant(
                "selected method declaration is unavailable",
            ));
        };
        let source = self.class_source(Request {
            class,
            method: Some(MethodSelector {
                name: declaration.name_units().to_vec(),
                prototype: declaration.prototype_units().to_vec(),
            }),
        })?;
        Ok(SingleMethodAnalysis {
            rendered: RenderOutcome::Ready(Rendered::external(class, source.text, facts)),
        })
    }
}
