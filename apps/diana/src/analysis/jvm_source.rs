//! Resolve a JVM declaration and adapt the engine's owned source publication.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use diana_decomp::input::jvm::ClassFile;
use diana_decomp::jvm::{self, Source, SourceError, SourceIssue};
use serde::Serialize;

use super::at::escape_units;
use super::jvm::{JvmRoots, Location};
use super::jvm_code::method_at;
use super::query::{QueryError, QueryIssue};
use super::request::RequestScope;
use super::selector::{self, MethodSelector, SelectorError};

const CLASSES_PER_CLAIM: usize = 32;

#[derive(Debug, thiserror::Error)]
pub(crate) enum RetrievalError {
    #[error(transparent)]
    Selector(#[from] SelectorError),
    #[error(transparent)]
    Input(#[from] QueryError),
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error("selection is unresolved at {context} ({kind}): {reason}")]
    Unresolved {
        context: String,
        kind: String,
        reason: String,
    },
}

impl RetrievalError {
    pub(crate) fn code(&self) -> u8 {
        match self {
            Self::Selector(error) => error.code(),
            _ => 1,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct Retrieved {
    selection: Selection,
    pub(crate) result: Rendered,
    pub(crate) issues: Vec<Issue>,
}

#[derive(Serialize)]
struct Selection {
    input_kind: &'static str,
    class: String,
    method: Option<u16>,
    class_at: String,
    method_at: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Rendered {
    class: String,
    notation: &'static str,
    pub(crate) text: String,
    methods: Vec<Method>,
}

#[derive(Serialize)]
struct Method {
    at: String,
    ordinal: u16,
    name: String,
    prototype: String,
    access_flags: u16,
}

#[derive(Serialize)]
pub(crate) struct Issue {
    pub(crate) context: String,
    kind: String,
    stage: &'static str,
    pub(crate) reason: String,
    range_bytes: Option<[u32; 2]>,
}

impl Issue {
    pub(crate) fn into_reference_issue(self) -> super::xref::Issue {
        super::xref::Issue {
            context: self.context,
            kind: self.kind,
            reason: self.reason,
            range_bytes: self
                .range_bytes
                .map(|[start, end]| [start as usize, end as usize]),
        }
    }

    fn selection(issue: QueryIssue) -> Self {
        Self {
            context: issue.context,
            kind: issue.kind,
            stage: "selection",
            reason: issue.reason,
            range_bytes: None,
        }
    }

    fn recovery(class_at: &str, issue: SourceIssue) -> Self {
        let context = match (issue.method, issue.field) {
            (Some(method), _) => format!("{class_at}:m{method}"),
            (None, Some(field)) => format!("{class_at}:f{field}"),
            (None, None) => class_at.to_owned(),
        };
        Self {
            context,
            kind: format!("{:?}", issue.kind),
            stage: issue.stage,
            reason: issue.reason,
            range_bytes: issue
                .offset_byte
                .zip(issue.end_offset_byte)
                .map(|(start, end)| [u32::from(start), end]),
        }
    }
}

/// A physical selection opens only its coordinate chain. A named selection needs
/// the checked declarations of the input in order to determine uniqueness.
pub(crate) fn run_if_jvm(
    scope: &RequestScope,
    text: &str,
    method: bool,
    jobs: usize,
) -> Result<Option<Retrieved>, RetrievalError> {
    let prefix = text.split_once(':').map_or(text, |(prefix, _)| prefix);
    if let Ok(location) = prefix.parse::<Location>() {
        if !JvmRoots::supports(scope) {
            return Ok(None);
        }
        let ordinal = if method {
            Some(method_at(text)?.1)
        } else {
            if text != prefix {
                return Err(SelectorError::Syntax("JVM class locator names only a class").into());
            }
            None
        };
        let roots = JvmRoots::focused(scope, &location)?;
        let mut reader = roots.reader();
        let class = reader.read(
            roots
                .by_location(&location)
                .ok_or(SelectorError::InvalidIndex)?,
        )?;
        return Ok(Some(publish(
            &location,
            jvm::recover(&class, ordinal, jobs)?,
            ordinal,
            [],
        )?));
    }

    let lookup_jobs = super::image_jobs();
    let Some(roots) = JvmRoots::select_with_jobs(scope, lookup_jobs)? else {
        return Ok(None);
    };
    let method_selector = method.then(|| selector::method(text)).transpose()?;
    let class_selector = match &method_selector {
        Some(method) => &method.class,
        None => &selector::class(text)?,
    };
    if class_selector.class_def.is_some() {
        return Err(SelectorError::Syntax("JVM input requires a JVM locator").into());
    }
    let (location, class_issues) = find_class(&roots, &class_selector.descriptor, lookup_jobs)?;
    let mut reader = roots.reader();
    let class = reader.read(
        roots
            .by_location(&location)
            .ok_or(SelectorError::InvalidIndex)?,
    )?;
    let (ordinal, method_issues) = match method_selector {
        Some(selector) => {
            let (ordinal, issues) = find_method(&class, &location, &selector)?;
            (Some(ordinal), issues)
        }
        None => (None, Vec::new()),
    };
    let source = jvm::recover(&class, ordinal, jobs)?;
    Ok(Some(publish(
        &location,
        source,
        ordinal,
        class_issues.into_iter().chain(method_issues),
    )?))
}

enum Candidate {
    Match(Location),
    Unavailable(Location, QueryError),
}

pub(crate) fn find_class(
    roots: &JvmRoots<'_>,
    descriptor: &[u16],
    jobs: usize,
) -> Result<(Location, Vec<Issue>), RetrievalError> {
    let mut found = None;
    let mut ambiguous = false;
    let mut issues = Vec::new();
    issues
        .try_reserve_exact(roots.issues().len())
        .map_err(super::input::InputError::Allocation)
        .map_err(QueryError::from)?;
    issues.extend(roots.issues().iter().cloned().map(Issue::selection));
    super::runner::run(
        roots.len().div_ceil(CLASSES_PER_CLAIM),
        jobs,
        || roots.reader(),
        |reader, claim, emit| -> Result<(), QueryError> {
            let start = claim * CLASSES_PER_CLAIM;
            let end = start.saturating_add(CLASSES_PER_CLAIM).min(roots.len());
            for ordinal in start..end {
                let root = roots.get(ordinal).ok_or(QueryError::WorkerLost)?;
                let location = root.location();
                match reader
                    .read(root)
                    .and_then(|class| class.name().map_err(QueryError::from))
                {
                    Ok(name)
                        if std::iter::once(u16::from(b'L'))
                            .chain(name.units())
                            .chain([u16::from(b';')])
                            .eq(descriptor.iter().copied()) =>
                    {
                        emit(Candidate::Match(location.clone()))?;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        emit(Candidate::Unavailable(location.clone(), error))?;
                    }
                }
            }
            Ok(())
        },
        &mut |candidate| {
            match candidate {
                Candidate::Match(location) => {
                    ambiguous |= found.replace(location).is_some();
                }
                Candidate::Unavailable(location, error) => {
                    push_selection_issue(&mut issues, location.to_string(), error)?;
                }
            }
            Ok(())
        },
    )?;
    if ambiguous {
        return Err(SelectorError::Ambiguous.into());
    }
    selected(found, issues)
}

pub(crate) fn find_method(
    class: &ClassFile<'_>,
    location: &Location,
    selector: &MethodSelector,
) -> Result<(u16, Vec<Issue>), RetrievalError> {
    let mut found = None;
    let mut issues = Vec::new();
    for (position, declaration) in class.methods().map_err(QueryError::from)?.enumerate() {
        let (ordinal, matches) = match declaration {
            Ok(member) => (
                Some(member.ordinal()),
                member.name(class).and_then(|name| {
                    if name.units().eq(selector.name.iter().copied()) {
                        member.descriptor(class).map(|descriptor| {
                            descriptor.units().eq(selector.prototype.iter().copied())
                        })
                    } else {
                        Ok(false)
                    }
                }),
            ),
            Err(error) => (u16::try_from(position).ok(), Err(error)),
        };
        match matches {
            Ok(true) => {
                if found
                    .replace(ordinal.ok_or(SelectorError::InvalidIndex)?)
                    .is_some()
                {
                    return Err(SelectorError::Ambiguous.into());
                }
            }
            Ok(false) => {}
            Err(error) => {
                let context = ordinal.map_or_else(
                    || location.to_string(),
                    |ordinal| format!("{location}:m{ordinal}"),
                );
                push_selection_issue(&mut issues, context, error.into())?;
            }
        }
    }
    selected(found, issues)
}

fn push_selection_issue(
    issues: &mut Vec<Issue>,
    context: String,
    error: QueryError,
) -> Result<(), QueryError> {
    issues
        .try_reserve(1)
        .map_err(super::input::InputError::Allocation)?;
    issues.push(Issue::selection(QueryIssue::new(context, &error)));
    Ok(())
}

fn selected<T>(
    found: Option<T>,
    mut issues: Vec<Issue>,
) -> Result<(T, Vec<Issue>), RetrievalError> {
    if let Some(found) = found {
        return Ok((found, issues));
    }
    sort_issues(&mut issues);
    if let Some(issue) = issues.into_iter().next() {
        return Err(RetrievalError::Unresolved {
            context: issue.context,
            kind: issue.kind,
            reason: issue.reason,
        });
    }
    Err(SelectorError::NotFound.into())
}

fn sort_issues(issues: &mut [Issue]) {
    issues.sort_unstable_by(|left, right| {
        let order = left
            .context
            .split(['/', ':'])
            .map(context_part)
            .cmp(right.context.split(['/', ':']).map(context_part));
        order.then_with(|| {
            (left.stage, left.range_bytes, &left.kind, &left.reason).cmp(&(
                right.stage,
                right.range_bytes,
                &right.kind,
                &right.reason,
            ))
        })
    });
}

fn context_part(part: &str) -> (Option<(u8, usize)>, &str) {
    let physical = part.split_at_checked(1).and_then(|(prefix, ordinal)| {
        let kind = match prefix {
            "j" => 0,
            "f" => 1,
            "m" => 2,
            "a" => 3,
            _ => return None,
        };
        Some((kind, ordinal.parse().ok()?))
    });
    (physical, part)
}

fn append_issues(text: &mut String, issues: &[Issue]) -> Result<(), QueryError> {
    if issues.is_empty() {
        return Ok(());
    }
    let mut missing = Vec::new();
    missing
        .try_reserve_exact(issues.len())
        .map_err(super::input::InputError::Allocation)?;
    {
        // Match only the emitter's exact unavailable-expression marker in one
        // pass. A repeated reason can refer to distinct original methods.
        let mut emitted = HashSet::new();
        for reason in text.lines().flat_map(|line| {
            line.split("/* Diana: ").skip(1).filter_map(|marker| {
                marker
                    .split_once(" */ <unavailable>(")
                    .map(|(reason, _)| reason)
            })
        }) {
            emitted
                .try_reserve(1)
                .map_err(super::input::InputError::Allocation)?;
            emitted.insert(reason);
        }
        let mut counts = HashMap::new();
        counts
            .try_reserve(issues.len())
            .map_err(super::input::InputError::Allocation)?;
        for issue in issues {
            *counts.entry(issue.reason.as_str()).or_insert(0usize) += 1;
        }
        for issue in issues {
            let inline = issue.stage == "JVM decoding"
                && issue.range_bytes.is_some()
                && counts.get(issue.reason.as_str()) == Some(&1)
                && emitted.contains(
                    issue
                        .reason
                        .escape_debug()
                        .to_string()
                        .replace("*/", "* /")
                        .as_str(),
                );
            if !inline {
                missing.push(issue);
            }
        }
    }
    for issue in missing {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let context = issue.context.replace(['\r', '\n'], " ");
        let reason = issue.reason.replace(['\r', '\n'], " ");
        writeln!(
            text,
            "// Diana: {} at {context} ({}): {reason}",
            issue.stage, issue.kind
        )
        .expect("formatting into a String cannot fail");
    }
    Ok(())
}

pub(crate) fn export_issues(
    at: &str,
    text: &mut String,
    failures: Vec<SourceIssue>,
) -> Result<Vec<Issue>, QueryError> {
    let mut issues = Vec::new();
    issues
        .try_reserve_exact(failures.len())
        .map_err(super::input::InputError::Allocation)?;
    issues.extend(failures.into_iter().map(|issue| Issue::recovery(at, issue)));
    sort_issues(&mut issues);
    append_issues(text, &issues)?;
    Ok(issues)
}

fn publish(
    location: &Location,
    mut source: Source,
    ordinal: Option<u16>,
    selection_issues: impl IntoIterator<Item = Issue>,
) -> Result<Retrieved, QueryError> {
    let class_at = location.to_string();
    let descriptor = format!("L{};", escape_units(&source.name));
    let mut methods = Vec::new();
    methods
        .try_reserve_exact(source.methods.len())
        .map_err(super::input::InputError::Allocation)?;
    methods.extend(source.methods.into_iter().map(|method| Method {
        at: format!("{class_at}:m{}", method.ordinal),
        ordinal: method.ordinal,
        name: escape_units(&method.name),
        prototype: escape_units(&method.descriptor),
        access_flags: method.access_flags,
    }));
    let selection_issues = selection_issues.into_iter();
    let mut issues = Vec::new();
    issues
        .try_reserve_exact(
            source
                .issues
                .len()
                .saturating_add(selection_issues.size_hint().0),
        )
        .map_err(super::input::InputError::Allocation)?;
    issues.extend(selection_issues);
    issues.extend(
        source
            .issues
            .into_iter()
            .map(|issue| Issue::recovery(&class_at, issue)),
    );
    sort_issues(&mut issues);
    append_issues(&mut source.text, &issues)?;
    Ok(Retrieved {
        selection: Selection {
            input_kind: "jvm",
            class: descriptor.clone(),
            method: ordinal,
            method_at: ordinal.map(|ordinal| format!("{class_at}:m{ordinal}")),
            class_at,
        },
        result: Rendered {
            class: descriptor,
            notation: "diana-decomp",
            text: source.text,
            methods,
        },
        issues,
    })
}
