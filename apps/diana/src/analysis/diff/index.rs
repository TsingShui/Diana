//! One physical-input pass. Each worker releases its DEX facts after that root.
use crate::analysis::archive::ExportReader;
use diana_decomp::input::{DexContainer, ProgramIndex, RawPrintable};
use diana_decomp::p0::{self, Fact};
use diana_decomp::{ClassDefId, ClassKey, DexId};

use crate::analysis::apk::DexRoot;
use crate::analysis::{at, images::ImageError, request::RequestScope, runner};

mod blocks;
mod body;
mod compare;
pub(super) mod jvm;
mod protection_diff;
mod reference;
mod symbols;
pub(super) use symbols::Symbols;

// The same code item or long prototype may be referenced by arbitrarily many
// directory entries. All logical members backed by one inflated root share a
// retained-size allowance; malformed roots cannot exhaust independent roots.
const ROOT_CAPACITY: &str = "physical DEX index expansion capacity exhausted";
struct ExpansionBudget {
    remaining: usize,
}
impl ExpansionBudget {
    fn charge(&mut self, bytes: usize) -> bool {
        let remaining = self.remaining.checked_sub(bytes);
        self.remaining = remaining.unwrap_or(0);
        remaining.is_some()
    }
}

#[derive(Debug)]
pub(super) struct Index<T = Method> {
    pub methods: Vec<T>,
    /// Root, image or class errors mean the declaration inventory is incomplete.
    pub issues: Vec<InventoryIssue>,
}

#[derive(Debug)]
pub(super) struct InventoryIssue {
    pub at: Option<String>,
    pub reason: String,
}

impl From<String> for InventoryIssue {
    fn from(reason: String) -> Self {
        Self { at: None, reason }
    }
}

#[derive(Debug)]
pub(super) struct JvmDeclaration {
    /// Outer None means the original optional metadata was unavailable.
    pub signature: Option<Option<String>>,
    pub exceptions: Option<Vec<String>>,
}

impl Default for JvmDeclaration {
    fn default() -> Self {
        Self {
            signature: Some(None),
            exceptions: Some(Vec::new()),
        }
    }
}

impl JvmDeclaration {
    fn is_empty(&self) -> bool {
        matches!(self.signature, Some(None)) && self.exceptions.as_ref().is_some_and(Vec::is_empty)
    }
}

impl<T> Default for Index<T> {
    fn default() -> Self {
        Self {
            methods: Vec::new(),
            issues: Vec::new(),
        }
    }
}

/// Only the already-corresponding locations and diagnostics must survive when
/// both input leases contain exactly the same physical bytes.
pub(super) struct InventoryMethod {
    pub at: String,
    pub signature: String,
    pub issues: Vec<String>,
}

#[derive(Debug)]
pub(super) struct Method {
    pub at: String,
    pub owner: String,
    pub name: String,
    pub prototype: String,
    pub access: u32,
    pub jvm: Option<Box<JvmDeclaration>>,
    pub body: Option<Body>,
    /// Empty on a fully inspected body or a valid native/abstract declaration.
    pub issues: Vec<String>,
}

impl Method {
    fn retained_bytes(&self) -> usize {
        let declarations = size_of::<Self>()
            + self.at.capacity()
            + self.owner.capacity()
            + self.name.capacity()
            + self.prototype.capacity();
        let diagnostics = self.issues.capacity() * size_of::<String>()
            + self.issues.iter().map(String::capacity).sum::<usize>();
        let body = self.body.as_ref().map_or(0, |b| {
            b.code.capacity() * size_of::<Op>()
                + b.operands.capacity()
                + b.raw_registers.capacity() * size_of::<u16>()
                + b.protection.capacity()
                + b.calls.capacity() * size_of::<u32>()
                + b.anchors.capacity() * size_of::<u64>()
        });
        let metadata = self.jvm.as_ref().map_or(0, |declaration| {
            size_of::<JvmDeclaration>()
                + declaration
                    .signature
                    .as_ref()
                    .and_then(Option::as_ref)
                    .map_or(0, String::capacity)
                + declaration.exceptions.as_ref().map_or(0, |entries| {
                    entries.capacity() * size_of::<String>()
                        + entries.iter().map(String::capacity).sum::<usize>()
                })
        });
        declarations + diagnostics + body + metadata
    }

