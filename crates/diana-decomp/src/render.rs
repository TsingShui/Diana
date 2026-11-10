//! Rendered text product.

use std::sync::Arc;

use serde::Serialize;

use crate::p0::ClassFacts;
use crate::ClassKey;

/// One class recovered by the backend, with the original facts it consumed.
pub struct ExternalProduct {
    pub class: ClassKey,
    pub text: String,
    pub facts: Arc<ClassFacts>,
}

pub enum SourceProduct {
    External(Arc<ExternalProduct>),
}

#[derive(Serialize)]
pub struct Rendered {
    #[serde(skip)]
    source: SourceProduct,
    class: ClassKey,
    notation: &'static str,
    text: String,
}

impl Rendered {
    pub(crate) fn external(class: ClassKey, text: String, facts: Arc<ClassFacts>) -> Self {
        Self {
            source: SourceProduct::External(Arc::new(ExternalProduct {
                class,
                text: text.clone(),
                facts,
            })),
            class,
            notation: "diana-decomp",
            text,
        }
    }

    pub fn class(&self) -> ClassKey {
        self.class
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn source(&self) -> &SourceProduct {
        &self.source
    }

    pub fn write_text(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writer.write_all(self.text.as_bytes())
    }

    pub fn write_json(&self, writer: &mut impl std::io::Write) -> serde_json::Result<()> {
        serde_json::to_writer(writer, self)
    }
}
