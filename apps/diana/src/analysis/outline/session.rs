//! Request-local, lazy Engine cache. A physical root is inflated/opened once for
//! expansion; P0 facts and owned method displays share their ordinary Arc facts.
use super::OutlineError;
use crate::analysis::{
    apk::DexRoot,
    at,
    decompile::{self, DecompileError, Decompiled, Selection},
    images::LogicalDex,
    selector::{MethodSelector, SelectorError},
};
use diana_decomp::{
    ByteOffset, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId, RequestError,
    input::DexContainer,
};

struct Cached<'a> {
    engine: Engine<'a>,
    headers: Vec<ByteOffset>,
}
pub(super) struct Session<'a, 'input> {
    roots: &'a [DexRoot<'input>],
    engines: Vec<Option<Cached<'a>>>,
}
impl<'a, 'input> Session<'a, 'input> {
    pub fn new(roots: &'a [DexRoot<'input>]) -> Result<Self, OutlineError> {
        let mut engines = Vec::new();
        engines
            .try_reserve_exact(roots.len())
            .map_err(RequestError::Allocation)?;
        engines.resize_with(roots.len(), || None);
        Ok(Self { roots, engines })
    }
    pub fn roots(&self) -> &'a [DexRoot<'input>] {
        self.roots
    }

    pub fn analyze(&mut self, selector: &MethodSelector) -> Result<Decompiled, OutlineError> {
        let entry = selector.class.entry.ok_or(OutlineError::Invariant(
            "expansion class has no original entry",
        ))?;
        let ordinal = self
            .roots
            .iter()
            .position(|r| r.entry().map_or(0, |e| e.index().0) == entry as usize)
            .ok_or(SelectorError::NotFound)
            .map_err(DecompileError::from)?;
        if self.engines[ordinal].is_none() {
            let root: &'a DexRoot<'input> = &self.roots[ordinal];
            let bytes = root.bytes()?;
            let headers = DexContainer::open(bytes)
                .map_err(DecompileError::from)?
                .logical_headers()
                .map_err(DecompileError::from)?;
            let mut images = Vec::new();
            images
                .try_reserve_exact(headers.len())
                .map_err(RequestError::Allocation)?;
            for &header_offset in &headers {
                images.push(DexImage {
                    bytes,
                    header_offset,
                    origin: ImageOrigin {
                        input: InputId(u64::from(entry)),
                    },
                });
            }
            let engine = Engine::open(images)
                .map_err(DecompileError::from)?
                .with_outline();
            self.engines[ordinal] = Some(Cached { engine, headers });
        }
        let cached = self.engines[ordinal]
            .as_ref()
            .ok_or(OutlineError::Invariant(
                "expansion Engine was not delivered",
            ))?;
        let logical = selector.class.logical.ok_or(OutlineError::Invariant(
            "expansion class has no logical identity",
        ))?;
        let header_offset = *cached
            .headers
            .get(logical as usize)
            .ok_or(SelectorError::InvalidIndex)
            .map_err(DecompileError::from)?;
        let class = ClassKey {
            dex: DexId(logical),
            class_def: diana_decomp::ClassDefId(selector.class.class_def.ok_or(
                OutlineError::Invariant("expansion class has no definition identity"),
            )?),
        };
        let facts = cached.engine.class_facts(class)?;
        if facts.descriptor().units() != selector.class.descriptor {
            return Err(DecompileError::from(SelectorError::InvalidIndex).into());
        }
        let method = decompile::select_method(&facts, selector)?;
        let result = cached.engine.analyze_method(class, method)?;
        let entry = self.roots[ordinal].entry().map(|e| e.index().0);
        let class_at = at::class(
            entry,
            logical,
            class.class_def.0,
            &selector.class.descriptor,
        );
        let method_at = at::method(
            entry,
            logical,
            class.class_def.0,
            method.0,
            &selector.class.descriptor,
            &selector.name,
            &selector.prototype,
        );
        Ok(Decompiled {
            selection: Selection {
                entry,
                logical: LogicalDex(logical),
                header_offset,
                class,
                method: Some(method),
                class_at,
                method_at: Some(method_at),
            },
            rendered: result.rendered,
        })
    }
}