    pub(super) fn identity(&self) -> String {
        format!("{}->{}{}", self.owner, self.name, self.prototype)
    }
}

#[derive(Debug, Default)]
pub(super) struct Body {
    pub format: Format,
    pub code: Vec<Op>,
    pub operands: Vec<u8>,
    raw_registers: Vec<u16>,
    pub frame: [u16; 3],
    pub jvm_major: u16,
    code_units: u32,
    pub protection: Vec<u8>,
    pub calls: Vec<u32>,
    names: symbols::Names,
    pub anchors: Vec<u64>,
    has_symbol_dependency: bool,
    pub(super) complete_control: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Format {
    #[default]
    Dex,
    Jvm,
}

impl Format {
    pub fn code(self) -> &'static str {
        match self {
            Self::Dex => "DEX code",
            Self::Jvm => "JVM Code",
        }
    }

    fn switch(self, opcode: u8) -> bool {
        match self {
            Self::Dex => matches!(opcode, 0x2b | 0x2c),
            Self::Jvm => matches!(opcode, 0xaa | 0xab),
        }
    }
}

impl Body {
    pub fn has_owner_member_ref(&self) -> bool {
        self.has_symbol_dependency
    }

    pub fn call_names(&self) -> impl Iterator<Item = &str> {
        self.calls.iter().filter_map(|&id| self.names.get(id))
    }

    fn symbol(&self, data: &[u8]) -> Option<&str> {
        self.names.get(u32::from_le_bytes(data.try_into().ok()?))
    }

