//! JVM original bytecode and reference presentation.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::io::Write;
use std::path::Path;

use diana_decomp::FailureKind;
use diana_decomp::input::jvm::bytecode::{ConstantOperand, Instruction, Operands, Reference};
use diana_decomp::input::jvm::{ClassFile, Constant, Member, ReferenceKind};

use super::output::{Failure, Format, publish_with};
use super::table::{RowSummary, encode_cell, publish_rows};
use crate::analysis::filter::best_first;
use crate::analysis::input::InputError;
use crate::analysis::jvm::JvmRoots;
use crate::analysis::jvm_code::{self, Event, Hit, Plan};
use crate::analysis::query::QueryError;
use crate::analysis::xref::Issue;

pub(super) fn xref(
    roots: &JvmRoots<'_>,
    plan: &Plan<'_>,
    limit: usize,
    format: Format,
    input: &Path,
    output: Option<&Path>,
) -> Result<(), Failure> {
    let ranked = plan.target.is_ranked();
    if !ranked && limit == 0 {
        return stream_xref(roots, plan, format, input, output);
    }
    let mut issues = Vec::new();
    let mut rows = Vec::new();
    let mut top = BinaryHeap::new();
    let mut total = 0usize;
    jvm_code::scan(roots, plan, &mut |event| {
        match event {
            Event::Hit(hit) => {
                total = total.checked_add(1).ok_or(InputError::PlatformCapacity)?;
                if ranked && limit != 0 {
                    let candidate = Ranked(hit);
                    if top.len() < limit {
                        top.try_reserve(1).map_err(InputError::Allocation)?;
                        top.push(candidate);
                    } else if top.peek().is_some_and(|last| candidate < *last) {
                        *top.peek_mut().expect("nonempty ranked query") = candidate;
                    }
                } else if limit == 0 || rows.len() < limit {
                    rows.try_reserve(1).map_err(InputError::Allocation)?;
                    rows.push(hit);
                }
            }
            Event::Issue(issue) => {
                issues.try_reserve(1).map_err(InputError::Allocation)?;
                issues.push(issue);
            }
        }
        Ok(())
    })
    .map_err(query_failure)?;
    if ranked && limit != 0 {
        rows = top.into_sorted_vec().into_iter().map(|row| row.0).collect();
    } else if ranked {
        rows.sort_unstable_by(compare);
    }
    let shown = rows.len();
    let rows = rows.into_iter().map(hit_cells).collect::<Vec<_>>();
    let warning = (shown < total).then(|| {
        format!(
            "//Diana Warn: Too much results, only show top {shown} , use --no-limit or --limit N"
        )
    });
    publish_rows(
        &rows,
        &REFERENCE_COLUMNS,
        "xref",
        format,
        input,
        output,
        RowSummary {
            total,
            warning: warning.as_deref(),
            issues: &issues,
        },
    )
}

const REFERENCE_COLUMNS: [&str; 4] = ["context", "match", "use", "offset_byte"];

fn hit_cells(hit: Hit) -> Vec<String> {
    vec![
        hit.context(),
        hit.matched,
        format!("{} depth {}", hit.use_, hit.depth),
        hit.offset_byte
            .map_or_else(String::new, |pc| pc.to_string()),
    ]
}

