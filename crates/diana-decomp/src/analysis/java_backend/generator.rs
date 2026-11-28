use crate::analysis::{
    ClassMethodInput, MethodRecoveryFailure, MethodRecoveryStage, NestedClassInput,
};
use crate::frontend::{ClassNode, MethodNode};
use crate::ir::cfg::CFG;
use rayon::prelude::*;

use super::declaration_lowering::{JavaCompilationUnitLowering, JavaSingleMethodLowering};
use super::java_model::method::collect_param_debug_names;
use super::java_model::{
    JavaClassModel, JavaMethodDeclaration, JavaMethodModel, OuterInstanceField,
};
use super::method_pipeline::{MethodBodyAnalysis, MethodBodyPipeline};
use super::{JavaDecompiler, JavaDecompilerError};
use crate::language::java::JavaPrinter;

impl JavaDecompiler {
    /// Generate structured output for a method
    ///
    /// Pipeline:
    /// 1. Normalize CFG and construct SSA
    /// 2. Build exception, synchronization, loop, and switch regions
    /// 3. Resolve cross-region leaves and cleanup chains
    /// 4. Construct semantic IR from SCC and reaching-condition facts
    /// 5. Recover values and lower the Java AST
    pub fn generate_method(&mut self, cfg: &mut CFG) -> Result<String, JavaDecompilerError> {
        let current_type = cfg.method().owner().clone();
        let method = self.build_method_model(cfg)?;
        if let Some(failure) = &method.failure {
            observe_body_gap(self.observer.as_ref(), cfg, failure);
        }
        let declaration = JavaSingleMethodLowering::lower(
            method,
            None,
            &current_type,
            &self.source_abi,
            self.type_hierarchy.clone(),
            self.observer.clone(),
        )?;
        Ok(JavaPrinter::new(self.config.indent.clone())
            .with_preserved_dex_names(self.config.preserve_dex_names)
            .print_method_declaration(&declaration, 0)?)
    }

    pub fn generate_method_with_context(
        &mut self,
        class: &ClassNode,
        method: &MethodNode,
        cfg: &mut CFG,
    ) -> Result<String, JavaDecompilerError> {
        let exception_contracts = super::exception_contract::DeclaredExceptionAnalysis::new(
            self.source_abi.as_ref(),
            self.type_hierarchy.as_ref(),
        )
        .solve(class, std::iter::once((method, &*cfg)));
        let reference = crate::ir::MethodReference {
            owner: class.class_type().clone(),
            name: method.name().to_string(),
            descriptor: crate::ir::MethodDescriptor {
                parameters: method.param_types().to_vec(),
                return_type: method.return_type().clone(),
            },
        };
        let method = self.build_method_model_from_node(
            class,
            method,
            cfg,
            exception_contracts.get(&reference).map(Vec::as_slice),
            None,
            None,
            None,
            OuterInstanceField::analyze(class).as_ref(),
        )?;
        if let Some(failure) = &method.failure {
            observe_body_gap(self.observer.as_ref(), cfg, failure);
        }
        let declaration = JavaSingleMethodLowering::lower(
            method,
            source_package_name(class),
            class.class_type(),
            &self.source_abi,
            self.type_hierarchy.clone(),
            self.observer.clone(),
        )?;
        Ok(JavaPrinter::new(self.config.indent.clone())
            .with_preserved_dex_names(self.config.preserve_dex_names)
            .print_method_declaration(&declaration, 0)?)
    }

    /// Generate a class together with its nested inner classes, rendered as a
    /// single source file. Each inner class entry is `(ClassNode, methods)`
    /// in the same shape as the outer.
    pub(crate) fn generate_class_with_nested(
        &mut self,
        class: &ClassNode,
        methods: Vec<ClassMethodInput>,
        inner: Vec<NestedClassInput>,
    ) -> Result<String, JavaDecompilerError> {
        {
            self.generate_class_with_nested_impl(class, methods, inner)
        }
    }

