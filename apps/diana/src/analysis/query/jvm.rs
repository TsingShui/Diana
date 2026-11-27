//! JVM declarations delivered through the common bounded query channel.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use diana_decomp::input::jvm::{ClassError, ClassFile, JvmString};

use crate::analysis::filter::FilterMatch;
use crate::analysis::jvm::{JvmRoots, Location};

use super::*;

const CLASSES_PER_BATCH: usize = 32;
// A worker retains bounded formatted output across its class batches.
const JVM_CHUNK_BYTES: usize = 128 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JvmRow<'a> {
    pub relevance: Option<Relevance>,
    pub location: &'a Location,
    pub shard: usize,
    pub ordinal: usize,
    pub item: JvmItem<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JvmItem<'a> {
    Class(JvmString<'a>),
    Method {
        index: u16,
        owner: JvmString<'a>,
        name: JvmString<'a>,
        descriptor: JvmString<'a>,
    },
    Field {
        index: u16,
        owner: JvmString<'a>,
        name: JvmString<'a>,
        descriptor: JvmString<'a>,
    },
    String {
        index: u16,
        value: JvmString<'a>,
    },
}

impl JvmItem<'_> {
    pub fn index(self) -> u16 {
        match self {
            Self::Class(_) => 0,
            Self::Method { index, .. } | Self::Field { index, .. } | Self::String { index, .. } => {
                index
            }
        }
    }
}

impl Query<'_> {
    pub(super) fn jvm_batches(roots: &JvmRoots<'_>) -> usize {
        roots.len().div_ceil(CLASSES_PER_BATCH)
    }

    pub(super) fn run_jvm<R, S>(
        &self,
        roots: &JvmRoots<'_>,
        render: &R,
        sink: &mut S,
    ) -> Result<QueryOutcome, QueryError>
    where
        R: for<'r> Fn(Row<'r>, &mut Vec<u8>, bool) -> Result<(), QueryError> + Sync,
        S: FnMut(Vec<u8>) -> Result<(), QueryError>,
    {
        let ranked = self.filter.as_ref().is_some_and(TextFilter::is_fuzzy);
        let emitted = AtomicUsize::new(0);
        let truncated = AtomicBool::new(false);
        let mut collected = ranking::Collected::default();
        collected
            .issues
            .try_reserve(roots.issues().len())
            .map_err(InputError::Allocation)?;
        collected.issues.extend_from_slice(roots.issues());
        super::super::runner::run_indexed_with_finish(
            Self::jvm_batches(roots),
            self.jobs,
            |worker| {
                (
                    worker,
                    roots.reader(),
                    self.filter.as_ref().map(TextFilter::for_worker),
                    String::new(),
                    ranking::Buffer::with_chunk_bytes(ranked, JVM_CHUNK_BYTES),
                )
            },
            |(worker, reader, filter, descriptor, buffer), batch, emit| {
                let start = batch * CLASSES_PER_BATCH;
                let end = start.saturating_add(CLASSES_PER_BATCH).min(roots.len());
                for ordinal in start..end {
                    let root = roots.get(ordinal).ok_or(QueryError::WorkerLost)?;
                    let location = root.location();
                    let class = match reader.read(root) {
                        Ok(class) => class,
                        Err(error) => {
                            emit(ranking::Batch::Issue(QueryIssue::new(
                                location.to_string(),
                                &error,
                            )))?;
                            continue;
                        }
                    };
                    let mut deliver = |product| match product {
                        ClassProduct::Item(item, selected) => buffer.push(
                            Row::Jvm(JvmRow {
                                relevance: selected.relevance,
                                location,
                                shard: *worker,
                                ordinal,
                                item,
                            }),
                            render,
                            self.limit,
                            &emitted,
                            &truncated,
                            emit,
                        ),
                        ClassProduct::Issue(error, member) => {
                            let context = member.map_or_else(
                                || location.to_string(),
                                |(kind, index)| format!("{location}:{kind}{index}"),
                            );
                            emit(ranking::Batch::Issue(QueryIssue::new(
                                context,
                                &QueryError::from(error),
                            )))
                        }
                    };
                    let result =
                        self.visit_jvm(&class, filter.as_deref(), descriptor, &mut deliver);
                    match result {
                        Err(QueryError::JvmClass(error)) => {
                            deliver(ClassProduct::Issue(error, None))?
                        }
                        other => other?,
                    }
                }
                Ok(())
            },
            |(_, _, _, _, buffer), emit| buffer.flush(emit),
            &mut |batch| collected.accept(batch, sink),
        )?;
        let mut issues = std::mem::take(&mut collected.issues);
        issues.sort_unstable_by(|left, right| {
            (&left.context, &left.reason).cmp(&(&right.context, &right.reason))
        });
        collected.finish(sink)?;
        Ok(QueryOutcome {
            truncated: truncated.load(Ordering::Relaxed),
            issues,
        })
    }

    fn visit_jvm<'r>(
        &self,
        class: &ClassFile<'r>,
        filter: Option<&TextFilter>,
        descriptor_buffer: &mut String,
        deliver: &mut impl FnMut(ClassProduct<'r>) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        match self.kind {
            QueryKind::Classes => {
                let name = class.name()?;
                if !allows_class(&self.class_filter, class)? {
                    return Ok(());
                }
                let selection = if let Some(filter) = filter {
                    if let Some(name) = name.utf8() {
                        descriptor_buffer.clear();
                        descriptor_buffer
                            .try_reserve(name.len() + 2)
                            .map_err(InputError::Allocation)?;
                        descriptor_buffer.push('L');
                        descriptor_buffer.push_str(name);
                        descriptor_buffer.push(';');
                        filter.evaluate_text(descriptor_buffer)?
                    } else {
                        filter.evaluate_units(&descriptor(name)?)?
                    }
                } else {
                    Some(FilterMatch::default())
                };
                if let Some(selected) = selection {
                    deliver(ClassProduct::Item(JvmItem::Class(name), selected))?;
                }
            }
            QueryKind::Fields | QueryKind::Methods => {
                let owner = class.name()?;
                if let Some(expected) = &self.class
                    && !matches_descriptor(owner, expected)?
                {
                    return Ok(());
                }
                let methods = self.kind == QueryKind::Methods;
                let members = if methods {
                    class.methods()?
                } else {
                    class.fields()?
                };
                for member in members {
                    let member = match member {
                        Ok(member) => member,
                        Err(error) => {
                            deliver(ClassProduct::Issue(error, None))?;
                            continue;
                        }
                    };
                    let index = member.ordinal();
                    let name = match member.name(class) {
                        Ok(parts) => parts,
                        Err(error) => {
                            deliver(ClassProduct::Issue(
                                error,
                                Some((if methods { 'm' } else { 'f' }, index)),
                            ))?;
                            continue;
                        }
                    };
                    if let Some(selected) = select_string(filter, name)? {
                        let descriptor = match member.descriptor(class) {
                            Ok(descriptor) => descriptor,
                            Err(error) => {
                                deliver(ClassProduct::Issue(
                                    error,
                                    Some((if methods { 'm' } else { 'f' }, index)),
                                ))?;
                                continue;
                            }
                        };
                        let item = if methods {
                            JvmItem::Method {
                                index,
                                owner,
                                name,
                                descriptor,
                            }
                        } else {
                            JvmItem::Field {
                                index,
                                owner,
                                name,
                                descriptor,
                            }
                        };
                        deliver(ClassProduct::Item(item, selected))?;
                    }
                }
            }
            QueryKind::Strings => {
                for entry in class.strings() {
                    match entry {
                        Ok((index, value)) => {
                            if let Some(selected) = select_string(filter, value)? {
                                deliver(ClassProduct::Item(
                                    JvmItem::String { index, value },
                                    selected,
                                ))?;
                            }
                        }
                        Err(error) => deliver(ClassProduct::Issue(error, None))?,
                    }
                }
            }
            QueryKind::Entries => unreachable!("archive listing is routed before JVM selection"),
        }
        Ok(())
    }
}