/// An unrestricted unranked query can publish each hit in arrival order. Its
/// summary follows the items, once the same scan has measured coverage.
fn stream_xref(
    roots: &JvmRoots<'_>,
    plan: &Plan<'_>,
    format: Format,
    input: &Path,
    output: Option<&Path>,
) -> Result<(), Failure> {
    publish_with(output, input, |writer| {
        match format {
            Format::Text => writer
                .write_all(b"context\tmatch\tuse\toffset_byte\n")
                .map_err(Failure::output)?,
            Format::Json => writer
                .write_all(b"{\"schema_version\":3,\"command\":\"xref\",\"items\":[")
                .map_err(Failure::output)?,
        }
        let mut issues = Vec::new();
        let mut total = 0usize;
        let mut output_error = None;
        let scan = jvm_code::scan(roots, plan, &mut |event| {
            match event {
                Event::Hit(hit) => {
                    let cells = hit_cells(hit);
                    if let Err(error) = write_hit(writer, &cells, format, total == 0) {
                        output_error = Some(error);
                        return Err(QueryError::WorkerLost);
                    }
                    total = total.checked_add(1).ok_or(InputError::PlatformCapacity)?;
                }
                Event::Issue(issue) => {
                    issues.try_reserve(1).map_err(InputError::Allocation)?;
                    issues.push(issue);
                }
            }
            Ok(())
        });
        if let Some(error) = output_error {
            return Err(error);
        }
        scan.map_err(query_failure)?;
        for issue in &issues {
            eprintln!(
                "diana: warning: {}: {}: {}",
                issue.context, issue.kind, issue.reason
            );
        }
        if format == Format::Text {
            if !issues.is_empty() {
                writer
                    .write_all(b"//Diana Warn: coverage incomplete; counts describe recovered matches only\n")
                    .map_err(Failure::output)?;
            }
            return Ok(());
        }
        let complete = issues.is_empty();
        writer.write_all(b"],\"total\":").map_err(Failure::output)?;
        if complete {
            write!(writer, "{total}").map_err(Failure::output)?;
        } else {
            writer.write_all(b"null").map_err(Failure::output)?;
        }
        write!(
            writer,
            ",\"recovered_total\":{total},\"complete\":{complete},\"issues\":"
        )
        .map_err(Failure::output)?;
        serde_json::to_writer(&mut *writer, &issues)
            .map_err(|error| Failure::output(std::io::Error::other(error)))?;
        write!(
            writer,
            ",\"shown\":{total},\"truncated\":false,\"warnings\":[]}}"
        )
        .map_err(Failure::output)
    })
}

fn write_hit(
    writer: &mut dyn Write,
    cells: &[String],
    format: Format,
    first: bool,
) -> Result<(), Failure> {
    if format == Format::Json {
        writer
            .write_all(if first { b"{" } else { b",{" })
            .map_err(Failure::output)?;
    }
    for (index, (name, value)) in REFERENCE_COLUMNS.iter().zip(cells).enumerate() {
        match format {
            Format::Json => {
                if index != 0 {
                    writer.write_all(b",").map_err(Failure::output)?;
                }
                write!(writer, "\"{name}\":").map_err(Failure::output)?;
            }
            Format::Text if index != 0 => writer.write_all(b"\t").map_err(Failure::output)?,
            Format::Text => {}
        }
        writer
            .write_all(&encode_cell(value, format)?)
            .map_err(Failure::output)?;
    }
    writer
        .write_all(if format == Format::Json { b"}" } else { b"\n" })
        .map_err(Failure::output)
}

fn compare(left: &Hit, right: &Hit) -> Ordering {
    best_first(left.relevance, right.relevance)
        .then_with(|| left.depth.cmp(&right.depth))
        .then_with(|| left.location.cmp(&right.location))
        .then_with(|| left.member.cmp(&right.member))
        .then_with(|| left.offset_byte.cmp(&right.offset_byte))
        .then_with(|| left.matched.cmp(&right.matched))
        .then_with(|| left.use_.cmp(&right.use_))
}

struct Ranked(Hit);
impl Eq for Ranked {}
impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(&self.0, &other.0)
    }
}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn bytecode(
    roots: &JvmRoots<'_>,
    selector: &str,
    format: Format,
    input: &Path,
    output: Option<&Path>,
) -> Result<(), Failure> {
    let (location, ordinal) = jvm_code::method_at(selector).map_err(|error| Failure {
        code: error.code(),
        message: error.to_string(),
    })?;
    let root = roots.by_location(&location).ok_or(Failure {
        code: 3,
        message: format!("no JVM definition at {location}"),
    })?;
    let mut reader = roots.reader();
    let class = reader.read(root).map_err(query_failure)?;
    let member = find_method(&class, ordinal)?;
    let context = format!("{location}:m{ordinal}");
    let mut report = Bytecode::new(&context);
    report.read(&class, member).map_err(query_failure)?;
    let total = report.rows.len();
    publish_rows(
        &report.rows,
        &["context", "offset_byte", "mnemonic", "operands"],
        "bytecode",
        format,
        input,
        output,
        RowSummary {
            total,
            warning: None,
            issues: &report.issues,
        },
    )
}

fn find_method<'a>(class: &ClassFile<'a>, ordinal: u16) -> Result<Member<'a>, Failure> {
    for entry in class
        .methods()
        .map_err(jvm_code::class_error)
        .map_err(query_failure)?
    {
        let member = entry
            .map_err(jvm_code::class_error)
            .map_err(query_failure)?;
        if member.ordinal() == ordinal {
            return Ok(member);
        }
    }
    Err(Failure {
        code: 3,
        message: format!("no JVM method at class member ordinal {ordinal}"),
    })
}

