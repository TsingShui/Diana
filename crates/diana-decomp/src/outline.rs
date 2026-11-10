//! Call-tree outline over P0 declarations and P1 invoke sites.
//!
//! The outline answers one question: which same-image methods does a class or
//! method reach, and how deep. External calls are aggregated per owner type
//! under an imports node; they never enter the call tree.

use crate::{ClassKey, MethodId};

/// How far one outline expands call edges.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutlineDepth {
    /// Declared members only; no call expansion.
    Members,
    /// Expands this many call levels below each method.
    Levels(u32),
    /// Expands every reachable same-image method once.
    All,
}

impl OutlineDepth {
    pub(crate) fn budget(self) -> u32 {
        match self {
            Self::Members => 0,
            Self::Levels(levels) => levels,
            Self::All => u32::MAX,
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Members => Self::Members,
            Self::Levels(levels) => Self::Levels(levels.saturating_sub(1)),
            Self::All => Self::All,
        }
    }
}

/// The original identity a tree node can be opened with.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutlineMember {
    pub class: ClassKey,
    pub method: MethodId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutlineKind {
    Class,
    Imports,
    Import,
    Method,
    Call,
    /// A repeated or cyclic method that points at its first expansion.
    Link,
}

impl OutlineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Imports => "imports",
            Self::Import => "import",
            Self::Method => "method",
            Self::Call => "call",
            Self::Link => "link",
        }
    }
}

#[derive(Debug)]
pub struct OutlineNode {
    pub parent: Option<usize>,
    pub kind: OutlineKind,
    pub text: String,
    pub member: Option<OutlineMember>,
}

#[derive(Debug)]
pub struct OutlineIssue {
    pub detail: String,
}

#[derive(Debug)]
pub struct OutlineDocument {
    pub root: usize,
    pub nodes: Vec<OutlineNode>,
    pub issues: Vec<OutlineIssue>,
}
