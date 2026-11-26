//! Canonical CFG normalization and SSA construction.

use crate::ir::analysis::{DominatorTree, SsaInvariantError, SsaValueGraph, TypeHierarchy};
use crate::ir::passes::{
    BindResults, CanonicalizationError, CanonicalizeCFG, CfgInvariantError,
    ConstructorRecoveryError, MonitorSplitError, Pass, PassResult, PhiSimplificationError,
    PhiSimplify, PruneUnreachable, RecoverConstructors, ResultBindingError, SSATransform,
    SplitCriticalEdges, SplitMonitorEntries, SsaTransformError, ValidateCFG,
};
use crate::ir::{AnalysisCancelled, CFG};

pub struct CfgPipeline<'a> {
    hierarchy: &'a dyn TypeHierarchy,
}

#[derive(Debug, Clone)]
pub struct CfgAnalysis {
    pub values: SsaValueGraph,
    pub change: PassResult,
}

#[derive(Debug)]
pub enum CfgPipelineError {
    Cancelled(AnalysisCancelled),
    Canonicalization(CanonicalizationError),
    ResultBinding(ResultBindingError),
    MonitorSplit(MonitorSplitError),
    SsaConstruction(SsaTransformError),
    PhiSimplification(PhiSimplificationError),
    ConstructorRecovery(ConstructorRecoveryError),
    Validation(CfgInvariantError),
    SsaInvariant(SsaInvariantError),
}

impl std::fmt::Display for CfgPipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled(error) => error.fmt(formatter),
            Self::Canonicalization(error) => write!(formatter, "CFG canonicalization: {error}"),
            Self::ResultBinding(error) => write!(formatter, "DEX result binding: {error}"),
            Self::MonitorSplit(error) => write!(formatter, "monitor boundary split: {error}"),
            Self::SsaConstruction(error) => write!(formatter, "SSA construction: {error}"),
            Self::PhiSimplification(error) => write!(formatter, "Phi simplification: {error}"),
            Self::ConstructorRecovery(error) => write!(formatter, "constructor recovery: {error}"),
            Self::Validation(error) => write!(formatter, "CFG invariant: {error}"),
            Self::SsaInvariant(error) => write!(formatter, "SSA invariant: {error}"),
        }
    }
}

impl std::error::Error for CfgPipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cancelled(source) => Some(source),
            Self::Canonicalization(source) => Some(source),
            Self::ResultBinding(source) => Some(source),
            Self::MonitorSplit(source) => Some(source),
            Self::SsaConstruction(source) => Some(source),
            Self::PhiSimplification(source) => Some(source),
            Self::ConstructorRecovery(source) => Some(source),
            Self::Validation(source) => Some(source),
            Self::SsaInvariant(source) => Some(source),
        }
    }
}

impl From<std::convert::Infallible> for CfgPipelineError {
    fn from(error: std::convert::Infallible) -> Self {
        match error {}
    }
}

impl From<AnalysisCancelled> for CfgPipelineError {
    fn from(error: AnalysisCancelled) -> Self {
        Self::Cancelled(error)
    }
}