struct Bytecode<'a> {
    context: &'a str,
    rows: Vec<Vec<String>>,
    issues: Vec<Issue>,
}

impl<'a> Bytecode<'a> {
    fn new(context: &'a str) -> Self {
        Self {
            context,
            rows: Vec::new(),
            issues: Vec::new(),
        }
    }

    fn read<'b>(&mut self, class: &ClassFile<'b>, member: Member<'b>) -> Result<(), QueryError> {
        let owner = self.identity(class.name())?;
        let name = self.identity(member.name(class))?;
        let descriptor = self.identity(member.descriptor(class))?;
        self.row(
            None,
            "method",
            format!(
                "{owner}->{name}{descriptor} flags=0x{:04x}",
                member.access_flags()
            ),
        )?;
        let code = match jvm_code::method_code(class, member) {
            Ok(Some(code)) => code,
            Ok(None) => return Ok(()),
            Err(error) => {
                self.issues.try_reserve(1).map_err(InputError::Allocation)?;
                self.issues
                    .push(Issue::new(self.context.to_owned(), &error, None));
                return Ok(());
            }
        };
        self.row(
            None,
            "Code",
            format!(
                "max_stack={} max_locals={} class_file_byte={}",
                code.max_stack,
                code.max_locals,
                code.class_file_offset()
            ),
        )?;
        for item in code.instructions() {
            match item {
                Ok(insn) => self.instruction(class, insn)?,
                Err(error) => {
                    let range = [usize::from(error.range.start), usize::from(error.range.end)];
                    self.issue(error.kind(), error, Some(range))?;
                }
            }
        }
        if let Some(missing) = code.missing_bytes() {
            self.issue(
                FailureKind::Malformed,
                "truncated JVM Code bytes",
                Some([usize::from(missing.start), usize::from(missing.end)]),
            )?;
            return Ok(());
        }
        match code.exception_handlers() {
            Ok(handlers) => {
                if let Some(error) = handlers.issue() {
                    self.issue(FailureKind::Malformed, error, None)?;
                }
                for handler in handlers {
                    if let Some(problem) = handler.range_problem(code) {
                        self.issue(FailureKind::Malformed, format_args!("{problem:?}"), None)?;
                    }
                    let mut details =
                        format!("[{}, {}) -> {}", handler.start, handler.end, handler.target);
                    if let Some(index) = handler.catch_type {
                        details.push_str(&format!(" #{index}"));
                        match class.class_name(index) {
                            Ok(name) => {
                                details.push(' ');
                                details.push_str(&jvm_code::type_text(name)?);
                            }
                            Err(error) => self.issue(error.kind(), error, None)?,
                        }
                    } else {
                        details.push_str(" all");
                    }
                    self.row(None, "catch", details)?;
                }
            }
            Err(error) => self.issue(FailureKind::Malformed, error, None)?,
        }
        Ok(())
    }

    fn identity(
        &mut self,
        value: Result<
            diana_decomp::input::jvm::JvmString<'_>,
            diana_decomp::input::jvm::ClassError,
        >,
    ) -> Result<String, QueryError> {
        match value {
            Ok(value) => jvm_code::display(value),
            Err(error) => {
                self.issue(error.kind(), error, None)?;
                Ok("<?>".to_owned())
            }
        }
    }

    fn instruction<'b>(
        &mut self,
        class: &ClassFile<'b>,
        insn: Instruction<'b>,
    ) -> Result<(), QueryError> {
        let start = usize::from(insn.offset());
        let range = Some([start, start + insn.bytes().len()]);
        if let Some(problem) = insn.operand_problem() {
            self.issue(FailureKind::Malformed, format_args!("{problem:?}"), range)?;
        }
        let operand = match insn.operands() {
            Operands::None => String::new(),
            Operands::Immediate(value) => value.to_string(),
            Operands::Local(index) => index.to_string(),
            Operands::Increment { local, value } => format!("{local}, {value}"),
            Operands::ArrayType(kind) => format!(
                "{kind} ({})",
                match kind {
                    4 => "boolean",
                    5 => "char",
                    6 => "float",
                    7 => "double",
                    8 => "byte",
                    9 => "short",
                    10 => "int",
                    11 => "long",
                    _ => "?",
                }
            ),
            Operands::Branch(relative) => branch(insn, relative),
            Operands::Switch(switch) => {
                use std::fmt::Write;
                let mut text = format!("default: {}", branch(insn, switch.default));
                for (key, relative) in switch.cases() {
                    write!(text, ", {key}: {}", branch(insn, relative))
                        .expect("writing to a String");
                }
                text
            }
            Operands::Constant { index, extra } => {
                let mut text = format!("#{index}");
                if let Some(extra) = extra {
                    match extra {
                        ConstantOperand::InterfaceSlots(slots) => {
                            text.push_str(&format!(", slots={slots}"))
                        }
                        ConstantOperand::Dimensions(dimensions) => {
                            text.push_str(&format!(", dimensions={dimensions}"))
                        }
                    }
                }
                let resolved = match insn.reference(class) {
                    Ok(Some(reference)) => describe_reference(reference)?,
                    Ok(None) => match class.constant(index) {
                        Ok(Constant::Integer(value)) => value.to_string(),
                        Ok(Constant::Long(value)) => format!("{value}L"),
                        Ok(Constant::Float(bits)) => {
                            format!("{}F (0x{bits:08x})", f32::from_bits(bits))
                        }
                        Ok(Constant::Double(bits)) => {
                            format!("{}D (0x{bits:016x})", f64::from_bits(bits))
                        }
                        _ => String::new(),
                    },
                    Err(error) => {
                        self.issue(error.kind(), error, range)?;
                        String::new()
                    }
                };
                if !resolved.is_empty() {
                    text.push_str(" // ");
                    text.push_str(&resolved);
                }
                text
            }
        };
        if let Some(modified) = insn.modified_mnemonic() {
            self.row(Some(insn.offset()), &format!("wide {modified}"), operand)
        } else {
            self.row(Some(insn.offset()), insn.mnemonic(), operand)
        }
    }

    fn row(
        &mut self,
        offset: Option<u16>,
        label: &str,
        operands: String,
    ) -> Result<(), QueryError> {
        self.rows.try_reserve(1).map_err(InputError::Allocation)?;
        self.rows.push(vec![
            self.context.to_owned(),
            offset.map_or_else(String::new, |byte| byte.to_string()),
            label.to_owned(),
            operands,
        ]);
        Ok(())
    }

    fn issue(
        &mut self,
        kind: FailureKind,
        problem: impl std::fmt::Display,
        range_bytes: Option<[usize; 2]>,
    ) -> Result<(), QueryError> {
        self.issues.try_reserve(1).map_err(InputError::Allocation)?;
        self.issues.push(Issue {
            context: self.context.to_owned(),
            kind: format!("{kind:?}"),
            reason: problem.to_string(),
            range_bytes,
        });
        Ok(())
    }
}

