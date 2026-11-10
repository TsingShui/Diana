//! Original-member outline over the backend publication.
//!
//! The backend does not expose the old P2/P8/P9 plan graph. This migration keeps
//! the stable outline document and original member locators, while call
//! expansion remains an explicit later capability.

use super::{
    at,
    decompile::{self, Decompiled},
    request::RequestScope,
    selector::Selector,
};
use diana_decomp::p0::Fact;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum OutlineError {
    #[error(transparent)]
    Decompile(#[from] decompile::DecompileError),
}
impl OutlineError {
    pub fn code(&self) -> u8 {
        match self {
            Self::Decompile(error) => error.code(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Expansion {
    Depth(usize),
    All,
}

#[derive(Serialize)]
pub struct Node {
    pub parent: Option<usize>,
    pub kind: &'static str,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRange>,
}

#[derive(Serialize)]
pub struct SourceRange {
    pub publication: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize)]
pub struct Call {
    pub node: usize,
    pub caller: usize,
    pub pc: u32,
    pub dispatch: &'static str,
    pub origin_only: bool,
    pub abi_attachment: bool,
    pub target: Option<String>,
    pub body: Option<usize>,
    pub state: &'static str,
    pub uncertainty: Option<&'static str>,
}

#[derive(Serialize)]
pub struct Issue {
    pub at: Option<String>,
    pub detail: String,
}

#[derive(Serialize)]
pub struct Document {
    pub root: usize,
    pub complete: bool,
    pub nodes: Vec<Node>,
    pub calls: Vec<Call>,
    pub issues: Vec<Issue>,
    pub publications: Vec<Decompiled>,
}

pub fn run(
    scope: &RequestScope,
    selector: &Selector,
    _expansion: Expansion,
    jobs: usize,
) -> Result<Document, OutlineError> {
    let publication = decompile::run_with_jobs(scope, selector, jobs)?;
    let selection = &publication.selection;
    let facts = publication_facts(&publication);
    let mut nodes = Vec::new();
    nodes.push(Node {
        parent: None,
        kind: "class",
        text: selection.class_at.clone(),
        at: Some(selection.class_at.clone()),
        source: None,
    });
    let mut issues = Vec::new();
    for entry in facts.methods() {
        match entry.declaration() {
            Fact::Present(declaration) => {
                let name = declaration.name_units();
                let prototype = declaration.prototype_units();
                let method = entry.method().0;
                let at = at::method(
                    selection.entry,
                    selection.logical.0,
                    selection.class.class_def.0,
                    method,
                    facts.descriptor().units(),
                    name,
                    prototype,
                );
                nodes.push(Node {
                    parent: Some(0),
                    kind: "method",
                    text: at
                        .rsplit_once("::")
                        .map_or(at.clone(), |(_, tail)| tail.to_owned()),
                    at: Some(at),
                    source: None,
                });
            }
            Fact::Unavailable(error) => issues.push(Issue {
                at: Some(selection.class_at.clone()),
                detail: format!(
                    "method#{} declaration unavailable: {error}",
                    entry.method().0
                ),
            }),
            Fact::Absent | Fact::NotRead => issues.push(Issue {
                at: Some(selection.class_at.clone()),
                detail: format!("method#{} declaration was not delivered", entry.method().0),
            }),
        }
    }
    for entry in facts.fields() {
        match entry.declaration() {
            Fact::Present(declaration) => {
                let at = at::field(
                    selection.entry,
                    selection.logical.0,
                    selection.class.class_def.0,
                    entry.field().0,
                    facts.descriptor().units(),
                    declaration.name.units(),
                    declaration.descriptor.units(),
                );
                nodes.push(Node {
                    parent: Some(0),
                    kind: "field",
                    text: at
                        .rsplit_once("::")
                        .map_or(at.clone(), |(_, tail)| tail.to_owned()),
                    at: Some(at),
                    source: None,
                });
            }
            Fact::Unavailable(error) => issues.push(Issue {
                at: Some(selection.class_at.clone()),
                detail: format!("field#{} declaration unavailable: {error}", entry.field().0),
            }),
            Fact::Absent | Fact::NotRead => issues.push(Issue {
                at: Some(selection.class_at.clone()),
                detail: format!("field#{} declaration was not delivered", entry.field().0),
            }),
        }
    }
    if let Some(error) = facts.methods_issue() {
        issues.push(Issue {
            at: Some(selection.class_at.clone()),
            detail: format!("method directory is incomplete: {error}"),
        });
    }
    if let Some(error) = facts.fields_issue() {
        issues.push(Issue {
            at: Some(selection.class_at.clone()),
            detail: format!("field directory is incomplete: {error}"),
        });
    }
    Ok(Document {
        root: 0,
        complete: issues.is_empty(),
        nodes,
        calls: Vec::new(),
        issues,
        publications: vec![publication],
    })
}

fn publication_facts(publication: &Decompiled) -> &diana_decomp::p0::ClassFacts {
    match publication.rendered.source() {
        diana_decomp::render::SourceProduct::External(external) => &external.facts,
    }
}
