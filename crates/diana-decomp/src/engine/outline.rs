//! Call-tree outline assembly over P0 declarations and P1 invoke sites.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ir::{InsnType, MemberReference, MethodReference};
use crate::outline::{
    OutlineDepth, OutlineDocument, OutlineIssue, OutlineKind, OutlineMember, OutlineNode,
};
use crate::p0::{ClassFacts, Fact};
use crate::{ClassDefId, ClassKey, MethodId, RequestError};

use super::Engine;

/// Bounds how many method bodies one outline decodes before it stops expanding.
const MAX_BODIES: usize = 4096;

pub(super) fn outline(
    engine: &Engine<'_>,
    class: ClassKey,
    method: Option<MethodId>,
    depth: OutlineDepth,
) -> Result<OutlineDocument, RequestError> {
    let image = engine.image(class.dex).ok_or(RequestError::UnknownDex)?;
    let facts = image.index.class_facts_cached(class.class_def)?;
    let mut builder = Builder {
        engine,
        class,
        nodes: Vec::new(),
        issues: Vec::new(),
        imports: BTreeMap::new(),
        resolved: BTreeMap::new(),
        expanded: BTreeMap::new(),
        bodies: 0,
        truncated: false,
    };
    let class_text = String::from_utf16_lossy(facts.descriptor().units());
    let (root, imports_parent) = match method {
        Some(method) => {
            let node = builder.method_node(&facts, method, None);
            (node, node)
        }
        None => {
            let node = builder.push(None, OutlineKind::Class, class_text, None);
            (node, node)
        }
    };
    let imports_node = builder.push(
        Some(imports_parent),
        OutlineKind::Imports,
        "imports".to_owned(),
        None,
    );
    match method {
        Some(method) => builder.expand(&facts, method, root, depth),
        None => {
            let methods = facts
                .methods()
                .iter()
                .map(|entry| entry.method())
                .collect::<Vec<_>>();
            for method in methods {
                builder.method(&facts, method, root, depth);
            }
        }
    }
    let imports = std::mem::take(&mut builder.imports);
    let mut trie = PackageTrie::default();
    for (package, types) in imports {
        for (name, methods) in types {
            trie.insert(&package, &name, methods);
        }
    }
    trie.emit(&mut builder, imports_node);
    Ok(OutlineDocument {
        root,
        nodes: builder.nodes,
        issues: builder.issues,
    })
}

/// Package to type to method to call count.
type Imports = BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>;
/// One resolved call target: image position, class_def and method.
type Resolution = Option<((usize, ClassDefId), MethodId)>;
/// Call target key: owner descriptor, method name and prototype.
type ResolutionKey = (String, String, String);

struct Builder<'a, 'i> {
    engine: &'i Engine<'a>,
    class: ClassKey,
    nodes: Vec<OutlineNode>,
    issues: Vec<OutlineIssue>,
    imports: Imports,
    resolved: BTreeMap<ResolutionKey, Resolution>,
    expanded: BTreeMap<(u32, u32, u32), usize>,
    bodies: usize,
    truncated: bool,
}

impl Builder<'_, '_> {
    fn push(
        &mut self,
        parent: Option<usize>,
        kind: OutlineKind,
        text: String,
        member: Option<OutlineMember>,
    ) -> usize {
        self.nodes.push(OutlineNode {
            parent,
            kind,
            text,
            member,
        });
        self.nodes.len() - 1
    }

    fn method(&mut self, facts: &ClassFacts, method: MethodId, parent: usize, depth: OutlineDepth) {
        let node = self.method_node(facts, method, Some(parent));
        self.expand(facts, method, node, depth);
    }

    fn method_node(
        &mut self,
        facts: &ClassFacts,
        method: MethodId,
        parent: Option<usize>,
    ) -> usize {
        let text = facts
            .method_entry(method)
            .and_then(|entry| match entry.declaration() {
                Fact::Present(declaration) => Some((
                    String::from_utf16_lossy(declaration.name_units()),
                    String::from_utf16_lossy(declaration.prototype_units()),
                )),
                Fact::Unavailable(_) | Fact::Absent | Fact::NotRead => None,
            })
            .map_or_else(
                || {
                    self.issue(format!("method#{} declaration is unavailable", method.0));
                    format!("method#{}", method.0)
                },
                |(name, prototype)| simple_signature(&name, &prototype),
            );
        self.push(
            parent,
            OutlineKind::Method,
            text,
            Some(OutlineMember {
                class: self.class,
                method,
            }),
        )
    }

