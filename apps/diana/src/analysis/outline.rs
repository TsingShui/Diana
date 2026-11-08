//! Navigation over the ordinary Engine/P9 products. No code/text reconstruction.
mod session;
mod targets;

use super::{
    at,
    decompile::{self, Decompiled, Selection},
    input::InputError,
    request::RequestScope,
    selector::{ClassSelector, MethodSelector, Selector},
};
use diana_decomp::{
    MethodId, RequestError,
    p0::Fact,
    p8,
    p9::{Origin, OutlineTarget, RenderOutcome, SourceProduct},
};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, thiserror::Error)]
pub enum OutlineError {
    #[error(transparent)]
    Decompile(#[from] decompile::DecompileError),
    #[error(transparent)]
    Input(#[from] InputError),
    #[error(transparent)]
    Request(#[from] RequestError),
    #[error("outline presentation invariant: {0}")]
    Invariant(&'static str),
}
impl OutlineError {
    pub fn code(&self) -> u8 {
        if let Self::Decompile(error) = self {
            error.code()
        } else {
            1
        }
    }
}

#[derive(Clone, Copy)]
pub enum Expansion {
    Depth(usize),
    All,
}
impl Expansion {
    fn allows(self, depth: usize) -> bool {
        match self {
            Self::Depth(max) => depth < max,
            Self::All => true,
        }
    }
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
    /// Index of the retained ordinary P9 publication, not the input DEX.
    pub publication: usize,
    pub start: usize,
    pub end: usize,
    pub origin: Option<Origin>,
}
#[derive(Serialize)]
pub struct Call {
    pub node: usize,
    pub caller: usize,
    pub pc: u32,
    pub origin_only: bool,
    pub abi_attachment: bool,
    #[serde(serialize_with = "dispatch")]
    pub dispatch: diana_decomp::p2::InvokeKind,
    pub target: Option<OutlineTarget>,
    pub body: Option<usize>,
    pub state: &'static str,
    pub uncertainty: Option<&'static str>,
}
fn dispatch<S: serde::Serializer>(
    value: &diana_decomp::p2::InvokeKind,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&format_args!("{value:?}"))
}
#[derive(Serialize)]
pub struct Issue {
    pub at: Option<String>,
    pub detail: String,
}

/// Both CLI presentations consume this arena. Source publications retain all
/// original facts and exact text for the ranges used by nodes.
#[derive(Serialize)]
pub struct Document {
    pub root: usize,
    pub complete: bool,
    pub nodes: Vec<Node>,
    pub calls: Vec<Call>,
    pub issues: Vec<Issue>,
    pub publications: Vec<Decompiled>,
}
struct Method {
    node: usize,
    class_at: String,
    class: ClassSelector,
    signature: Option<OutlineTarget>,
    depth: usize,
    discovered_from: Option<usize>,
}
struct Builder {
    document: Document,
    methods: Vec<Method>,
    known: HashMap<OutlineTarget, Vec<usize>>,
    queue: VecDeque<(usize, usize)>, // call index, caller method record
    expanded: Option<usize>,
}

pub(super) fn copied<T: Copy>(items: &[T]) -> Result<Vec<T>, RequestError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(items.len())
        .map_err(RequestError::Allocation)?;
    result.extend_from_slice(items);
    Ok(result)
}
pub(super) fn push<T>(to: &mut Vec<T>, value: T) -> Result<(), RequestError> {
    to.try_reserve(1).map_err(RequestError::Allocation)?;
    to.push(value);
    Ok(())
}
fn text(value: &str) -> Result<String, RequestError> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(RequestError::Allocation)?;
    result.push_str(value);
    Ok(result)
}
fn target_copy(target: &OutlineTarget) -> Result<OutlineTarget, RequestError> {
    Ok(OutlineTarget {
        owner: copied(&target.owner)?,
        name: copied(&target.name)?,
        prototype: copied(&target.prototype)?,
    })
}
fn class_copy(class: &ClassSelector) -> Result<ClassSelector, RequestError> {
    Ok(ClassSelector {
        entry: class.entry,
        logical: class.logical,
        class_def: class.class_def,
        descriptor: copied(&class.descriptor)?,
    })
}
fn facts(source: &SourceProduct) -> &diana_decomp::p0::ClassFacts {
    match source {
        SourceProduct::Class(c) => c.facts(),
        SourceProduct::Method(m) => m.facts(),
        SourceProduct::ClassStopped(s) | SourceProduct::MethodStopped(s) => s.facts(),
    }
}
fn members(source: &SourceProduct) -> &[p8::MethodMember] {
    match source {
        SourceProduct::Class(c) => c.methods(),
        SourceProduct::Method(m) => std::slice::from_ref(m.member()),
        SourceProduct::ClassStopped(s) | SourceProduct::MethodStopped(s) => s.methods(),
    }
}
fn view(result: &RenderOutcome) -> (&str, &SourceProduct, Option<&diana_decomp::p9::Outline>) {
    match result {
        RenderOutcome::Ready(r) => (r.text(), r.source(), r.outline()),
        RenderOutcome::Stopped(s) => (s.text_prefix(), s.source(), s.outline()),
    }
}
fn original_class(selection: &Selection, owner: &[u16]) -> Result<ClassSelector, OutlineError> {
    Ok(ClassSelector {
        entry: Some(
            u32::try_from(selection.entry.unwrap_or(0))
                .map_err(|_| RequestError::PlatformCapacity)?,
        ),
        logical: Some(selection.logical.0),
        class_def: Some(selection.class.class_def.0),
        descriptor: copied(owner)?,
    })
}

pub fn run(
    scope: &RequestScope,
    selector: &Selector,
    expansion: Expansion,
    jobs: usize,
) -> Result<Document, OutlineError> {
    let root = decompile::run_outline(scope, selector, jobs)?;
    let mut builder = Builder {
        document: Document {
            root: 0,
            complete: true,
            nodes: Vec::new(),
            calls: Vec::new(),
            issues: Vec::new(),
            publications: Vec::new(),
        },
        methods: Vec::new(),
        known: HashMap::new(),
        queue: VecDeque::new(),
        expanded: None,
    };
    builder.add(root, None, 0, None)?;
    let roots = if expansion.allows(0) && !builder.queue.is_empty() {
        match scope.roots() {
            Ok(roots) => roots,
            Err(error) => {
                builder.stop_expansion(format!("expansion input unavailable: {error}"))?;
                return Ok(builder.document);
            }
        }
    } else {
        Vec::new()
    };
    let mut session = match session::Session::new(&roots) {
        Ok(session) => session,
        Err(error) => {
            builder.stop_expansion(error.to_string())?;
            return Ok(builder.document);
        }
    };
    let mut targets = None;
    while let Some((edge, caller)) = builder.queue.pop_front() {
        builder.expand(&mut session, expansion, &mut targets, edge, caller)?;
    }
    builder.document.complete &= builder.document.issues.is_empty();
    Ok(builder.document)
}

impl Builder {
    fn node(
        &mut self,
        parent: Option<usize>,
        kind: &'static str,
        label: String,
        at: Option<String>,
        source: Option<SourceRange>,
    ) -> Result<usize, OutlineError> {
        let index = self.document.nodes.len();
        push(
            &mut self.document.nodes,
            Node {
                parent,
                kind,
                text: label,
                at,
                source,
            },
        )?;
        Ok(index)
    }
    fn issue(&mut self, at: Option<String>, detail: String) -> Result<(), OutlineError> {
        self.document.complete = false;
        push(&mut self.document.issues, Issue { at, detail })?;
        Ok(())
    }
    fn known_target(&self, edge: usize, class_at: &str) -> Result<Option<usize>, &'static str> {
        let candidates = self.document.calls[edge]
            .target
            .as_ref()
            .and_then(|s| self.known.get(s));
        let mut matches = candidates
            .into_iter()
            .flatten()
            .copied()
            .filter(|&n| self.methods[n].class_at == class_at);
        let first = matches.next();
        if matches.next().is_some() {
            Err("ambiguous original member declarations")
        } else {
            Ok(first)
        }
    }
    fn apply_known(
        &mut self,
        edge: usize,
        caller: usize,
        target: Result<Option<usize>, &'static str>,
    ) -> bool {
        match target {
            Ok(Some(target)) => {
                self.link(edge, caller, target);
                true
            }
            Ok(None) => false,
            Err(reason) => {
                self.document.complete = false;
                self.document.calls[edge].state = "ambiguous";
                self.document.calls[edge].uncertainty = Some(reason);
                true
            }
        }
    }
    fn link_local(&mut self, edge: usize, caller: usize) -> bool {
        let target = self.known_target(edge, &self.methods[caller].class_at);
        self.apply_known(edge, caller, target)
    }
    fn stop_expansion(&mut self, reason: String) -> Result<(), OutlineError> {
        while let Some((edge, caller)) = self.queue.pop_front() {
            if !self.link_local(edge, caller) {
                self.document.calls[edge].state = "unavailable";
                self.document.calls[edge].uncertainty =
                    Some("expansion input or shared scratch unavailable");
            }
        }
        self.issue(None, reason)
    }
    fn expanded_parent(&mut self) -> Result<usize, OutlineError> {
        if let Some(node) = self.expanded {
            return Ok(node);
        }
        let node = self.node(
            Some(self.document.root),
            "expansions",
            text("expanded methods (referenced declaration bodies)")?,
            None,
            None,
        )?;
        self.expanded = Some(node);
        Ok(node)
    }

