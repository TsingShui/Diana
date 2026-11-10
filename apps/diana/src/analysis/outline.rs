//! Call-tree outline over the Engine's P0 declarations and P1 invoke sites.

use diana_decomp::outline::{OutlineDepth, OutlineKind, OutlineMember};
use diana_decomp::{ClassKey, DexImage, Engine, ImageOrigin, InputId, RequestError};
use serde::Serialize;

use super::at;
use super::decompile::{self, DecompileError, Selection};
use super::images::ImageSet;
use super::request::RequestScope;
use super::selector::{Selector, SelectorError};

#[derive(Debug, thiserror::Error)]
pub enum OutlineError {
    #[error(transparent)]
    Decompile(#[from] DecompileError),
    #[error(transparent)]
    Request(#[from] RequestError),
}

impl OutlineError {
    pub fn code(&self) -> u8 {
        match self {
            Self::Decompile(error) => error.code(),
            Self::Request(_) => 1,
        }
    }
}

/// Requested call-tree depth.
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
}

#[derive(Serialize)]
pub struct Issue {
    pub at: Option<String>,
    pub detail: String,
}

#[derive(Serialize)]
pub struct Document {
    pub root: usize,
    pub nodes: Vec<Node>,
    pub issues: Vec<Issue>,
}

pub fn run(
    scope: &RequestScope,
    selector: &Selector,
    expansion: Expansion,
    jobs: usize,
) -> Result<Document, OutlineError> {
    let class_selector = match selector {
        Selector::Class(class) => class,
        Selector::Method(method) => &method.class,
        Selector::Field(_) | Selector::String(_) => {
            return Err(OutlineError::Decompile(DecompileError::Selector(
                SelectorError::Syntax("this command selects a class or a method"),
            )));
        }
    };
    let (images, resolved) = ImageSet::resolve(scope, class_selector, super::image_jobs())
        .map_err(DecompileError::from)?;
    let selected = images.selection(&resolved);
    let mut dex_images = Vec::new();
    dex_images
        .try_reserve(images.images().len())
        .map_err(|_| RequestError::PlatformCapacity)?;
    for image in images.images() {
        dex_images.push(DexImage {
            bytes: images.bytes(image).map_err(DecompileError::from)?,
            header_offset: image.header_offset,
            origin: ImageOrigin {
                input: InputId(
                    u64::try_from(image.entry.map_or(0, |entry| entry.0))
                        .map_err(|_| RequestError::PlatformCapacity)?,
                ),
            },
        });
    }
    let engine = Engine::open(dex_images)
        .map_err(DecompileError::from)?
        .with_method_jobs(jobs);
    let class = ClassKey {
        dex: selected.image.dex,
        class_def: selected.class_def,
    };
    let method = match selector {
        Selector::Method(method_selector) => {
            let facts = engine.class_facts(class)?;
            Some(decompile::select_method(&facts, method_selector)?)
        }
        _ => None,
    };
    let depth = match expansion {
        Expansion::All => OutlineDepth::All,
        Expansion::Depth(0) => OutlineDepth::Members,
        Expansion::Depth(levels) => {
            OutlineDepth::Levels(u32::try_from(levels).map_err(|_| RequestError::PlatformCapacity)?)
        }
    };
    let entry = selected.image.entry.map(|entry| entry.0);
    let selection = Selection {
        entry,
        logical: selected.image.logical,
        header_offset: selected.image.header_offset,
        class,
        method,
        class_at: at::class(
            entry,
            selected.image.logical.0,
            class.class_def.0,
            &selected.descriptor,
        ),
        method_at: None,
    };
    let outline = engine.outline(class, method, depth)?;
    let mut nodes = Vec::with_capacity(outline.nodes.len());
    for node in outline.nodes {
        let at = match (node.kind, node.member) {
            (OutlineKind::Class, _) => Some(selection.class_at.clone()),
            (_, Some(member)) => locator(&images, member),
            _ => None,
        };
        nodes.push(Node {
            parent: node.parent,
            kind: node.kind.as_str(),
            text: node.text,
            at,
        });
    }
    Ok(Document {
        root: outline.root,
        nodes,
        issues: outline
            .issues
            .into_iter()
            .map(|issue| Issue {
                at: None,
                detail: issue.detail,
            })
            .collect(),
    })
}

fn locator(images: &ImageSet<'_>, member: OutlineMember) -> Option<String> {
    let image = images.images().get(member.class.dex.0 as usize)?;
    Some(at::method_index(
        image.entry.map(|entry| entry.0),
        image.logical.0,
        member.class.class_def.0,
        member.method.0,
    ))
}
