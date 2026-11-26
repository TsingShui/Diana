use crate::analysis::{value_recovery::ValueRecovery, SemanticTransform};
use crate::ir::semantic::SemanticDeadCodeElimination;
use crate::ir::{
    analysis::{SourceVariableAllocation, TypeHierarchy, TypeSolver},
    cfg::CFG,
    passes::CfgPipeline,
    structure::RegionReducer,
    ArgType, ExceptionAnalyzer, MemberReference, RegionGraphBuilder, SemanticMethod, SemanticNode,
    SemanticVisitor, SourceSyntaxSemantics, StringBuildingRecovery,
};
use crate::language::java::{JavaValueSyntax, SourceSyntaxRecovery};

use super::JavaDecompilerError;

pub(super) struct MethodBodyAnalysis {
    pub body: SemanticNode,
    pub types: crate::ir::analysis::SourceTypeEnvironment,
    pub is_static: bool,
    pub this_code_var: Option<u32>,
    pub parameter_code_vars: Vec<Option<u32>>,
    pub type_uses: BodyTypeUses,
}

pub(super) struct MethodBodyPipeline<'a> {
    hierarchy: &'a dyn TypeHierarchy,
    observer: &'a dyn crate::ir::AnalysisObserver,
}

impl<'a> MethodBodyPipeline<'a> {
    pub(super) fn new(
        hierarchy: &'a dyn TypeHierarchy,
        observer: &'a dyn crate::ir::AnalysisObserver,
    ) -> Self {
        Self {
            hierarchy,
            observer,
        }
    }

    pub(super) fn analyze(&self, cfg: &mut CFG) -> Result<MethodBodyAnalysis, JavaDecompilerError> {
        self.analyze_impl(cfg)
    }