    fn expand(&mut self, facts: &ClassFacts, method: MethodId, node: usize, depth: OutlineDepth) {
        // Imports are a class-level fact, so a members-only outline still scans
        // one level of invoke sites; it just does not grow the tree.
        let collect_only = depth.budget() == 0;
        let key = (self.class.dex.0, self.class.class_def.0, method.0);
        if let Some(first) = self.expanded.get(&key) {
            self.nodes[node].text.push_str(&format!(" -> see #{first}"));
            self.nodes[node].kind = OutlineKind::Link;
            return;
        }
        self.expanded.insert(key, node);
        if self.bodies >= MAX_BODIES {
            if !self.truncated {
                self.truncated = true;
                self.issue(format!(
                    "outline expansion stopped after {MAX_BODIES} method bodies"
                ));
            }
            return;
        }
        self.bodies += 1;
        let Some(image) = self.engine.image(self.class.dex) else {
            return;
        };
        let source = match image.index.method_source(facts, method) {
            Ok(source) => source,
            Err(error) => {
                self.issue(format!("method#{} body is unavailable: {error}", method.0));
                return;
            }
        };
        let decoded = match image.index.decode_method_source(&source) {
            Ok(decoded) => decoded,
            Err(error) => {
                self.issue(format!(
                    "method#{} body failed to decode: {error}",
                    method.0
                ));
                return;
            }
        };
        let Some(decoded) = decoded.decoded else {
            let detail = decoded.error.map_or_else(
                || "method decoding stopped".to_owned(),
                |error| error.to_string(),
            );
            self.issue(format!("method#{} body is unavailable: {detail}", method.0));
            return;
        };
        let Some(ir) = decoded.ir() else {
            let detail = decoded
                .ir_error()
                .unwrap_or("P1 produced no recovery IR")
                .to_owned();
            self.issue(format!("method#{} body is unavailable: {detail}", method.0));
            return;
        };
        for insn in ir.instructions() {
            if insn.insn_type != InsnType::Invoke {
                continue;
            }
            let Some(MemberReference::Method(reference)) = insn.payload.reference.as_deref() else {
                continue;
            };
            if collect_only {
                self.collect_import(reference);
            } else {
                self.call(reference, node, depth.next());
            }
        }
    }

    fn collect_import(&mut self, reference: &MethodReference) {
        let owner = reference.owner.to_descriptor();
        if self.engine.class_location(&owner).is_none() && platform_owner(&owner) {
            self.record_import(&owner, &reference.name);
        }
    }

    fn call(&mut self, reference: &MethodReference, parent: usize, depth: OutlineDepth) {
        let owner = reference.owner.to_descriptor();
        let name = reference.name.clone();
        let prototype = reference.descriptor.to_string();
        let text = format!(
            "{}.{}",
            simple_owner(&owner),
            simple_signature(&name, &prototype)
        );
        let Some(location) = self.engine.class_location(&owner) else {
            if platform_owner(&owner) {
                self.record_import(&owner, &name);
            } else {
                self.push(
                    Some(parent),
                    OutlineKind::Call,
                    format!("{text} [other-dex]"),
                    None,
                );
            }
            return;
        };
        let key = (owner.clone(), name.clone(), prototype.clone());
        let resolved = if let Some(hit) = self.resolved.get(&key) {
            *hit
        } else {
            let resolved = self.resolve_in_hierarchy(location, &name, &prototype);
            self.resolved.insert(key, resolved);
            resolved
        };
        let Some((declaring, method)) = resolved else {
            self.push(
                Some(parent),
                OutlineKind::Call,
                format!("{text} [unresolved]"),
                None,
            );
            return;
        };
        let Some(image) = self.engine.image_at(declaring.0) else {
            return;
        };
        let class = ClassKey {
            dex: image.dex,
            class_def: declaring.1,
        };
        let node = self.push(
            Some(parent),
            OutlineKind::Call,
            text,
            Some(OutlineMember { class, method }),
        );
        let Ok(facts) = image.index.class_facts_cached(declaring.1) else {
            return;
        };
        let saved = self.class;
        self.class = class;
        self.expand(&facts, method, node, depth);
        self.class = saved;
    }

    fn resolve_in_hierarchy(
        &self,
        start: (usize, ClassDefId),
        name: &str,
        prototype: &str,
    ) -> Option<((usize, ClassDefId), MethodId)> {
        let name_units = name.encode_utf16().collect::<Vec<_>>();
        let prototype_units = prototype.encode_utf16().collect::<Vec<_>>();
        let mut pending = VecDeque::from([start]);
        let mut seen = BTreeSet::new();
        while let Some((position, class_def)) = pending.pop_front() {
            if !seen.insert((position as u32, class_def.0)) {
                continue;
            }
            let Some(image) = self.engine.image_at(position) else {
                continue;
            };
            let Ok(facts) = image.index.class_facts_cached(class_def) else {
                continue;
            };
            if let Ok(method) = facts.resolve_method(&name_units, &prototype_units) {
                return Some(((position, class_def), method));
            }
            if let Fact::Present(super_class) = facts.superclass() {
                let descriptor = String::from_utf16_lossy(super_class.units());
                if let Some(next) = self.engine.class_location(&descriptor) {
                    pending.push_back(next);
                }
            }
            if let Fact::Present(interfaces) = facts.interfaces() {
                for interface in interfaces {
                    let Fact::Present(descriptor) = &interface.descriptor else {
                        continue;
                    };
                    let descriptor = String::from_utf16_lossy(descriptor.units());
                    if let Some(next) = self.engine.class_location(&descriptor) {
                        pending.push_back(next);
                    }
                }
            }
        }
        None
    }