fn branch(insn: Instruction<'_>, relative: i32) -> String {
    match insn.target(relative) {
        Ok(target) => format!("{target} ({relative:+})"),
        Err(_) => format!("? ({relative:+})"),
    }
}

fn describe_reference(reference: Reference<'_>) -> Result<String, QueryError> {
    Ok(match reference {
        Reference::Class(name) => jvm_code::type_text(name)?,
        Reference::String(text) => format!("\"{}\"", jvm_code::display(text)?),
        Reference::Member(member) | Reference::MethodHandle { member, .. } => {
            let owner = jvm_code::type_text(member.owner)?;
            let name = jvm_code::display(member.name)?;
            let descriptor = jvm_code::display(member.descriptor)?;
            let separator = if member.kind == ReferenceKind::Field {
                ":"
            } else {
                ""
            };
            match reference {
                Reference::MethodHandle {
                    kind,
                    reference_index,
                    ..
                } => {
                    format!(
                        "handle {kind} #{reference_index} {owner}->{name}{separator}{descriptor}"
                    )
                }
                _ => format!("{owner}->{name}{separator}{descriptor}"),
            }
        }
        Reference::Dynamic {
            bootstrap_index,
            name,
            descriptor,
            ..
        } => {
            let name = jvm_code::display(name)?;
            let descriptor = jvm_code::display(descriptor)?;
            format!("bootstrap {bootstrap_index} {name}{descriptor}")
        }
        Reference::MethodType(descriptor) => jvm_code::display(descriptor)?,
    })
}

fn query_failure(error: QueryError) -> Failure {
    Failure::query(error, 1)
}