    fn analyze_impl(&self, cfg: &mut CFG) -> Result<MethodBodyAnalysis, JavaDecompilerError> {
        let stats = method_stats_enabled();
        let mut stages: Vec<(&str, u128)> = Vec::new();
        let mark = |stages: &mut Vec<(&str, u128)>,
                    name: &'static str,
                    start: Option<std::time::Instant>| {
            if let Some(start) = start {
                stages.push((name, start.elapsed().as_micros()));
            }
        };

        self.observer.checkpoint()?;
        let cfg_pipeline = CfgPipeline::new(self.hierarchy);
        let t = stats.then(std::time::Instant::now);
        let (cfg_analysis, dominators) =
            cfg_pipeline.analyze_with_dominators(cfg, self.observer)?;
        mark(&mut stages, "cfg_ssa", t);
        self.observe_stage(cfg, "cfg_ssa:done")?;
        let ssa_values = cfg_analysis.values;

        let t = stats.then(std::time::Instant::now);
        let exception_analysis =
            { ExceptionAnalyzer::new(cfg, &ssa_values, self.hierarchy).analyze() }?;
        mark(&mut stages, "exceptions", t);
        self.observe_stage(cfg, "exceptions:done")?;
        self.observer.observe(crate::ir::AnalysisEvent::Exceptions {
            cfg,
            analysis: &exception_analysis,
        });
        self.observer
            .observe(crate::ir::AnalysisEvent::ControlFlow(cfg));

        let t = stats.then(std::time::Instant::now);
        let region_graph = RegionGraphBuilder::new(cfg, &exception_analysis, &ssa_values)
            .build_with_dominators(Some(dominators))?;
        drop(exception_analysis);
        mark(&mut stages, "regions", t);
        self.observe_stage(cfg, "regions:done")?;
        self.observer.observe(crate::ir::AnalysisEvent::Regions {
            cfg,
            graph: &region_graph,
        });
        let t = stats.then(std::time::Instant::now);
        let body = {
            RegionReducer::new(cfg, &region_graph, self.observer)
                .and_then(|reducer| reducer.reduce())
                .map_err(JavaDecompilerError::from)
        }?;
        mark(&mut stages, "structure", t);
        self.observe_stage(cfg, "structure:done")?;
        self.observe_semantics(cfg, crate::ir::SemanticStage::Structured, &body);
        let semantic = SemanticMethod::from_ssa(body, region_graph, ssa_values);
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify1", t);
        let mut value_recovery = ValueRecovery::from_structured(cfg, &semantic);
        let t = stats.then(std::time::Instant::now);
        let semantic = { value_recovery.transform(semantic) }?;
        mark(&mut stages, "value_recovery", t);
        self.observer
            .observe(crate::ir::AnalysisEvent::ValueRecovery {
                cfg,
                diagnostics: value_recovery.diagnostics(),
            });
        self.observe_stage(cfg, "values:done")?;
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify2", t);
        self.observe_semantics(
            cfg,
            crate::ir::SemanticStage::ValuesRecovered,
            semantic.body(),
        );
        let t = stats.then(std::time::Instant::now);
        let types = {
            TypeSolver::new(self.hierarchy).solve(
                cfg,
                semantic.state().values(),
                semantic.state().constants(),
            )
        }?;
        mark(&mut stages, "types", t);
        self.observe_stage(cfg, "types:done")?;
        let t = stats.then(std::time::Instant::now);
        let source_variables = SourceVariableAllocation::analyze_with_flow(
            cfg,
            semantic.state().values(),
            semantic.state().constants(),
            semantic.state().recovered_phis(),
            semantic.body(),
            &types,
            self.hierarchy,
            semantic.state().regions(),
            || value_recovery.semantic_flow(semantic.body()),
        )?;
        mark(&mut stages, "source_analysis", t);
        self.observe_stage(cfg, "source_analysis:done")?;
        let t = stats.then(std::time::Instant::now);
        let mut semantic = { source_variables.apply(cfg, semantic, types, self.hierarchy) }?;
        drop(source_variables);
        mark(&mut stages, "source_apply", t);
        value_recovery.bind_source_inputs(cfg);
        self.observe_stage(cfg, "source_apply:done")?;
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify3", t);
        self.observe_semantics(
            cfg,
            crate::ir::SemanticStage::SourceAllocated,
            semantic.body(),
        );
        let t = stats.then(std::time::Instant::now);
        { value_recovery.prepare_source(&mut semantic) }?;
        if cfg.method().descriptor().return_type == ArgType::VOID {
            semantic.normalize_void_method_completion()?;
        }
        mark(&mut stages, "source_prepare", t);
        self.observe_stage(cfg, "source_prepare:done")?;
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify4", t);
        self.observe_semantics(
            cfg,
            crate::ir::SemanticStage::SourceVariables,
            semantic.body(),
        );
        let t = stats.then(std::time::Instant::now);
        let mut semantic = { SourceSyntaxRecovery::new(self.hierarchy).transform(semantic) }?;
        mark(&mut stages, "java_syntax", t);
        self.observe_stage(cfg, "java_syntax:done")?;
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify5", t);
        self.observe_semantics(cfg, crate::ir::SemanticStage::SourceSyntax, semantic.body());
        let t = stats.then(std::time::Instant::now);
        let fp_iters =
            { JavaValueFixedPoint::new(&mut value_recovery, self.hierarchy).apply(&mut semantic) }?;
        drop(value_recovery);
        mark(&mut stages, "java_fixed_point", t);
        if stats {
            stages.push(("fp_iters", u128::from(fp_iters)));
        }
        self.observe_stage(cfg, "java_values:done")?;
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify6", t);
        semantic.compact()?;
        self.observe_stage(cfg, "compact:done")?;
        let t = stats.then(std::time::Instant::now);
        semantic.verify()?;
        mark(&mut stages, "verify7", t);
        self.observe_semantics(cfg, crate::ir::SemanticStage::Normalized, semantic.body());
        if stats {
            let total: u128 = stages
                .iter()
                .filter(|(name, _)| *name != "fp_iters")
                .map(|(_, us)| *us)
                .sum();
            if total >= 5_000 {
                let method = cfg.method();
                eprintln!(
                    "diana-decomp method {}->{}{} total={:.2}ms fp_iters={} [{}]",
                    method.owner(),
                    method.name(),
                    method.descriptor(),
                    total as f64 / 1000.0,
                    fp_iters,
                    stages
                        .iter()
                        .map(|(name, us)| {
                            if *name == "fp_iters" {
                                format!("{name}={us}")
                            } else {
                                format!("{name}={:.2}", *us as f64 / 1000.0)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
        }
        if self
            .observer
            .is_enabled(crate::ir::AnalysisEventKind::IncompleteMethod)
            && cfg.method().descriptor().return_type != ArgType::VOID
            && crate::ir::semantic::SemanticCompletion::analyze(semantic.body())
                .can_complete_normally()
        {
            self.observer
                .observe(crate::ir::AnalysisEvent::IncompleteMethod {
                    cfg,
                    stage: crate::ir::SemanticStage::Normalized,
                });
        }
        let type_uses = MethodTypeUses::collect(&semantic)?;
        self.observe_stage(cfg, "type_uses:done")?;

        let (body, state) = semantic.into_parts();
        Ok(MethodBodyAnalysis {
            body,
            types: state.into_types(),
            is_static: cfg.method().is_static(),
            this_code_var: cfg.this_code_variable(),
            parameter_code_vars: cfg.parameter_code_variables().to_vec(),
            type_uses,
        })
    }

    fn observe_semantics(&self, cfg: &CFG, stage: crate::ir::SemanticStage, root: &SemanticNode) {
        self.observer
            .observe(crate::ir::AnalysisEvent::Semantics { cfg, stage, root });
    }

    fn observe_stage(&self, cfg: &CFG, stage: &'static str) -> Result<(), JavaDecompilerError> {
        self.observer
            .observe(crate::ir::AnalysisEvent::MethodPipeline { cfg, stage });
        self.observer.checkpoint()?;
        Ok(())
    }
}

/// Alternates source-identity scheduling and Java expression canonicalization
/// until neither can expose another simplification.
///
/// Every component is monotone: scheduling removes definitions or substitutes
/// their uses, string building merges statements into the one that allocates
/// their builder, and syntax recovery replaces expressions with cheaper
/// equivalents. The fixed point therefore terminates without an iteration cap.
struct JavaValueFixedPoint<'a, 'hierarchy> {
    values: &'a mut ValueRecovery,
    syntax: JavaValueSyntax<'hierarchy>,
}

impl<'a, 'hierarchy> JavaValueFixedPoint<'a, 'hierarchy> {
    fn new(values: &'a mut ValueRecovery, hierarchy: &'hierarchy dyn TypeHierarchy) -> Self {
        Self {
            values,
            syntax: JavaValueSyntax::new(hierarchy),
        }
    }

    fn apply(
        &mut self,
        method: &mut SemanticMethod<SourceSyntaxSemantics>,
    ) -> Result<u32, JavaDecompilerError> {
        let mut iterations = 0u32;
        loop {
            iterations += 1;
            self.values.recover_source(method)?;
            let building_changed = StringBuildingRecovery::apply(method.body_mut())?;
            let syntax_changed = self.syntax.apply(method)?;
            let dead_changed = SemanticDeadCodeElimination::apply(method.body_mut())?;
            if !building_changed && !syntax_changed && !dead_changed {
                return Ok(iterations);
            }
        }
    }
}

fn method_stats_enabled() -> bool {
    std::env::var_os("DEXDEC_METHOD_STATS").is_some()
}

#[derive(Debug, Clone)]
pub(super) struct BodyTypeUses {
    pub uses: std::collections::BTreeSet<ArgType>,
    pub static_owners: Box<[ArgType]>,
    pub lambda_types: Box<[ArgType]>,
}

struct MethodTypeUses<'a> {
    types: &'a crate::ir::analysis::SourceTypeEnvironment,
    uses: std::collections::BTreeSet<ArgType>,
    static_owners: std::collections::BTreeSet<ArgType>,
    lambda_types: std::collections::BTreeSet<ArgType>,
    error: Option<crate::ir::analysis::TypeConstraintError>,
}

impl<'a> MethodTypeUses<'a> {
    fn collect(
        method: &'a SemanticMethod<SourceSyntaxSemantics>,
    ) -> Result<BodyTypeUses, JavaDecompilerError> {
        let mut uses = method
            .state()
            .types()
            .known_types()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        uses.extend([
            ArgType::object("java/lang/Float"),
            ArgType::object("java/lang/Double"),
            ArgType::object("java/lang/Long"),
        ]);
        let mut collector = Self {
            types: method.state().types(),
            uses,
            static_owners: Default::default(),
            lambda_types: Default::default(),
            error: None,
        };
        collector.visit_node(method.body());
        match collector.error {
            Some(error) => Err(error.into()),
            None => Ok(BodyTypeUses {
                uses: collector.uses,
                static_owners: collector.static_owners.into_iter().collect(),
                lambda_types: collector.lambda_types.into_iter().collect(),
            }),
        }
    }

    fn insert(&mut self, ty: &ArgType) {
        let mut ty = ty;
        while ty.is_known() {
            if !self.uses.contains(ty) {
                self.uses.insert(ty.clone());
            }
            match ty {
                ArgType::Array(element) => ty = element,
                _ => break,
            }
        }
    }
}

impl SemanticVisitor for MethodTypeUses<'_> {
    fn enter_node(&mut self, node: &SemanticNode) {
        let catches = match node {
            SemanticNode::Try { catches, .. } => Some(catches.as_slice()),
            _ => None,
        };
        if let Some(catches) = catches {
            for ty in catches
                .iter()
                .flat_map(|catch| catch.exception_types.iter())
            {
                self.insert(ty);
            }
        }
    }

    fn enter_operation(&mut self, operation: &crate::ir::SemanticOperation) {
        if let Some(result) = &operation.result {
            match self.types.register_type(result) {
                Ok(ty) => self.insert(ty),
                Err(error) if self.error.is_none() => self.error = Some(error),
                Err(_) => {}
            }
        }
        if let Some(lambda) = operation.payload.lambda.as_deref() {
            self.lambda_types.insert(lambda.interface.clone());
            self.lambda_types
                .insert(lambda.implementation.owner.clone());
        }
        self.insert_option(operation.payload.class_type.as_deref());
        self.insert_option(operation.payload.cast_type.as_deref());
        match operation.payload.reference.as_deref() {
            Some(MemberReference::Field(field)) => {
                if matches!(
                    operation.insn_type,
                    crate::ir::InsnType::Sget | crate::ir::InsnType::Sput
                ) && !self.static_owners.contains(&field.owner)
                {
                    self.static_owners.insert(field.owner.clone());
                }
                self.insert(&field.owner);
                self.insert(&field.field_type);
            }
            Some(MemberReference::Method(method)) => {
                if operation.payload.invoke_type == Some(crate::ir::InvokeType::Static)
                    && !self.static_owners.contains(&method.owner)
                {
                    self.static_owners.insert(method.owner.clone());
                }
                self.insert(&method.owner);
                for ty in &method.descriptor.parameters {
                    self.insert(ty);
                }
                self.insert(&method.descriptor.return_type);
            }
            None => {}
        }
    }

    fn visit_register(&mut self, register: &crate::ir::RegisterArg) {
        match self.types.register_type(register) {
            Ok(ty) => self.insert(ty),
            Err(error) if self.error.is_none() => self.error = Some(error),
            Err(_) => {}
        }
    }

    fn visit_binding(
        &mut self,
        _kind: crate::ir::SemanticBindingKind,
        register: &crate::ir::RegisterArg,
    ) {
        self.insert(&register.ty);
        self.visit_register(register);
    }
}

impl MethodTypeUses<'_> {
    fn insert_option(&mut self, ty: Option<&ArgType>) {
        if let Some(ty) = ty {
            self.insert(ty);
        }
    }
}