    fn add(
        &mut self,
        publication: Decompiled,
        parent: Option<usize>,
        depth: usize,
        discovered_from: Option<usize>,
    ) -> Result<Vec<usize>, OutlineError> {
        let publication_id = self.document.publications.len();
        let (rendered_text, source, layout) = view(&publication.rendered);
        let layout = layout.ok_or(OutlineError::Invariant(
            "P9 did not capture requested outline facts",
        ))?;
        let class_facts = facts(source);
        let class = original_class(&publication.selection, class_facts.descriptor().units())?;
        let class_at = &publication.selection.class_at;
        let is_class = publication.selection.method.is_none();
        let base = if is_class {
            let header = layout
                .lines()
                .iter()
                .find(|line| matches!(line.header, Some(Origin::Class)));
            let label = header
                .and_then(|l| rendered_text.get(l.start.0..l.end.0))
                .unwrap_or(class_at)
                .trim();
            self.node(parent, "class", text(label)?, Some(text(class_at)?), None)?
        } else {
            parent.unwrap_or(self.document.root)
        };
        if is_class {
            self.document.root = base;
        }
        let mut line_nodes = Vec::new();
        line_nodes
            .try_reserve_exact(layout.lines().len())
            .map_err(RequestError::Allocation)?;
        line_nodes.resize(layout.lines().len(), None);
        let mut by_method: HashMap<MethodId, Vec<usize>> = HashMap::new();
        let mut outside = Vec::new();
        for (index, line) in layout.lines().iter().enumerate() {
            if let Some(method) = line.method {
                if !by_method.contains_key(&method) {
                    by_method.try_reserve(1).map_err(RequestError::Allocation)?;
                }
                push(by_method.entry(method).or_default(), index)?;
            } else if !matches!(line.header, Some(Origin::Class)) {
                push(&mut outside, index)?;
            }
        }
        if is_class {
            self.lines(
                rendered_text,
                layout,
                &outside,
                &mut line_nodes,
                base,
                publication_id,
            )?;
        }
        let mut added = Vec::new();
        let mut member_records = HashMap::new();
        for member in members(source) {
            let declaration = class_facts.method_entry(member.id).map(|m| m.declaration());
            let signature = if let Some(Fact::Present(d)) = declaration {
                Some(OutlineTarget {
                    owner: copied(class_facts.descriptor().units())?,
                    name: copied(d.name_units())?,
                    prototype: copied(d.prototype_units())?,
                })
            } else {
                None
            };
            let at = signature.as_ref().map(|s| {
                at::method(
                    publication.selection.entry,
                    publication.selection.logical.0,
                    publication.selection.class.class_def.0,
                    member.id.0,
                    &s.owner,
                    &s.name,
                    &s.prototype,
                )
            });
            let rows = by_method.get(&member.id).map_or(&[][..], Vec::as_slice);
            let header = rows
                .iter()
                .copied()
                .find(|&n| matches!(layout.lines()[n].header, Some(Origin::Method(_))));
            let label = if let Some(header) = header {
                text(
                    rendered_text
                        .get(layout.lines()[header].start.0..layout.lines()[header].end.0)
                        .ok_or(OutlineError::Invariant(
                            "method header range is unavailable",
                        ))?
                        .trim(),
                )?
            } else if let Some(at) = &at {
                let empty_initializer = matches!(source, SourceProduct::Class(class) if class.initializer_prints_nothing(member.id));
                format!(
                    "{} [{}]",
                    at.split_once("::")
                        .map_or(at.as_str(), |(_, reference)| reference),
                    if empty_initializer {
                        "empty initializer"
                    } else {
                        "display unavailable"
                    }
                )
            } else {
                format!("method#{} [declaration unavailable]", member.id.0)
            };
            let node = self.node(
                if is_class { Some(base) } else { parent },
                "method",
                label,
                at,
                None,
            )?;
            if !is_class && parent.is_none() {
                self.document.root = node;
            }
            let record = self.methods.len();
            if let Some(signature) = &signature {
                let key = target_copy(signature)?;
                if !self.known.contains_key(&key) {
                    self.known
                        .try_reserve(1)
                        .map_err(RequestError::Allocation)?;
                }
                push(self.known.entry(key).or_default(), record)?;
            }
            push(
                &mut self.methods,
                Method {
                    node,
                    class_at: text(class_at)?,
                    class: class_copy(&class)?,
                    signature,
                    depth,
                    discovered_from,
                },
            )?;
            push(&mut added, record)?;
            member_records
                .try_reserve(1)
                .map_err(RequestError::Allocation)?;
            member_records.insert(member.id, record);
            // Header and annotations stay associated with this original member;
            // the header itself is the method node, not a repeated code line.
            let mut body_rows = Vec::new();
            for &index in rows {
                if Some(index) == header {
                    line_nodes[index] = Some(node);
                } else {
                    push(&mut body_rows, index)?;
                }
            }
            self.lines(
                rendered_text,
                layout,
                &body_rows,
                &mut line_nodes,
                node,
                publication_id,
            )?;
            match &member.delivery {
                Ok(p8::MethodDelivery::Stopped(stop)) => self.issue(
                    Some(text(class_at)?),
                    format!(
                        "method#{} stopped at {:?} ({:?}); available fragments retained",
                        member.id.0, stop.stage, stop.cause_kind
                    ),
                )?,
                Err(error) => self.issue(
                    Some(text(class_at)?),
                    format!("method#{} delivery unavailable: {error}", member.id.0),
                )?,
                Ok(p8::MethodDelivery::Declared(_)) => {
                    self.node(
                        Some(node),
                        "declaration",
                        text("no code (native/abstract or declared member)")?,
                        None,
                        None,
                    )?;
                }
                _ => {}
            }
        }
        if !is_class && let Some(&record) = added.first() {
            self.lines(
                rendered_text,
                layout,
                &outside,
                &mut line_nodes,
                self.methods[record].node,
                publication_id,
            )?;
        }
        if !is_class && added.is_empty() {
            let node = self.node(
                parent,
                "method",
                text("selected method unavailable; retained publication follows")?,
                publication
                    .selection
                    .method_at
                    .as_deref()
                    .map(text)
                    .transpose()?,
                None,
            )?;
            if parent.is_none() {
                self.document.root = node;
            }
            self.lines(
                rendered_text,
                layout,
                &outside,
                &mut line_nodes,
                node,
                publication_id,
            )?;
            self.issue(
                Some(text(class_at)?),
                text("assembly did not deliver the selected member")?,
            )?;
        }
        for (index, line) in layout.lines().iter().enumerate() {
            if let (Some(Origin::Field(field)), Some(node)) = (line.header, line_nodes[index])
                && let Some(entry) = class_facts.field_entry(field)
                && let Fact::Present(declaration) = entry.declaration()
            {
                self.document.nodes[node].at = Some(at::field(
                    publication.selection.entry,
                    publication.selection.logical.0,
                    publication.selection.class.class_def.0,
                    field.0,
                    class_facts.descriptor().units(),
                    declaration.name.units(),
                    declaration.descriptor.units(),
                ));
            }
        }
        // Original PCs retain invocation/effect order inside a composed line.
        let mut calls: Vec<_> = Vec::new();
        calls
            .try_reserve_exact(layout.calls().len())
            .map_err(RequestError::Allocation)?;
        calls.extend(layout.calls());
        calls.sort_unstable_by_key(|call| (call.method, call.pc.0, call.position.0));
        for call in calls {
            let Some(&caller) = member_records.get(&call.method) else {
                continue;
            };
            let row = layout
                .lines()
                .partition_point(|line| line.start.0 <= call.position.0)
                .checked_sub(1);
            let parent = row
                .filter(|&n| !call.origin_only && layout.lines()[n].method == Some(call.method))
                .and_then(|n| line_nodes[n])
                .unwrap_or(self.methods[caller].node);
            let label = if let Some(target) = &call.target {
                // The locator encoder escapes arbitrary original UTF-16 units.
                let at = at::method(
                    None,
                    0,
                    0,
                    0,
                    &target.owner,
                    &target.name,
                    &target.prototype,
                );
                format!(
                    "call {} @0x{:x}",
                    at.split_once("::").map_or(at.as_str(), |(_, s)| s),
                    call.pc.0
                )
            } else {
                format!("call reference#{} @0x{:x}", call.reference.0, call.pc.0)
            };
            let node = self.node(Some(parent), "call", label, None, None)?;
            let edge = self.document.calls.len();
            let dynamic = matches!(
                call.dispatch,
                diana_decomp::p2::InvokeKind::Virtual
                    | diana_decomp::p2::InvokeKind::Interface
                    | diana_decomp::p2::InvokeKind::Super
                    | diana_decomp::p2::InvokeKind::Polymorphic
                    | diana_decomp::p2::InvokeKind::Custom
            );
            push(
                &mut self.document.calls,
                Call {
                    node,
                    caller: self.methods[caller].node,
                    pc: call.pc.0,
                    origin_only: call.origin_only,
                    abi_attachment: call.abi_attachment,
                    dispatch: call.dispatch,
                    target: call.target.as_ref().map(target_copy).transpose()?,
                    body: None,
                    state: "not_expanded",
                    uncertainty: dynamic
                        .then_some("referenced declaration only; runtime target not established"),
                },
            )?;
            self.queue
                .try_reserve(1)
                .map_err(RequestError::Allocation)?;
            self.queue.push_back((edge, caller));
        }
        match &publication.rendered {
            RenderOutcome::Ready(ready) => {
                for issue in ready.issues() {
                    self.issue(
                        Some(text(class_at)?),
                        format!("rendering {:?}: {}", issue.kind, issue.detail),
                    )?;
                }
            }
            RenderOutcome::Stopped(stop) => self.issue(
                Some(text(class_at)?),
                format!(
                    "rendering stopped ({:?}); prefix retained: {}",
                    stop.error().kind(),
                    stop.error()
                ),
            )?,
        }
        for issue in [
            class_facts.fields_issue(),
            class_facts.methods_issue(),
            class_facts.annotations_issue(),
        ]
        .into_iter()
        .flatten()
        {
            self.issue(
                Some(text(class_at)?),
                format!("class facts incomplete: {issue}"),
            )?;
        }
        push(&mut self.document.publications, publication)?;
        Ok(added)
    }