    fn generate_class_with_nested_impl(
        &mut self,
        class: &ClassNode,
        methods: Vec<ClassMethodInput>,
        inner: Vec<NestedClassInput>,
    ) -> Result<String, JavaDecompilerError> {
        self.observer.checkpoint()?;
        let stats = std::env::var_os("DEXDEC_BATCH_STATS").is_some();
        let start = || stats.then(std::time::Instant::now);
        let elapsed = |started: Option<std::time::Instant>| {
            started.map_or(std::time::Duration::ZERO, |t| t.elapsed())
        };
        let signature_started = start();
        let source_signatures = super::signature_inference::SourceSignatureInference::analyze(
            self.type_hierarchy.as_ref(),
            class.class_type(),
            &methods,
            &inner,
        );
        let signature_ms = elapsed(signature_started);
        let methods_started = start();
        let (outer_methods, outer_instance) =
            { self.build_class_methods(class, methods, &source_signatures) }?;
        let methods_ms = elapsed(methods_started);
        self.observer.checkpoint()?;
        self.class_stage(class.type_descriptor(), "build_outer_methods:done");
        let nested_started = start();
        let nested_models = { self.build_nested_class_models(inner, &source_signatures) }?;
        let nested_ms = elapsed(nested_started);
        self.observer.checkpoint()?;
        self.class_stage(class.type_descriptor(), "build_nested_models:done");
        let model_started = start();
        let mut class_model = {
            JavaClassModel::from_class_node(class, outer_methods, outer_instance)
                .map(|model| model.with_nested(nested_models))
        }?;
        self.observer.checkpoint()?;
        class_model.assign_lexical_type_names(&self.source_abi);
        let model_ms = elapsed(model_started);
        self.class_stage(class.type_descriptor(), "class_model:done");
        let render_started = start();
        let source = {
            self.observer.checkpoint()?;
            self.render_class_model(class_model)
        }?;
        if stats {
            let render_ms = elapsed(render_started);
            let total = signature_ms + methods_ms + nested_ms + model_ms + render_ms;
            if total.as_millis() >= 200 {
                eprintln!(
                    "diana-decomp class {}: signatures={:.0}ms methods={:.0}ms nested={:.0}ms model={:.0}ms render={:.0}ms",
                    class.type_descriptor(),
                    signature_ms.as_secs_f64() * 1000.0,
                    methods_ms.as_secs_f64() * 1000.0,
                    nested_ms.as_secs_f64() * 1000.0,
                    model_ms.as_secs_f64() * 1000.0,
                    render_ms.as_secs_f64() * 1000.0,
                );
            }
        }
        Ok(source)
    }

    fn build_nested_class_models(
        &self,
        inputs: Vec<NestedClassInput>,
        source_signatures: &super::signature_inference::SourceSignatureInference<'_>,
    ) -> Result<Vec<JavaClassModel>, JavaDecompilerError> {
        self.build_nested_class_forest(inputs, source_signatures)
    }

    fn build_nested_class_forest(
        &self,
        inputs: Vec<NestedClassInput>,
        source_signatures: &super::signature_inference::SourceSignatureInference<'_>,
    ) -> Result<Vec<JavaClassModel>, JavaDecompilerError> {
        if self.parallel_methods && inputs.len() > 1 {
            inputs
                .into_par_iter()
                .map(|input| self.build_nested_class_tree(input, source_signatures))
                .collect()
        } else {
            inputs
                .into_iter()
                .map(|input| self.build_nested_class_tree(input, source_signatures))
                .collect()
        }
    }

    fn build_nested_class_tree(
        &self,
        input: NestedClassInput,
        source_signatures: &super::signature_inference::SourceSignatureInference<'_>,
    ) -> Result<JavaClassModel, JavaDecompilerError> {
        self.observer.checkpoint()?;
        self.class_stage(input.class.type_descriptor(), "build_nested_class:start");
        let (methods, outer_instance) =
            self.build_class_methods(&input.class, input.methods, source_signatures)?;
        let children = self.build_nested_class_forest(input.nested, source_signatures)?;
        self.class_stage(input.class.type_descriptor(), "build_nested_class:done");
        Ok(
            JavaClassModel::from_class_node(&input.class, methods, outer_instance)?
                .as_nested_source_member(&input.class)
                .with_nested(children),
        )
    }

