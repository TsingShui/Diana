//! Rendered text boundary for the recovery backend.

use std::sync::Arc;

use serde::Serialize;

use crate::p0::ClassFacts;
use crate::ClassKey;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderIssueKind {
    Backend,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderIssueCause {
    Backend,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenderIssue {
    pub kind: RenderIssueKind,
    pub detail: String,
    pub cause: RenderIssueCause,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderErrorKind {
    Backend,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenderError {
    kind: RenderErrorKind,
    detail: String,
}

impl RenderError {
    pub fn backend(detail: impl Into<String>) -> Self {
        Self {
            kind: RenderErrorKind::Backend,
            detail: detail.into(),
        }
    }

    pub fn kind(&self) -> RenderErrorKind {
        self.kind
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detail.fmt(formatter)
    }
}

impl std::error::Error for RenderError {}

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
    issues: Vec<RenderIssue>,
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
            issues: Vec::new(),
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

    pub fn issues(&self) -> &[RenderIssue] {
        &self.issues
    }

    pub fn write_text(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writer.write_all(self.text.as_bytes())
    }

    pub fn write_json(&self, writer: &mut impl std::io::Write) -> serde_json::Result<()> {
        serde_json::to_writer(writer, self)
    }
}

#[derive(Serialize)]
pub struct RenderStop {
    #[serde(skip)]
    source: SourceProduct,
    text: String,
    issues: Vec<RenderIssue>,
    error: RenderError,
}

impl RenderStop {
    pub fn text_prefix(&self) -> &str {
        &self.text
    }

    pub fn source(&self) -> &SourceProduct {
        &self.source
    }

    pub fn issues(&self) -> &[RenderIssue] {
        &self.issues
    }

    pub fn error(&self) -> &RenderError {
        &self.error
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "product", rename_all = "snake_case")]
pub enum RenderOutcome {
    Ready(Rendered),
    Stopped(RenderStop),
}