    fn lines(
        &mut self,
        rendered: &str,
        layout: &diana_decomp::p9::Outline,
        rows: &[usize],
        line_nodes: &mut [Option<usize>],
        root: usize,
        publication: usize,
    ) -> Result<(), OutlineError> {
        let mut scopes: Vec<(usize, usize)> = Vec::new();
        for &index in rows {
            let line = &layout.lines()[index];
            let label = rendered
                .get(line.start.0..line.end.0)
                .ok_or(OutlineError::Invariant("outline line range is unavailable"))?
                .trim();
            if label.is_empty() || matches!(line.kind, "package" | "import") {
                continue;
            }
            while scopes.last().is_some_and(|&(depth, _)| depth >= line.depth) {
                scopes.pop();
            }
            let parent = scopes.last().map_or(root, |&(_, node)| node);
            let node = self.node(
                Some(parent),
                if matches!(line.header, Some(Origin::Field(_))) {
                    "field"
                } else {
                    line.kind
                },
                text(label)?,
                None,
                Some(SourceRange {
                    publication,
                    start: line.start.0,
                    end: line.end.0,
                    origin: line.header,
                }),
            )?;
            line_nodes[index] = Some(node);
            push(&mut scopes, (line.depth, node))?;
        }
        Ok(())
    }