    fn build_class_methods(
        &self,
        class: &ClassNode,
        methods: Vec<ClassMethodInput>,
        source_signatures: &super::signature_inference::SourceSignatureInference<'_>,
    ) -> Result<(Vec<JavaMethodModel>, Option<OuterInstanceField>), JavaDecompilerError> {
        let outer_instance = OuterInstanceField::analyze_cfgs(
            class,
            methods
                .iter()
                .filter_map(|input| input.cfg().map(|cfg| (input.method(class), cfg))),
        )
        .or_else(|| OuterInstanceField::analyze(class));
        let exception_contracts = super::exception_contract::DeclaredExceptionAnalysis::new(
            self.source_abi.as_ref(),
            self.type_hierarchy.as_ref(),
        )
        .solve(
            class,
            methods
                .iter()
                .filter_map(|input| input.cfg().map(|cfg| (input.method(class), cfg))),
        );
        let mut input_positions = vec![None; class.methods().len()];
        for (index, input) in methods.iter().enumerate() {
            input_positions[input.position()] = Some(index);
        }
        let is_function_object = super::FunctionObjectClass::analyze(class);
        let parallel = self.parallel_methods
            && methods
                .iter()
                .filter(|input| input.cfg().is_some())
                .nth(7)
                .is_some();
        let recover = |input: ClassMethodInput| {
            let decoded_method = input.method(class);
            let mut cfg = match input.into_cfg() {
                Ok(cfg) => cfg,
                Err(failure) => return Ok(Err(failure)),
            };
            let method_reference = cfg.method().reference();
            let inferred_parameter_types = source_signatures.parameter_types(method_reference);
            let inferred_return_type = source_signatures.return_type(method_reference);
            let exceptions = exception_contracts.get(method_reference).map(Vec::as_slice);
            let function_types = is_function_object
                .then(|| {
                    super::function_object_types::FunctionObjectMethodInference::infer(
                        decoded_method,
                        &class.interfaces,
                        &source_signatures.body_parameter_types(method_reference),
                        inferred_return_type.as_ref(),
                        &self.source_abi,
                    )
                })
                .flatten()
                .filter(|types| {
                    let interface = types.interface().erased();
                    class
                        .interfaces
                        .iter()
                        .any(|declared| declared == &interface)
                });
            let source_parameter_types = function_types
                .as_ref()
                .map(|types| types.parameters())
                .unwrap_or(inferred_parameter_types.as_slice());
            let descriptor = std::cell::LazyCell::new(|| decoded_method.info.descriptor());
            let observe_stage = |stage| {
                if self
                    .observer
                    .is_enabled(crate::ir::AnalysisEventKind::MethodStage)
                {
                    self.method_stage(
                        class.type_descriptor(),
                        &decoded_method.info.name,
                        &descriptor,
                        stage,
                    );
                }
            };
            observe_stage("start");
            let recovered = self.build_method_model_from_node(
                class,
                decoded_method,
                &mut cfg,
                exceptions,
                (!source_parameter_types.is_empty()).then_some(source_parameter_types),
                inferred_return_type.as_ref(),
                function_types.as_ref().map(|types| types.interface()),
                outer_instance.as_ref(),
            );
            match recovered {
                Ok(model) => {
                    if !parallel {
                        if let Some(failure) = &model.failure {
                            failure.observe(
                                self.observer.as_ref(),
                                class.type_descriptor(),
                                &decoded_method.info.name,
                                &descriptor,
                            );
                        }
                    }
                    observe_stage("done");
                    Ok(Ok(model))
                }
                Err(error) if error.is_cancelled() => Err(error),
                Err(error) => {
                    observe_stage("failed");
                    let stage = if matches!(error, JavaDecompilerError::GenericSignature(_)) {
                        MethodRecoveryStage::Metadata
                    } else {
                        MethodRecoveryStage::Semantics
                    };
                    let failure = MethodRecoveryFailure::new(stage, error);
                    if !parallel {
                        failure.observe(
                            self.observer.as_ref(),
                            class.type_descriptor(),
                            &decoded_method.info.name,
                            &descriptor,
                        );
                    }
                    Ok(Ok(JavaMethodModel::from_failure(
                        class,
                        decoded_method,
                        failure,
                    )))
                }
            }
        };
        let recovered = if parallel {
            methods.into_par_iter().map(recover).collect::<Vec<_>>()
        } else {
            methods.into_iter().map(recover).collect::<Vec<_>>()
        };
        if parallel {
            // Recovery failures precede declaration and lowering failures. Indexed method
            // results restore the original serial observation order within this stage.
            for (method, position) in class.methods().iter().zip(&input_positions) {
                let Some(index) = *position else { continue };
                if let Ok(Ok(model)) = &recovered[index] {
                    if let Some(failure) = &model.failure {
                        failure.observe(
                            self.observer.as_ref(),
                            class.type_descriptor(),
                            &method.info.name,
                            &method.info.descriptor(),
                        );
                    }
                }
            }
        }
        let mut recovered_models = recovered
            .into_iter()
            .map(|result| result.map(Some))
            .collect::<Result<Vec<_>, _>>()?;
        let mut built = Vec::with_capacity(class.methods().len());
        for (position, method) in class
            .methods()
            .iter()
            .enumerate()
            .filter(|(_, method)| self.physical_methods || !method.access_flags.is_bridge())
        {
            let Some(index) = input_positions[position] else {
                let declaration = match JavaMethodDeclaration::from_method_node(
                    class,
                    method,
                    self.source_abi.lexical_type_variables(class.class_type()),
                    ((class.access_flags.is_enum() || class.access_flags.is_annotation())
                        && method.signature.is_some())
                    .then(|| {
                        self.source_abi
                            .inherited_declaration_signature(class, method)
                    })
                    .flatten(),
                ) {
                    Ok(declaration) => declaration,
                    Err(error) => {
                        let failure =
                            MethodRecoveryFailure::new(MethodRecoveryStage::Metadata, error);
                        failure.observe(
                            self.observer.as_ref(),
                            class.type_descriptor(),
                            &method.info.name,
                            &method.info.descriptor(),
                        );
                        JavaMethodDeclaration::from_method_node_erased(class, method)
                    }
                };
                built.push(JavaMethodModel::from_declaration(declaration));
                continue;
            };
            let model = match recovered_models[index].take() {
                Some(Ok(model)) => model,
                failed => {
                    let failure = failed.and_then(Result::err).unwrap_or_else(|| {
                        MethodRecoveryFailure::new(
                            MethodRecoveryStage::Decode,
                            "decoded CFG is missing",
                        )
                    });
                    failure.observe(
                        self.observer.as_ref(),
                        class.type_descriptor(),
                        &method.info.name,
                        &method.info.descriptor(),
                    );
                    JavaMethodModel::from_failure(class, method, failure)
                }
            };
            built.push(model);
        }
        Ok((built, outer_instance))
    }