macro_rules! cfg_error_conversion {
    ($source:ty, $variant:ident) => {
        impl From<$source> for CfgPipelineError {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}

cfg_error_conversion!(CanonicalizationError, Canonicalization);
cfg_error_conversion!(ResultBindingError, ResultBinding);
cfg_error_conversion!(MonitorSplitError, MonitorSplit);
cfg_error_conversion!(SsaTransformError, SsaConstruction);
cfg_error_conversion!(PhiSimplificationError, PhiSimplification);
cfg_error_conversion!(ConstructorRecoveryError, ConstructorRecovery);
cfg_error_conversion!(CfgInvariantError, Validation);
cfg_error_conversion!(SsaInvariantError, SsaInvariant);

impl<'a> CfgPipeline<'a> {
    pub fn new(hierarchy: &'a dyn TypeHierarchy) -> Self {
        Self { hierarchy }
    }

    pub fn run(&self, cfg: &mut CFG) -> Result<PassResult, CfgPipelineError> {
        self.analyze(cfg).map(|analysis| analysis.change)
    }

    pub fn run_observed(
        &self,
        cfg: &mut CFG,
        observer: &dyn crate::ir::AnalysisObserver,
    ) -> Result<PassResult, CfgPipelineError> {
        self.analyze_observed(cfg, observer)
            .map(|analysis| analysis.change)
    }

    pub fn analyze(&self, cfg: &mut CFG) -> Result<CfgAnalysis, CfgPipelineError> {
        self.analyze_observed(cfg, &crate::ir::NullAnalysisObserver)
    }

    pub fn analyze_observed(
        &self,
        cfg: &mut CFG,
        observer: &dyn crate::ir::AnalysisObserver,
    ) -> Result<CfgAnalysis, CfgPipelineError> {
        self.analyze_with_dominators(cfg, observer)
            .map(|(analysis, _)| analysis)
    }

    pub(crate) fn analyze_with_dominators(
        &self,
        cfg: &mut CFG,
        observer: &dyn crate::ir::AnalysisObserver,
    ) -> Result<(CfgAnalysis, DominatorTree), CfgPipelineError> {
        if cfg.is_analysis_prepared() {
            let (values, tree) = SsaValueGraph::build_with_dominators(cfg, None)?;
            return Ok((
                CfgAnalysis {
                    values,
                    change: PassResult::Unchanged,
                },
                tree,
            ));
        }

        cfg.capture_exception_coverage();

        let mut context = StageContext::new(cfg, observer);
        context.validate("input")?;
        let mut changed = NormalizationStage::default().run(&mut context)?;
        let mut facts = SsaPassFacts::default();
        changed |= SsaStage::new(self.hierarchy).run(&mut context, &mut facts)?;

        context.validate("ssa")?;
        context.cfg.identify_instructions();
        context.cfg.mark_analysis_prepared();
        let (mut values, tree) = facts.finish(context.cfg)?;
        values.refresh_phi_identities(context.cfg);
        Ok((
            CfgAnalysis {
                values,
                change: changed.into(),
            },
            tree,
        ))
    }
}

trait CfgStage {
    const NAME: &'static str;

    fn run(&mut self, context: &mut StageContext<'_>) -> Result<bool, CfgPipelineError>;
}

#[derive(Default)]
struct NormalizationStage {
    prune: PruneUnreachable,
    canonicalize: CanonicalizeCFG,
    bind_results: BindResults,
    split_monitors: SplitMonitorEntries,
    split_edges: SplitCriticalEdges,
}

impl CfgStage for NormalizationStage {
    const NAME: &'static str = "cfg";

    fn run(&mut self, context: &mut StageContext<'_>) -> Result<bool, CfgPipelineError> {
        let mut changed = false;
        loop {
            let mut round = context.pass(Self::NAME, &mut self.prune)?;
            round |= context.pass(Self::NAME, &mut self.canonicalize)?;
            changed |= round;
            if !round {
                break;
            }
        }
        changed |= context.pass(Self::NAME, &mut self.bind_results)?;
        changed |= context.pass(Self::NAME, &mut self.split_monitors)?;
        changed |= context.pass(Self::NAME, &mut self.split_edges)?;
        context.validate(Self::NAME)?;
        Ok(changed)
    }
}

// Value passes may reuse only the exact CFG snapshot they checked. A pass
// that rewrites instructions or edges drops it before the next pass runs.
#[derive(Default)]
pub(super) struct SsaPassFacts {
    snapshot: Option<(SsaValueGraph, crate::ir::analysis::DominatorTree)>,
    reusable_tree: Option<DominatorTree>,
}

impl SsaPassFacts {
    pub(super) fn get(
        &mut self,
        cfg: &CFG,
    ) -> Result<&(SsaValueGraph, crate::ir::analysis::DominatorTree), SsaInvariantError> {
        match self.snapshot {
            Some(ref snapshot) => Ok(snapshot),
            None => Ok(self.snapshot.insert(SsaValueGraph::build_with_dominators(
                cfg,
                self.reusable_tree.take(),
            )?)),
        }
    }

    fn finish(self, cfg: &CFG) -> Result<(SsaValueGraph, DominatorTree), SsaInvariantError> {
        match self.snapshot {
            Some(pair) => Ok(pair),
            None => SsaValueGraph::build_with_dominators(cfg, self.reusable_tree),
        }
    }

    pub(super) fn invalidate_values(&mut self) {
        if let Some((_, tree)) = self.snapshot.take() {
            self.reusable_tree = Some(tree);
        }
    }

    pub(super) fn invalidate(&mut self) {
        *self = Self::default();
    }
}

struct SsaStage<'a> {
    construct: SSATransform,
    simplify: PhiSimplify,
    constructors: RecoverConstructors<'a>,
    prune: PruneUnreachable,
}

impl<'a> SsaStage<'a> {
    fn new(hierarchy: &'a dyn TypeHierarchy) -> Self {
        Self {
            construct: SSATransform,
            simplify: PhiSimplify,
            constructors: RecoverConstructors::new(hierarchy),
            prune: PruneUnreachable,
        }
    }
}

impl SsaStage<'_> {
    const NAME: &'static str = "ssa";

    fn run(
        &mut self,
        context: &mut StageContext<'_>,
        facts: &mut SsaPassFacts,
    ) -> Result<bool, CfgPipelineError> {
        let mut changed = context.pass_with(Self::NAME, self.construct.name(), |cfg| {
            // Phi placement and renaming only change instructions. The same
            // all-edge tree is valid for the SSA uses and subsequent regions.
            let (change, tree) = SSATransform::construct(cfg)?;
            facts.reusable_tree = Some(tree);
            Ok::<_, SsaTransformError>(change)
        })?;
        changed |= context.pass_with(Self::NAME, self.simplify.name(), |cfg| {
            self.simplify.run_reusing(cfg, facts)
        })?;
        let constructors_changed =
            context.pass_with(Self::NAME, self.constructors.name(), |cfg| {
                self.constructors.run_reusing(cfg, facts)
            })?;
        changed |= constructors_changed;
        if constructors_changed {
            // The all-edge dominator tree covers every physical block. A rewrite
            // that retained it can only need stale handler declarations pruned.
            changed |= context.pass_with(Self::NAME, self.prune.name(), |cfg| {
                if facts.reusable_tree.is_some() {
                    Ok(PruneUnreachable::prune_handlers(cfg))
                } else {
                    self.prune.run(cfg)
                }
            })?;
            changed |= context.pass_with(Self::NAME, self.simplify.name(), |cfg| {
                self.simplify.run_reusing(cfg, facts)
            })?;
        }
        context.validate(Self::NAME)?;
        Ok(changed)
    }
}

struct StageContext<'a> {
    cfg: &'a mut CFG,
    observer: &'a dyn crate::ir::AnalysisObserver,
    validated: bool,
}