    fn expand(
        &mut self,
        session: &mut session::Session<'_, '_>,
        expansion: Expansion,
        targets: &mut Option<targets::Targets>,
        edge: usize,
        caller: usize,
    ) -> Result<(), OutlineError> {
        if self.document.calls[edge].target.is_none() {
            self.document.calls[edge].state = "unresolved_target";
            return Ok(());
        }
        if matches!(
            self.document.calls[edge].dispatch,
            diana_decomp::p2::InvokeKind::Custom | diana_decomp::p2::InvokeKind::Polymorphic
        ) {
            self.document.calls[edge].state = "dynamic_target";
            return Ok(());
        }
        // Existing class members keep their fixed member position. This link is
        // useful even when expansion depth is zero and costs no new analysis.
        if self.link_local(edge, caller) {
            return Ok(());
        }
        if !expansion.allows(self.methods[caller].depth) {
            self.document.calls[edge].state = "depth_boundary";
            return Ok(());
        }
        let signature = target_copy(
            self.document.calls[edge]
                .target
                .as_ref()
                .ok_or(OutlineError::Invariant("call target disappeared"))?,
        )?;
        let local = self.methods[caller]
            .signature
            .as_ref()
            .is_some_and(|s| s.owner == signature.owner);
        let class = if local {
            class_copy(&self.methods[caller].class)?
        } else {
            if targets.is_none() {
                *targets = Some(
                    match targets::Targets::read(session.roots(), &mut self.document.issues) {
                        Ok(index) => index,
                        Err(error) => {
                            self.issue(None, format!("declaration index unavailable: {error}"))?;
                            targets::Targets::unavailable()
                        }
                    },
                );
            }
            let resolved = targets
                .as_ref()
                .ok_or(OutlineError::Invariant("declaration index not delivered"))?
                .resolve(&signature.owner);
            match resolved {
                Ok(class) => class_copy(class)?,
                Err(reason) => {
                    self.document.calls[edge].state =
                        if reason == "no owner definition in the input" {
                            "external"
                        } else {
                            "unavailable"
                        };
                    if self.document.calls[edge].state == "unavailable" {
                        self.issue(None, text(reason)?)?;
                    }
                    self.document.calls[edge].uncertainty = Some(reason);
                    return Ok(());
                }
            }
        };
        let class_at = at::class(
            class.entry.map(|v| v as usize),
            class.logical.unwrap_or(0),
            class.class_def.unwrap_or(0),
            &class.descriptor,
        );
        let target = self.known_target(edge, &class_at);
        if self.apply_known(edge, caller, target) {
            return Ok(());
        }
        let selector = MethodSelector {
            class,
            method: None,
            name: copied(&signature.name)?,
            prototype: copied(&signature.prototype)?,
        };
        let result = session.analyze(&selector);
        match result {
            Ok(result) => {
                let depth = self.methods[caller]
                    .depth
                    .checked_add(1)
                    .ok_or(RequestError::PlatformCapacity)?;
                let parent = self.expanded_parent()?;
                let added = self.add(result, Some(parent), depth, Some(caller))?;
                if let Some(&target) = added.first() {
                    self.link(edge, caller, target);
                } else {
                    self.document.calls[edge].state = "unavailable";
                    self.issue(
                        Some(class_at),
                        text("selected method body was not delivered")?,
                    )?;
                }
            }
            Err(error) => {
                self.document.calls[edge].state = "unavailable";
                self.issue(
                    Some(class_at),
                    format!("target declaration/body unavailable: {error}"),
                )?;
            }
        }
        Ok(())
    }
    fn link(&mut self, edge: usize, caller: usize, target: usize) {
        let mut ancestor = Some(caller);
        let mut recursive = false;
        while let Some(index) = ancestor {
            if index == target {
                recursive = true;
                break;
            }
            ancestor = self.methods[index].discovered_from;
        }
        self.document.calls[edge].body = Some(self.methods[target].node);
        self.document.calls[edge].state = if recursive {
            "recursive_reference"
        } else {
            "body_reference"
        };
    }
}