    fn build_method_model(&self, cfg: &mut CFG) -> Result<JavaMethodModel, JavaDecompilerError> {
        let declaration = JavaMethodDeclaration::from_cfg(cfg);
        let mut options = declaration.body_options(None);
        options.current_type = Some(cfg.method().owner().clone());
        let body = MethodBodyPipeline::new(self.type_hierarchy.as_ref(), self.observer.as_ref())
            .analyze(cfg)?;
        method_model_from_body_analysis(declaration, body, options)
    }

    fn build_method_model_from_node(
        &self,
        class: &ClassNode,
        method: &MethodNode,
        cfg: &mut CFG,
        inferred_exceptions: Option<&[crate::ir::ArgType]>,
        inferred_parameter_types: Option<&[Option<crate::ir::ArgType>]>,
        inferred_return_type: Option<&crate::ir::ArgType>,
        function_interface: Option<&crate::ir::generic_types::JvmTypeSignature>,
        outer_instance: Option<&OuterInstanceField>,
    ) -> Result<JavaMethodModel, JavaDecompilerError> {
        {
            self.build_method_model_from_node_impl(
                class,
                method,
                cfg,
                inferred_exceptions,
                inferred_parameter_types,
                inferred_return_type,
                function_interface,
                outer_instance,
            )
        }
    }

    fn build_method_model_from_node_impl(
        &self,
        class: &ClassNode,
        method: &MethodNode,
        cfg: &mut CFG,
        inferred_exceptions: Option<&[crate::ir::ArgType]>,
        inferred_parameter_types: Option<&[Option<crate::ir::ArgType>]>,
        inferred_return_type: Option<&crate::ir::ArgType>,
        function_interface: Option<&crate::ir::generic_types::JvmTypeSignature>,
        outer_instance: Option<&OuterInstanceField>,
    ) -> Result<JavaMethodModel, JavaDecompilerError> {
        let mut declaration = {
            JavaMethodDeclaration::from_method_node(
                class,
                method,
                self.source_abi.lexical_type_variables(class.class_type()),
                ((class.access_flags.is_enum() || class.access_flags.is_annotation())
                    && method.signature.is_some())
                .then(|| {
                    self.source_abi
                        .inherited_declaration_signature(class, method)
                })
                .flatten(),
            )
        }?;
        if declaration.throws.is_empty() {
            declaration
                .throws
                .extend(inferred_exceptions.into_iter().flatten().cloned());
        }
        if declaration.access_flags.is_synthetic()
            || super::FunctionObjectClass::analyze(class)
            || function_interface.is_some()
        {
            declaration.source_parameter_types = inferred_parameter_types
                .map(<[Option<crate::ir::ArgType>]>::to_vec)
                .unwrap_or_else(|| vec![None; declaration.parameters.len()]);
            declaration.source_return_type = inferred_return_type.cloned();
            declaration.function_interface = function_interface.cloned();
        }
        // `from_method_node` does not see the decoded debug stream, so attach
        // source parameter names here. SSA bindings remain owned exclusively
        // by MethodBodyAnalysis.
        {
            let param_names = collect_param_debug_names(cfg);
            for (idx, parameter) in declaration.parameters.iter_mut().enumerate() {
                if parameter.name.is_none() {
                    parameter.name = param_names
                        .get(idx)
                        .and_then(|name| name.as_deref())
                        .map(crate::language::java::JavaIdentifier::from_dex);
                }
            }
        };
        let mut options = declaration.body_options(Some(class));
        if let Some(outer_instance) = outer_instance {
            options.outer_instance = Some(outer_instance.clone());
        }
        let body = {
            MethodBodyPipeline::new(self.type_hierarchy.as_ref(), self.observer.as_ref())
                .analyze(cfg)
        }?;
        {
            JavaMethodModel::from_body_analysis_with_options(declaration, body, options)
        }
    }

