//! P9 prints frozen P7/P8 products. No decoder, solver or reconstruction entry.
mod body;
mod buffer;
mod classes;
mod error;
mod expressions;
mod names;
mod outline;
mod product;
mod raw;
mod values;
pub use error::*;
pub use outline::{Outline, OutlineCall, OutlineLine, OutlineTarget};
pub use product::*;

pub(crate) fn render(task: &crate::engine::RenderTask, source: SourceProduct) -> RenderOutcome {
    let mut out = buffer::Buffer::new(task.outline);
    let mut result = classes::render(&mut out, &source);
    // Retain a stopped writer's unfinished line without changing its text. An
    // existing stop keeps its original cause if further retention cannot allocate.
    if let Err(error) = out.finish_outline()
        && result.is_ok()
    {
        result = Err(error);
    }
    match result {
        Ok(()) => RenderOutcome::Ready(Rendered {
            class: source.class(),
            notation: "diana-java-like-v1",
            outline: out.outline,
            source,
            text: out.text,
            spans: out.spans,
            issues: out.issues,
        }),
        Err(error) => RenderOutcome::Stopped(RenderStop {
            outline: out.outline,
            issues: out.issues,
            source,
            error,
            text: out.text,
            spans: out.spans,
        }),
    }
}