    pub fn operand(&self, index: usize) -> &[u8] {
        let start = index
            .checked_sub(1)
            .map_or(0, |prev| self.code[prev].end as usize);
        &self.operands[start..self.code[index].end as usize]
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Op {
    pub pc: u32,
    pub opcode: u8,
    pub shape: u16,
    pub end: u32,
    register_end: u32,
    flow: blocks::Flow,
    pub(super) known_operands: bool,
}

pub(super) fn build(
    scope: &RequestScope,
    jobs: usize,
    symbols: &Symbols,
) -> Result<Index, ImageError> {
    build_with(scope, jobs, symbols, std::convert::identity)
}

pub(super) fn build_identical(
    scope: &RequestScope,
    jobs: usize,
    symbols: &Symbols,
) -> Result<Index<InventoryMethod>, ImageError> {
    build_with(scope, jobs, symbols, |m| InventoryMethod {
        signature: m.identity(),
        at: m.at,
        issues: m.issues,
    })
}

fn build_with<T: Send>(
    scope: &RequestScope,
    jobs: usize,
    symbols: &Symbols,
    publish: fn(Method) -> T,
) -> Result<Index<T>, ImageError> {
    let roots = scope.roots()?;
    let mut results = Vec::new();
    runner::run::<_, _, _, ImageError>(
        roots.len(),
        jobs.max(1),
        || (ExportReader::new(scope.input()), Vec::new()),
        |(reader, scratch), pos, emit| {
            let mut index = Index::default();
            let root = &roots[pos];
            if let Err(reason) = scan_root(root, reader, scratch, &mut index, symbols, publish) {
                index
                    .issues
                    .push(format!("{}: {reason}", root.label()).into());
            }
            emit((pos, index))
        },
        &mut |row| {
            results.push(row);
            Ok(())
        },
    )?;
    Ok(merge_ordered(results)?)
}

fn merge_ordered<T>(
    mut results: Vec<(usize, Index<T>)>,
) -> Result<Index<T>, crate::analysis::input::InputError> {
    results.sort_unstable_by_key(|(pos, _)| *pos);
    if results.len() == 1 {
        return Ok(results.pop().expect("checked single root").1);
    }
    let mut total = Index::default();
    total
        .methods
        .try_reserve_exact(results.iter().map(|(_, r)| r.methods.len()).sum())
        .map_err(crate::analysis::input::InputError::Allocation)?;
    total
        .issues
        .try_reserve_exact(results.iter().map(|(_, r)| r.issues.len()).sum())
        .map_err(crate::analysis::input::InputError::Allocation)?;
    for (_, mut index) in results {
        total.methods.append(&mut index.methods);
        total.issues.append(&mut index.issues);
    }
    Ok(total)
}

fn scan_root<T>(
    root: &DexRoot<'_>,
    reader: &mut ExportReader<'_>,
    scratch: &mut Vec<u8>,
    out: &mut Index<T>,
    symbols: &Symbols,
    publish: fn(Method) -> T,
) -> Result<(), ImageError> {
    let bytes = match root.entry() {
        Some(entry) => reader.read_entry(entry, scratch)?,
        None => root.bytes()?,
    };
    let container = DexContainer::open(bytes)?;
    let mut budget = ExpansionBudget {
        remaining: bytes.len().saturating_mul(64).saturating_add(1 << 20),
    };
    let entry = root.entry().map(|e| e.index().0);
    for (logical, offset) in container.logical_headers()?.into_iter().enumerate() {
        let logical = u32::try_from(logical)
            .map_err(|_| crate::analysis::input::InputError::PlatformCapacity)?;
        match container.program(offset) {
            Ok(index) => {
                if !scan_image(
                    &index.with_dex(DexId(logical)),
                    entry,
                    logical,
                    out,
                    symbols,
                    &mut budget,
                    publish,
                ) {
                    break;
                }
            }
            Err(error) => out
                .issues
                .push(format!("e{}:l{logical}: {error}", entry.unwrap_or(0)).into()),
        }
    }
    Ok(())
}

fn scan_image<T>(
    index: &ProgramIndex<'_>,
    entry: Option<usize>,
    logical: u32,
    out: &mut Index<T>,
    symbols: &Symbols,
    budget: &mut ExpansionBudget,
    publish: fn(Method) -> T,
) -> bool {
    let mut cache = reference::Cache::new(budget);
    for class in 0..index.class_count() {
        let key = ClassKey {
            dex: DexId(logical),
            class_def: ClassDefId(class),
        };
        let facts = match p0::collect(index, key) {
            Ok(facts) => facts,
            Err(error) => {
                out.issues
                    .push(format!("e{}:l{logical}:c{class}: {error}", entry.unwrap_or(0)).into());
                continue;
            }
        };
        if let Some(error) = facts.methods_issue() {
            out.issues.push(
                format!(
                    "{}: {error}",
                    at::class(entry, logical, class, facts.descriptor().units())
                )
                .into(),
            );
        }
        let owner = at::escape_units(facts.descriptor().units());
        for (ordinal, item) in facts.methods().iter().enumerate() {
            let id = item.method();
            let Fact::Present(decl) = item.declaration() else {
                let loc = at::method_index(entry, logical, class, id.0);
                let reason = item
                    .declaration()
                    .unavailable()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "method declaration unavailable".into());
                out.issues.push(format!("{loc}: {reason}").into());
                continue;
            };
            let mut method = Method {
                at: at::method(
                    entry,
                    logical,
                    class,
                    id.0,
                    facts.descriptor().units(),
                    decl.name_units(),
                    decl.prototype_units(),
                ),
                owner: owner.clone(),
                name: at::escape_units(decl.name_units()),
                prototype: at::escape_units(decl.prototype_units()),
                access: item.access_flags().raw(),
                jvm: None,
                body: None,
                issues: Vec::new(),
            };
            match index.method_source_by_ordinal(&facts, ordinal) {
                Ok(source) if matches!(source.raw(), RawPrintable::Method(_)) => {
                    match index.disassemble_method_source(&source) {
                        Ok(outcome) => body::collect(outcome, &mut method, &mut cache, symbols),
                        Err(error) => method.issues.push(error.to_string()),
                    }
                }
                Ok(_) => {}
                Err(error) => method.issues.push(error.to_string()),
            }
            if !cache.budget.charge(method.retained_bytes()) {
                out.issues
                    .push(format!("{}: {ROOT_CAPACITY}; coverage stops here", method.at).into());
                return false;
            }
            out.methods.push(publish(method));
        }
    }
    true
}