    fn render_class_model(&self, class: JavaClassModel) -> Result<String, JavaDecompilerError> {
        let stats_name = std::env::var_os("DEXDEC_BATCH_STATS").map(|_| {
            class
                .declaration
                .current_type()
                .map(|ty| ty.to_string())
                .unwrap_or_else(|| class.declaration.name.to_string())
        });
        let lower_started = std::time::Instant::now();
        let unit = {
            JavaCompilationUnitLowering::lower(
                class,
                &self.source_abi,
                self.type_hierarchy.clone(),
                self.observer.clone(),
            )
        }?;
        let lower_ms = lower_started.elapsed();
        let print_started = std::time::Instant::now();
        let source = {
            JavaPrinter::new(self.config.indent.clone())
                .with_preserved_dex_names(self.config.preserve_dex_names)
                .print_compilation_unit(&unit)
        }?;
        if let Some(name) = stats_name {
            let print_ms = print_started.elapsed();
            if (lower_ms + print_ms).as_millis() >= 50 {
                eprintln!(
                    "diana-decomp render {}: lower={:.0}ms print={:.0}ms",
                    name,
                    lower_ms.as_secs_f64() * 1000.0,
                    print_ms.as_secs_f64() * 1000.0,
                );
            }
        }
        Ok(source)
    }

    fn class_stage(&self, class: &str, stage: &'static str) {
        self.observer
            .observe(crate::ir::AnalysisEvent::ClassStage { class, stage });
    }

    fn method_stage(&self, class: &str, method: &str, descriptor: &str, stage: &'static str) {
        self.observer
            .observe(crate::ir::AnalysisEvent::MethodStage {
                class,
                method,
                descriptor,
                stage,
            });
    }
}

fn method_model_from_body_analysis(
    declaration: JavaMethodDeclaration,
    body: MethodBodyAnalysis,
    options: super::java_model::MethodBodyOptions,
) -> Result<JavaMethodModel, JavaDecompilerError> {
    JavaMethodModel::from_body_analysis_with_options(declaration, body, options)
}

fn observe_body_gap(
    observer: &dyn crate::ir::AnalysisObserver,
    cfg: &CFG,
    failure: &MethodRecoveryFailure,
) {
    let method = cfg.method();
    failure.observe(
        observer,
        &method.owner().to_string(),
        method.name(),
        &method.descriptor().to_string(),
    );
}

fn source_package_name(class: &ClassNode) -> Option<&str> {
    let package = class.package();
    (!package.is_empty()).then_some(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{ClassMethodInput, MethodRecoveryFailure, MethodRecoveryStage};
    use crate::frontend::{AccessInfo, ClassInfo, MethodInfo};
    use crate::ir::ArgType;

    #[test]
    fn class_generation_isolates_failed_method() {
        let descriptor = "Lexample/Partial;";
        let info = ClassInfo::from_type_descriptor(descriptor).expect("class descriptor");
        let mut class = ClassNode::new(0, info, AccessInfo::for_class(0x0001));
        let method = MethodNode::new(
            0,
            MethodInfo::new(
                descriptor.to_string(),
                "broken".to_string(),
                Vec::new(),
                ArgType::INT,
            ),
            AccessInfo::for_method(0x0009),
        );
        class.add_method(method.clone());
        let methods = vec![ClassMethodInput::failed(
            0,
            MethodRecoveryFailure::new(MethodRecoveryStage::Semantics, "test failure"),
        )];

        let source = JavaDecompiler::new(Default::default())
            .generate_class_with_nested(&class, methods, Vec::new())
            .expect("partial class source");

        assert!(source.contains("class Partial"));
        assert!(source.contains("int broken()"));
        assert!(source.contains("Method could not be decompiled during semantic recovery"));
    }
}
