//! Bounded delivery of per-definition source and local unavailability.

use std::io;

use diana_decomp::input::DexContainer;
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
}

#[derive(thiserror::Error, Debug)]
enum VisitError {
    #[error(transparent)]
    Engine(#[from] RequestError),
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
            let result = engine.visit_class_sources(dex, jobs, |class, result| {
                let event = match result {
                    Ok((descriptor, source)) => SourceEvent::Class {
                        entry,
                        logical,
                        class_def: class.0,
                        descriptor,
                        text: source.text,
                        failures: source.failures,
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
            }
        }
    }
    Ok(())
}
