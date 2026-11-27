//! Compare the methods of two independently leased inputs.

use serde::Serialize;

use super::{jvm::JvmRoots, query::QueryError, request::RequestScope};
use index::{Index, InventoryMethod, Method};

mod alignment;
mod index;
mod inline;
mod matching;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DiffError {
    #[error("{side} input: {source}")]
    Input {
        side: &'static str,
        source: super::images::ImageError,
    },
    #[error("{side} input: {source}")]
    Jvm {
        side: &'static str,
        source: QueryError,
    },
    #[error("comparison requires two DEX inputs or two JVM inputs")]
    MixedFormats,
}

#[derive(Serialize)]
pub(crate) struct Report {
    pub schema: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_unit: Option<&'static str>,
    pub summary: Summary,
    pub matches: Vec<MatchedMethod>,
    pub unmatched: Vec<UnmatchedGroup>,
    pub fragments: Vec<Fragment>,
    pub issues: Vec<Issue>,
}

#[derive(Serialize)]
pub(crate) struct Summary {
    pub old_methods: usize,
    pub new_methods: usize,
    pub candidate_pairs: usize,
}

#[derive(Serialize)]
pub(crate) struct MatchedMethod {
    pub old: MethodLocation,
    pub new: MethodLocation,
    pub evidence: Vec<&'static str>,
    pub status: &'static str,
    pub changes: Vec<Change>,
}

#[derive(Serialize)]
pub(crate) struct MethodLocation {
    pub at: String,
    pub signature: String,
}

impl From<&Method> for MethodLocation {
    fn from(method: &Method) -> Self {
        Self {
            at: method.at.clone(),
            signature: method.identity(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct Change {
    pub kind: &'static str,
    pub old: Option<Site>,
    pub new: Option<Site>,
}

#[derive(Serialize)]
pub(crate) struct Site {
    pub offset: Option<u32>,
    pub text: String,
}

#[derive(Serialize)]
pub(crate) struct UnmatchedGroup {
    pub status: &'static str,
    pub old: Vec<MethodLocation>,
    pub new: Vec<MethodLocation>,
}

#[derive(Serialize)]
pub(crate) struct Fragment {
    pub old: MethodLocation,
    pub new: MethodLocation,
    pub old_offset: u32,
    pub new_offset: u32,
    pub instruction_count: usize,
}

#[derive(Serialize)]
pub(crate) struct Issue {
    pub side: &'static str,
    pub at: Option<String>,
    pub reason: String,
}

pub(crate) fn compare(
    old: &RequestScope,
    new: &RequestScope,
    jobs: usize,
) -> Result<Report, DiffError> {
    let symbols = index::Symbols::default();
    let jvm = JvmRoots::supports(old);
    if jvm != JvmRoots::supports(new) {
        return Err(DiffError::MixedFormats);
    }
    if jvm {
        let open = |scope, side| {
            JvmRoots::select_with_jobs(scope, jobs)
                .map_err(|source| DiffError::Jvm { side, source })?
                .ok_or(DiffError::MixedFormats)
        };
        // A project descriptor's bytes do not identify the archives it
        // resolves beside the descriptor.
        if old.jvm_artifacts().is_none()
            && new.jvm_artifacts().is_none()
            && old.input().bytes() == new.input().bytes()
        {
            let side = "old and new";
            let roots = open(old, side)?;
            let index = index::jvm::build(&roots, jobs, &symbols, |method| InventoryMethod {
                signature: method.identity(),
                at: method.at,
                issues: method.issues,
            })
            .map_err(|source| DiffError::Jvm { side, source })?;
            return Ok(compare_identical(index, Some("byte")));
        }
        let build = |scope, side| {
            let roots = open(scope, side)?;
            index::jvm::build(&roots, jobs, &symbols, std::convert::identity)
                .map_err(|source| DiffError::Jvm { side, source })
        };
        let before = build(old, "old")?;
        let after = build(new, "new")?;
        symbols.finish();
        return Ok(compare_indexes(before, after, Some("byte")));
    }
    if old.input().bytes() == new.input().bytes() {
        let index =
            index::build_identical(old, jobs, &symbols).map_err(|source| DiffError::Input {
                side: "old and new",
                source,
            })?;
        return Ok(compare_identical(index, None));
    }
    let old = index::build(old, jobs, &symbols).map_err(|source| DiffError::Input {
        side: "old",
        source,
    })?;
    let new = index::build(new, jobs, &symbols).map_err(|source| DiffError::Input {
        side: "new",
        source,
    })?;
    symbols.finish();
    Ok(compare_indexes(old, new, None))
}

fn compare_identical(index: Index<InventoryMethod>, offset_unit: Option<&'static str>) -> Report {
    let mut issues = Vec::new();
    let count = index.methods.len();
    for side in ["old", "new"] {
        add_inventory_issues(&mut issues, side, &index.issues);
    }
    let matches = index
        .methods
        .into_iter()
        .map(|method| {
            for side in ["old", "new"] {
                add_issues(&mut issues, side, Some(&method.at), &method.issues);
            }
            MatchedMethod {
                old: MethodLocation {
                    at: method.at.clone(),
                    signature: method.signature.clone(),
                },
                new: MethodLocation {
                    at: method.at,
                    signature: method.signature,
                },
                evidence: vec!["identical_input_bytes"],
                status: if method.issues.is_empty() {
                    "unchanged"
                } else {
                    "unknown"
                },
                changes: Vec::new(),
            }
        })
        .collect();
    Report {
        schema: 1,
        offset_unit,
        summary: Summary {
            old_methods: count,
            new_methods: count,
            candidate_pairs: 0,
        },
        matches,
        unmatched: Vec::new(),
        fragments: Vec::new(),
        issues,
    }
}

fn compare_indexes(old: Index, new: Index, offset_unit: Option<&'static str>) -> Report {
    let paired = matching::compare(&old.methods, &new.methods, offset_unit.is_some());
    let mut issues = Vec::new();
    let mut matches = Vec::with_capacity(paired.pairs.len());
    let symbols_renamed = paired.callees.iter().any(|(before, after)| before != after);
    for pair in &paired.pairs {
        let before = &old.methods[pair.old];
        let after = &new.methods[pair.new];
        let mut method = alignment::method(before, after, &paired.callees, symbols_renamed);
        method.evidence = pair.evidence.names(offset_unit.is_some());
        matches.push(method);
    }
    let unmatched = paired.unmatched(&old, &new);
    let fragments = alignment::fragments(&old.methods, &new.methods, &paired);
    for (side, index) in [("old", &old), ("new", &new)] {
        add_inventory_issues(&mut issues, side, &index.issues);
        for method in &index.methods {
            add_issues(&mut issues, side, Some(&method.at), &method.issues);
        }
    }
    Report {
        schema: 1,
        offset_unit,
        summary: Summary {
            old_methods: old.methods.len(),
            new_methods: new.methods.len(),
            candidate_pairs: paired.candidate_pairs,
        },
        matches,
        unmatched,
        fragments,
        issues,
    }
}

fn add_issues(out: &mut Vec<Issue>, side: &'static str, at: Option<&str>, reasons: &[String]) {
    out.extend(reasons.iter().map(|reason| Issue {
        side,
        at: at.map(str::to_owned),
        reason: reason.clone(),
    }));
}

fn add_inventory_issues(
    out: &mut Vec<Issue>,
    side: &'static str,
    issues: &[index::InventoryIssue],
) {
    out.extend(issues.iter().map(|issue| Issue {
        side,
        at: issue.at.clone(),
        reason: issue.reason.clone(),
    }));
}
