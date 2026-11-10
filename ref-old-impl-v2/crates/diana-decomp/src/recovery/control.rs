use crate::p2::{BlockId, OpId};
use crate::p6::{LinkId as TransferId, RegionId as ControlId};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Predicate {
    pub terminal: BlockId,
    /// Copies belong to selection of these original edges, never to a common
    /// loop continuation. A break does not execute the predicate-exit copies.
    pub when_true: TransferId,
    pub when_false: TransferId,
    pub negate: bool,
}

#[derive(Clone, Debug, Serialize)]
pub enum LoopPlacement {
    Infinite,
    While {
        predicate: ControlId,
        test: Predicate,
        compact: bool,
    },
    DoWhile {
        predicate: ControlId,
        test: Predicate,
        compact: bool,
    },
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum IfShape {
    IfElse,
    /// One arm ends in an established return/throw. The other edge's copies
    /// and complete body follow the guard in the enclosing lexical scope.
    Guard {
        inverted: bool,
    },
    ElseIf,
    /// The false edge and its original body become the sole printed arm. Any
    /// true-edge copy bundles have no steps or diagnostics, only source anchors.
    Inverted,
    /// Only the header remains; evaluating the predicate has no displayed work.
    Empty,
}

/// The last condition test's original other edge either reaches the common
/// follow, executes its boundary transfer, or enters its delivered body.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ConditionElse {
    None,
    Copies,
    /// Execute the original final other edge's copies and boundary completion.
    Transfer,
    Body(ControlId),
}

#[derive(Clone, Debug, Serialize)]
pub enum ControlLayout {
    Block(crate::p6::NodeId),
    Sequence,
    /// Source attachments use the ordinary child grammar. The mode changes
    /// presentation only; an ABI/origin attachment is not another execution.
    Scope {
        body: ControlId,
        mode: SourceMode,
        heading: Option<&'static str>,
    },
    If {
        header: ControlId,
        yes: ControlId,
        no: ControlId,
        test: Predicate,
        shape: IfShape,
    },
    /// Several original predicates print as one short-circuit condition. The
    /// shared arm prints on `select`; the optional other arm prints otherwise.
    Condition {
        header: ControlId,
        tests: SharedList<Predicate>,
        yes: ControlId,
        no: ConditionElse,
        fall: TransferId,
        /// P6-owned empty goto controls retain origins without printed routing.
        routing: Option<ControlId>,
    },
    /// A P6-owned alternative over the original pure integer tests.
    When {
        bodies: SharedList<ControlId>,
    },
    Loop {
        continue_target: crate::p6::NodeId,
        body: ControlId,
        placement: LoopPlacement,
    },
    Switch {
        header: ControlId,
        bodies: SharedList<ControlId>,
        cases: Option<SharedList<Vec<TransferId>>>,
        terminal: BlockId,
    },
    Labelled,
    Try {
        body: ControlId,
        catches: SharedList<ControlId>,
        finally: Option<Finally>,
    },
    /// Explicit planned exits prevent implicit second releases, including when
    /// an unlocked routing terminal remains within its original nested catch.
    Synchronized {
        implicit: bool,
        entry: ControlId,
        body: ControlId,
        acquire: OpId,
    },
    Graph,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum SourceMode {
    Code,
    Abi,
    Origins,
}

/// P6 publishes complete original-operation leaves and selected child roles.
/// `source` retains an existing semantic region; compounds need no new region
/// recognizer or duplicate source-membership tree.
#[derive(Clone, Debug, Serialize)]
pub struct ControlRecord {
    pub layout: ControlLayout,
    pub children: SharedList<ControlId>,
    pub source: Option<ControlId>,
    pub source_only: SharedList<ControlId>,
    pub end: Option<BodyEnd>,
    pub continuation: Option<crate::p6::Continuation>,
    pub entry: Option<crate::p6::NodeId>,
    pub follow: Option<crate::p6::NodeId>,
}

#[derive(Clone, Debug, Serialize)]
pub enum BodyEnd {
    None,
    Terminal(BlockId),
    /// Original terminal correspondence followed by the supplied normal edge.
    /// P6 establishes the projection; P7/P9 do not rematch the protocol.
    Annotated {
        terminal: BlockId,
        mode: SourceMode,
        normal: TransferId,
        suspended: TransferId,
    },
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Finally {
    pub cleanup: ControlId,
    pub duplicate: ControlId,
}

/// Immutable structure lists shared by the P6 delivery and owned P7 product.
/// Cloning this handle neither clones elements nor borrows a predecessor.
#[derive(Debug)]
pub struct SharedList<T>(Arc<Vec<T>>);
impl<T> Clone for SharedList<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T> From<Vec<T>> for SharedList<T> {
    fn from(items: Vec<T>) -> Self {
        Self(Arc::new(items))
    }
}
impl<T> std::ops::Deref for SharedList<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T> IntoIterator for &'a SharedList<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<T: Serialize> Serialize for SharedList<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}
impl<T: Copy> SharedList<T> {
    pub(crate) fn truncate(&mut self, len: usize) -> Result<(), crate::RequestError> {
        if let Some(items) = Arc::get_mut(&mut self.0) {
            items.truncate(len);
        } else {
            let source = &self[..len.min(self.len())];
            let mut items = Vec::new();
            crate::substrate::owned::reserve(&mut items, source.len())?;
            items.extend_from_slice(source);
            self.0 = Arc::new(items);
        }
        Ok(())
    }
}
