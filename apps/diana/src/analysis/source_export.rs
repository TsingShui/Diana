//! Bounded delivery of per-definition source and local unavailability.

use std::io;

use diana_decomp::input::DexContainer;
use diana_decomp::jvm::{self, ClassFailure, ClassText, SourceError};

use super::jvm::{JvmRoots, Location};
use super::query::{QueryError, QueryIssue};
use diana_decomp::{DexId, DexImage, Engine, ImageOrigin, InputId, MethodFailure, RequestError};

use super::request::RequestScope;

pub enum SourceEvent {
    Class {
        entry: Option<usize>,
        logical: u32,
        class_def: u32,
        descriptor: Vec<u16>,
        text: String,
        failures: Vec<MethodFailure>,
    },
    Issue {
        entry: Option<usize>,
        logical: Option<u32>,
        class_def: Option<u32>,
        reason: String,
    },
    JvmClass {
        at: Location,
        versioned: bool,
        source: ClassText,
    },
    JvmIssue(QueryIssue),
}

#[derive(thiserror::Error, Debug)]
enum VisitError {
    #[error(transparent)]
    Engine(#[from] RequestError),
    #[error(transparent)]
    Jvm(#[from] SourceError),
    #[error(transparent)]
    Sink(#[from] io::Error),
}

/// Visits one physical DEX at a time and delivers definitions concurrently
/// within its logical images. Root, image and class failures carry their
/// original coordinates and leave siblings available. Sink errors stop output.
pub fn visit(
    scope: &RequestScope,
    jobs: usize,
    sink: impl Fn(SourceEvent) -> io::Result<()> + Sync,
) -> io::Result<()> {
    let roots = scope.roots().map_err(io::Error::other)?;
    let mut reader = super::archive::ExportReader::new(scope.input());
    let mut scratch = Vec::new();
    for root in roots {
        let entry = root.entry().map(|record| record.index().0);
        let issue = |logical, reason| {
            sink(SourceEvent::Issue {
                entry,
                logical,
                class_def: None,
                reason,
            })
        };
        let payload = match root.entry() {
            Some(entry) => reader.read_entry(entry, &mut scratch),
            None => Ok(scope.input().bytes()),
        };
        let bytes = match payload {
            Ok(bytes) => bytes,
            Err(error) => {
                issue(None, error.to_string())?;
                continue;
            }
        };
        let headers =
            match DexContainer::open(bytes).and_then(|container| container.logical_headers()) {
                Ok(headers) => headers,
                Err(error) => {
                    issue(None, error.to_string())?;
                    continue;
                }
            };
        let origin = ImageOrigin {
            input: InputId(u64::try_from(entry.unwrap_or(0)).map_err(io::Error::other)?),
        };
        for (logical, header_offset) in headers.into_iter().enumerate() {
            let logical = u32::try_from(logical).map_err(io::Error::other)?;
            // A focused Engine sees precisely this logical image. The callback
            // maps every local ClassKey back to its physical entry and header.
            let dex = DexId(logical);
            let image = DexImage {
                bytes,
                origin,
                header_offset,
            };
            let engine = match Engine::focus(image, dex) {
                Ok(engine) => engine,
                Err(error) => {
                    issue(Some(logical), error.to_string())?;
                    continue;
                }
            };
            let result = engine.visit_class_texts(dex, jobs, |class, result| {
                let event = match result {
                    Ok((descriptor, text, failures)) => SourceEvent::Class {
                        entry,
                        logical,
                        class_def: class.0,
                        descriptor,
                        text,
                        failures,
                    },
                    Err(error) => SourceEvent::Issue {
                        entry,
                        logical: Some(logical),
                        class_def: Some(class.0),
                        reason: error.to_string(),
                    },
                };
                sink(event).map_err(VisitError::Sink)
            });
            match result {
                Ok(()) => {}
                Err(VisitError::Engine(error)) => issue(Some(logical), error.to_string())?,
                Err(VisitError::Sink(error)) => return Err(error),
                Err(error @ VisitError::Jvm(_)) => return Err(io::Error::other(error)),
            }
        }
    }
    Ok(())
}

/// Visits each JVM class through the same engine path as get-class.
/// The sink runs on the claiming worker, so source text does not queue.
pub fn visit_jvm(
    roots: &JvmRoots<'_>,
    jobs: usize,
    sink: impl Fn(SourceEvent) -> io::Result<()> + Sync,
) -> io::Result<()> {
    for issue in roots.issues() {
        sink(SourceEvent::JvmIssue(issue.clone()))?;
    }
    let result = jvm::visit_class_texts(
        roots.len(),
        jobs,
        || (roots.reader(), roots),
        |(reader, roots), ordinal| reader.read(roots.get(ordinal).ok_or(QueryError::WorkerLost)?),
        |ordinal, result| {
            let root = roots
                .get(ordinal)
                .ok_or_else(|| VisitError::Sink(io::Error::other("missing original JVM class")))?;
            let at = root.location();
            let event = match result {
                Ok(source) => SourceEvent::JvmClass {
                    at: at.clone(),
                    versioned: root
                        .entry_name()
                        .is_some_and(|name| name.starts_with("META-INF/versions/")),
                    source,
                },
                Err(ClassFailure::Read(error)) => {
                    SourceEvent::JvmIssue(QueryIssue::new(at.to_string(), &error))
                }
                Err(ClassFailure::Recover(error)) => SourceEvent::JvmIssue(QueryIssue {
                    context: at.to_string(),
                    kind: format!("{:?}", error.kind()),
                    reason: error.to_string(),
                }),
            };
            sink(event).map_err(VisitError::Sink)
        },
    );
    result.map_err(|error| match error {
        VisitError::Sink(error) => error,
        error => io::Error::other(error),
    })
}