impl<'a> StageContext<'a> {
    fn new(cfg: &'a mut CFG, observer: &'a dyn crate::ir::AnalysisObserver) -> Self {
        Self {
            cfg,
            observer,
            validated: false,
        }
    }

    fn pass<P>(&mut self, stage: &'static str, pass: &mut P) -> Result<bool, CfgPipelineError>
    where
        P: Pass,
        CfgPipelineError: From<P::Error>,
    {
        self.pass_with(stage, pass.name(), |cfg| pass.run(cfg))
    }

    fn pass_with<E>(
        &mut self,
        stage: &'static str,
        name: &'static str,
        run: impl FnOnce(&mut CFG) -> Result<PassResult, E>,
    ) -> Result<bool, CfgPipelineError>
    where
        CfgPipelineError: From<E>,
    {
        self.observer.checkpoint()?;
        self.validated = false;
        let result = run(self.cfg).map_err(CfgPipelineError::from)?;
        self.observe(stage, name, result);
        self.observer.checkpoint()?;
        Ok(result == PassResult::Changed)
    }

    fn validate(&mut self, stage: &'static str) -> Result<(), CfgPipelineError> {
        self.observer.checkpoint()?;
        let mut validator = ValidateCFG;
        // Adjacent stage boundaries may publish the same immutable CFG. Keep
        // both observer events and checkpoints while checking that snapshot once.
        if !self.validated {
            validator.run(self.cfg)?;
            self.validated = true;
        }
        self.observe(stage, validator.name(), PassResult::Unchanged);
        self.observer.checkpoint()?;
        Ok(())
    }

    fn observe(&self, phase: &'static str, name: &'static str, result: PassResult) {
        self.observer
            .observe(crate::ir::AnalysisEvent::CfgTransform {
                phase,
                name,
                result,
                cfg: self.cfg,
            });
    }
}