enum ClassProduct<'a> {
    Item(JvmItem<'a>, FilterMatch),
    Issue(ClassError, Option<(char, u16)>),
}

fn select_string(
    filter: Option<&TextFilter>,
    value: JvmString<'_>,
) -> Result<Option<FilterMatch>, QueryError> {
    let Some(filter) = filter else {
        return Ok(Some(FilterMatch::default()));
    };
    filter.evaluate_jvm(value)
}

pub(in crate::analysis) fn descriptor(name: JvmString<'_>) -> Result<Vec<u16>, QueryError> {
    let mut units = Vec::new();
    units
        .try_reserve_exact(name.bytes().len() + 2)
        .map_err(InputError::Allocation)?;
    units.push(u16::from(b'L'));
    if let Some(text) = name.utf8() {
        units.extend(text.encode_utf16());
    } else {
        units.extend(name.to_utf16()?);
    }
    units.push(u16::from(b';'));
    Ok(units)
}

fn matches_descriptor(name: JvmString<'_>, expected: &[u16]) -> Result<bool, ClassError> {
    let Some(inner) = expected
        .strip_prefix(&[u16::from(b'L')])
        .and_then(|tail| tail.strip_suffix(&[u16::from(b';')]))
    else {
        return Ok(false);
    };
    if let Some(text) = name.utf8() {
        Ok(text.encode_utf16().eq(inner.iter().copied()))
    } else {
        Ok(name.to_utf16()? == inner)
    }
}

fn allows_class(relation: &ClassFilter, class: &ClassFile<'_>) -> Result<bool, ClassError> {
    let interface = class.access_flags() & 0x200 != 0;
    if relation
        .kind
        .is_some_and(|kind| interface != (kind == ClassKind::Interface))
    {
        return Ok(false);
    }
    if let Some(target) = &relation.extends {
        let Some(parent) = class.super_name()? else {
            return Ok(false);
        };
        if !matches_descriptor(parent, target)? {
            return Ok(false);
        }
    }
    for (configured, required_interface) in [
        (&relation.implements, false),
        (&relation.subinterface, true),
    ] {
        let Some(target) = configured else { continue };
        if interface != required_interface {
            return Ok(false);
        }
        let mut matched = false;
        for parent in class.interfaces() {
            matched |= matches_descriptor(parent?, target)?;
        }
        if !matched {
            return Ok(false);
        }
    }
    Ok(true)
}
