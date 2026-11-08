//! Query-only expansion of original references through the shared checked readers.
//! This is not dispatch analysis and does not construct method-analysis products.

use super::values::{EncodedKind, ValueStep};
use super::{MethodError, ProgramIndex, Prototype, ReferenceFacts};
use crate::raw::OperandReference;

impl<'a> ProgramIndex<'a> {
    /// Visits available original leaves and local read errors, including dynamic
    /// handles, call-site arguments and signature types. Callback errors stop
    /// delivery; a failed reference never becomes an empty successful match.
    pub fn visit_reference_leaves<'i, E: From<MethodError>>(
        &'i self,
        reference: OperandReference,
        mut visit: impl FnMut(&str, Result<ReferenceFacts<'i, 'a>, MethodError>) -> Result<(), E>,
    ) -> Result<(), E> {
        self.reference_leaves(reference, "", &mut visit)
    }

    fn prototype_leaves<'i, E>(
        &'i self,
        prototype: &Prototype<'i, 'a>,
        path: &str,
        visit: &mut impl FnMut(&str, Result<ReferenceFacts<'i, 'a>, MethodError>) -> Result<(), E>,
    ) -> Result<(), E> {
        visit(
            &format!("{path}return-type"),
            Ok(ReferenceFacts::Type(prototype.return_type)),
        )?;
        for (ordinal, &parameter) in prototype.parameter_descriptors().iter().enumerate() {
            visit(
                &format!("{path}parameter-type[{ordinal}]"),
                Ok(ReferenceFacts::Type(parameter)),
            )?;
        }
        Ok(())
    }

    fn reference_leaves<'i, E: From<MethodError>>(
        &'i self,
        reference: OperandReference,
        path: &str,
        visit: &mut impl FnMut(&str, Result<ReferenceFacts<'i, 'a>, MethodError>) -> Result<(), E>,
    ) -> Result<(), E> {
        match reference {
            OperandReference::MethodHandle(id) => match self.method_handle(id) {
                Ok(handle) => self.reference_leaves(
                    handle.member,
                    &format!("{path}method-handle[{}] → ", handle.kind),
                    visit,
                ),
                Err(error) => visit(path, Err(error.into())),
            },
            OperandReference::CallSite(id) => match self.call_site(id) {
                Err(error) => visit(&format!("{path}call-site"), Err(error)),
                Ok(site) => {
                    self.reference_leaves(
                        OperandReference::MethodHandle(site.handle_id),
                        &format!("{path}call-site.bootstrap → "),
                        visit,
                    )?;
                    visit(
                        &format!("{path}call-site.name"),
                        Ok(ReferenceFacts::String(site.name)),
                    )?;
                    self.prototype_leaves(&site.prototype, &format!("{path}call-site."), visit)?;
                    for (ordinal, step) in site.bootstrap.iter().enumerate() {
                        if let ValueStep::Value(value) = step {
                            let prefix = format!("{path}call-site.argument[{ordinal}] → ");
                            match value.kind {
                                EncodedKind::Reference(reference) => {
                                    self.reference_leaves(reference, &prefix, visit)?
                                }
                                EncodedKind::Annotation { type_id, .. } => visit(
                                    &prefix,
                                    self.reference_facts(OperandReference::Type(type_id))
                                        .map_err(Into::into),
                                )?,
                                _ => (),
                            }
                        }
                    }
                    Ok(())
                }
            },
            _ => match self.reference_facts(reference) {
                Err(error) => visit(path, Err(error.into())),
                Ok(facts) => {
                    match &facts {
                        ReferenceFacts::Method {
                            owner, prototype, ..
                        } => {
                            visit(
                                &format!("{path}owner-type"),
                                Ok(ReferenceFacts::Type(*owner)),
                            )?;
                            self.prototype_leaves(prototype, path, visit)?;
                        }
                        ReferenceFacts::Field {
                            owner, field_type, ..
                        } => {
                            visit(
                                &format!("{path}owner-type"),
                                Ok(ReferenceFacts::Type(*owner)),
                            )?;
                            visit(
                                &format!("{path}field-type"),
                                Ok(ReferenceFacts::Type(*field_type)),
                            )?;
                        }
                        ReferenceFacts::Proto(prototype) => {
                            self.prototype_leaves(prototype, path, visit)?
                        }
                        _ => (),
                    }
                    visit(path.trim_end_matches(" → "), Ok(facts))
                }
            },
        }
    }
}