    fn record_import(&mut self, owner: &str, method: &str) {
        let (package, name) = split_owner(owner);
        *self
            .imports
            .entry(package.to_owned())
            .or_default()
            .entry(name.to_owned())
            .or_default()
            .entry(method.to_owned())
            .or_default() += 1;
    }

    fn issue(&mut self, detail: String) {
        self.issues.push(OutlineIssue { detail });
    }
}

fn split_owner(descriptor: &str) -> (&str, &str) {
    let name = descriptor
        .strip_prefix('L')
        .and_then(|value| value.strip_suffix(';'))
        .unwrap_or(descriptor);
    match name.rsplit_once('/') {
        Some((package, simple)) => (package, simple),
        None => ("", name),
    }
}

fn simple_owner(descriptor: &str) -> &str {
    split_owner(descriptor).1
}

/// Package tree over external owners, so imports read as nested segments with
/// one aggregated leaf per referenced type.
#[derive(Default)]
struct PackageTrie {
    children: BTreeMap<String, PackageTrie>,
    types: BTreeMap<String, BTreeMap<String, usize>>,
}

impl PackageTrie {
    fn insert(&mut self, package: &str, name: &str, methods: BTreeMap<String, usize>) {
        let mut node = self;
        if !package.is_empty() {
            for segment in package.split('/') {
                node = node.children.entry(segment.to_owned()).or_default();
            }
        }
        node.types.insert(name.to_owned(), methods);
    }

    fn emit(&self, builder: &mut Builder<'_, '_>, parent: usize) {
        for (segment, child) in &self.children {
            let node = builder.push(Some(parent), OutlineKind::Import, segment.clone(), None);
            child.emit(builder, node);
        }
        for (name, methods) in &self.types {
            builder.push(
                Some(parent),
                OutlineKind::Import,
                type_text(name, methods),
                None,
            );
        }
    }
}

fn type_text(name: &str, methods: &BTreeMap<String, usize>) -> String {
    let mut text = format!("{name} {{ ");
    let mut first = true;
    for (method, count) in methods {
        if !first {
            text.push_str(", ");
        }
        first = false;
        text.push_str(method);
        if *count > 1 {
            text.push_str(&format!(" x{count}"));
        }
    }
    text.push_str(" }");
    text
}

/// Platform packages end the call tree; everything else is app code even when
/// it lives in another image of the same APK.
fn platform_owner(descriptor: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "android/",
        "androidx/",
        "java/",
        "javax/",
        "kotlin/",
        "kotlinx/",
        "org/jetbrains/",
        "org/intellij/",
        "org/w3c/",
        "org/xml/",
        "org/json/",
        "dalvik/",
        "sun/",
        "com/android/",
        "com/google/android/",
        "junit/",
        "org/junit/",
    ];
    let name = descriptor
        .strip_prefix('L')
        .and_then(|value| value.strip_suffix(';'))
        .unwrap_or(descriptor);
    PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// Renders a JVM descriptor as a readable signature with simple type names.
fn simple_signature(name: &str, descriptor: &str) -> String {
    let Some((parameters, returns)) = split_prototype(descriptor) else {
        return format!("{name}{descriptor}");
    };
    let mut text = format!("{name}(");
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            text.push_str(", ");
        }
        text.push_str(parameter);
    }
    text.push(')');
    if returns != "void" {
        text.push_str(": ");
        text.push_str(&returns);
    }
    text
}

fn split_prototype(descriptor: &str) -> Option<(Vec<String>, String)> {
    let rest = descriptor.strip_prefix('(')?;
    let end = rest.find(')')?;
    let parameters = parse_types(&rest[..end])?;
    let (returns, tail) = simple_type_at(&rest[end + 1..])?;
    tail.is_empty().then_some((parameters, returns))
}

fn parse_types(mut text: &str) -> Option<Vec<String>> {
    let mut types = Vec::new();
    while !text.is_empty() {
        let (ty, rest) = simple_type_at(text)?;
        types.push(ty);
        text = rest;
    }
    Some(types)
}

fn simple_type_at(text: &str) -> Option<(String, &str)> {
    let mut dimensions = 0;
    let mut index = 0;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'[' => {
                dimensions += 1;
                index += 1;
            }
            b'L' => {
                let end = text[index..].find(';')? + index;
                let name = &text[index + 1..end];
                let simple = name.rsplit('/').next().unwrap_or(name);
                let suffix = "[]".repeat(dimensions);
                return Some((format!("{simple}{suffix}"), &text[end + 1..]));
            }
            byte => {
                let ty = match byte {
                    b'Z' => "boolean",
                    b'B' => "byte",
                    b'C' => "char",
                    b'S' => "short",
                    b'I' => "int",
                    b'J' => "long",
                    b'F' => "float",
                    b'D' => "double",
                    b'V' => "void",
                    _ => return None,
                };
                let suffix = "[]".repeat(dimensions);
                return Some((format!("{ty}{suffix}"), &text[index + 1..]));
            }
        }
    }
    None
}
