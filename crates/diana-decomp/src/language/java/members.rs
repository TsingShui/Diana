//! Class-wide Java member symbol allocation.
//!
//! DEX permits field and method sets that Java source cannot declare without
//! renaming (for example fields distinguished only by type, or methods
//! distinguished only by return type). Allocation is deterministic and every
//! body reference consumes the same symbol table as its declaration.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{ArgType, FieldReference, MethodDescriptor, MethodReference};

use super::{JavaIdentifier, JavaNameScope};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JavaFieldSymbol {
    pub owner: ArgType,
    pub name: JavaIdentifier,
    pub field_type: ArgType,
}

impl JavaFieldSymbol {
    pub fn new(owner: ArgType, name: JavaIdentifier, field_type: ArgType) -> Self {
        Self {
            owner,
            name,
            field_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JavaMethodSymbol {
    pub owner: ArgType,
    pub name: JavaIdentifier,
    pub descriptor: MethodDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JavaConstructorLayout {
    owner: ArgType,
    descriptor: MethodDescriptor,
    hidden_parameters: BTreeSet<usize>,
    enclosing_parameter: Option<usize>,
}

impl JavaConstructorLayout {
    pub fn new(
        owner: ArgType,
        descriptor: MethodDescriptor,
        hidden_parameters: impl IntoIterator<Item = usize>,
    ) -> Self {
        Self {
            owner,
            descriptor,
            hidden_parameters: hidden_parameters.into_iter().collect(),
            enclosing_parameter: None,
        }
    }

    pub fn with_enclosing_parameter(mut self, parameter: usize) -> Self {
        self.hidden_parameters.insert(parameter);
        self.enclosing_parameter = Some(parameter);
        self
    }

    pub fn matches(&self, reference: &MethodReference) -> bool {
        reference.name == "<init>"
            && self.owner == reference.owner
            && self.descriptor == reference.descriptor
    }

    fn key(&self) -> ConstructorKey {
        ConstructorKey {
            owner: self.owner.clone(),
            descriptor: self.descriptor.clone(),
        }
    }
}

impl JavaMethodSymbol {
    pub fn new(owner: ArgType, name: JavaIdentifier, descriptor: MethodDescriptor) -> Self {
        Self {
            owner,
            name,
            descriptor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MethodOverloadKey {
    owner: ArgType,
    name: JavaIdentifier,
    arity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ConstructorKey {
    owner: ArgType,
    descriptor: MethodDescriptor,
}

impl From<&MethodReference> for ConstructorKey {
    fn from(reference: &MethodReference) -> Self {
        Self {
            owner: reference.owner.clone(),
            descriptor: reference.descriptor.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct JavaMemberNames {
    fields: BTreeMap<JavaFieldSymbol, JavaIdentifier>,
    methods: Vec<(JavaMethodSymbol, JavaIdentifier)>,
    overloads: BTreeMap<MethodOverloadKey, BTreeSet<MethodDescriptor>>,
    constructors: BTreeMap<ConstructorKey, ConstructorSourceLayout>,
}

#[derive(Debug, Clone, Default)]
struct ConstructorSourceLayout {
    hidden_parameters: BTreeSet<usize>,
    enclosing_parameter: Option<usize>,
}

impl JavaMemberNames {
    pub fn allocate(
        fields: impl IntoIterator<Item = JavaFieldSymbol>,
        methods: impl IntoIterator<Item = JavaMethodSymbol>,
    ) -> Self {
        let fields = allocate_names(
            fields.into_iter().collect(),
            |field: &JavaFieldSymbol| (&field.owner, &field.name),
            |_, _| true,
        );
        let methods = allocate_names(
            methods.into_iter().collect(),
            |method: &JavaMethodSymbol| (&method.owner, &method.name),
            |a, b| a.descriptor.parameters == b.descriptor.parameters,
        );
        let mut overloads = BTreeMap::<MethodOverloadKey, BTreeSet<MethodDescriptor>>::new();
        for (method, name) in &methods {
            overloads
                .entry(MethodOverloadKey {
                    owner: method.owner.clone(),
                    name: name.clone(),
                    arity: method.descriptor.parameters.len(),
                })
                .or_default()
                .insert(method.descriptor.clone());
        }
        Self {
            fields: fields.into_iter().collect(),
            methods,
            overloads,
            constructors: BTreeMap::new(),
        }
    }

    pub fn with_constructor_layouts(
        mut self,
        layouts: impl IntoIterator<Item = JavaConstructorLayout>,
    ) -> Self {
        for layout in layouts {
            self.overloads
                .entry(MethodOverloadKey {
                    owner: layout.owner.clone(),
                    name: JavaIdentifier::from_dex("<init>"),
                    arity: layout.descriptor.parameters.len(),
                })
                .or_default()
                .insert(layout.descriptor.clone());
            self.constructors.insert(
                layout.key(),
                ConstructorSourceLayout {
                    hidden_parameters: layout.hidden_parameters,
                    enclosing_parameter: layout.enclosing_parameter,
                },
            );
        }
        self
    }

    pub fn with_hidden_fields(mut self, fields: impl IntoIterator<Item = JavaFieldSymbol>) -> Self {
        let mut scopes = BTreeMap::<ArgType, JavaNameScope>::new();
        for (field, name) in &self.fields {
            scopes
                .entry(field.owner.clone())
                .or_default()
                .reserve(name.clone());
        }
        for field in fields.into_iter().collect::<BTreeSet<_>>() {
            if self.fields.contains_key(&field) {
                continue;
            }
            let name = scopes
                .entry(field.owner.clone())
                .or_default()
                .claim(field.name.clone());
            self.fields.insert(field, name);
        }
        self
    }

    pub fn with_overloads(mut self, methods: impl IntoIterator<Item = MethodReference>) -> Self {
        for method in methods {
            let name = self.method(&method);
            self.overloads
                .entry(MethodOverloadKey {
                    owner: method.owner,
                    name,
                    arity: method.descriptor.parameters.len(),
                })
                .or_default()
                .insert(method.descriptor);
        }
        self
    }

    pub fn hidden_constructor_parameters(
        &self,
        reference: &MethodReference,
    ) -> Option<&BTreeSet<usize>> {
        self.constructors
            .get(&ConstructorKey::from(reference))
            .map(|layout| &layout.hidden_parameters)
    }

    pub fn enclosing_constructor_parameter(&self, reference: &MethodReference) -> Option<usize> {
        self.constructors
            .get(&ConstructorKey::from(reference))
            .and_then(|layout| layout.enclosing_parameter)
    }

    pub fn field(&self, reference: &FieldReference) -> JavaIdentifier {
        let key = JavaFieldSymbol::new(
            reference.owner.clone(),
            JavaIdentifier::from_dex(&reference.name),
            reference.field_type.clone(),
        );
        self.fields.get(&key).cloned().unwrap_or(key.name)
    }

    pub fn field_symbol(&self, symbol: &JavaFieldSymbol) -> JavaIdentifier {
        self.fields
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| symbol.name.clone())
    }

    pub fn field_names(&self, owner: &ArgType) -> BTreeSet<JavaIdentifier> {
        self.fields
            .iter()
            .filter(|(field, _)| &field.owner == owner)
            .map(|(_, name)| name.clone())
            .collect()
    }

    pub fn method(&self, reference: &MethodReference) -> JavaIdentifier {
        let name = JavaIdentifier::from_dex(&reference.name);
        let key = (&reference.owner, &name, &reference.descriptor);
        self.methods
            .binary_search_by(|(symbol, _)| {
                (&symbol.owner, &symbol.name, &symbol.descriptor).cmp(&key)
            })
            .map_or(name, |index| self.methods[index].1.clone())
    }

    pub fn null_argument_requires_cast(
        &self,
        reference: &MethodReference,
        parameter: usize,
    ) -> bool {
        let expected = match reference.descriptor.parameters.get(parameter) {
            Some(expected) if expected.is_reference() => expected,
            _ => return false,
        };
        let key = MethodOverloadKey {
            owner: reference.owner.clone(),
            name: self.method(reference),
            arity: reference.descriptor.parameters.len(),
        };
        self.overloads.get(&key).is_some_and(|overloads| {
            overloads.iter().any(|descriptor| {
                descriptor != &reference.descriptor
                    && descriptor
                        .parameters
                        .get(parameter)
                        .is_some_and(|candidate| candidate != expected)
            })
        })
    }

    pub fn overloads(&self, reference: &MethodReference) -> Option<&BTreeSet<MethodDescriptor>> {
        self.overloads.get(&MethodOverloadKey {
            owner: reference.owner.clone(),
            name: self.method(reference),
            arity: reference.descriptor.parameters.len(),
        })
    }

    pub fn method_symbol(&self, symbol: &JavaMethodSymbol) -> JavaIdentifier {
        self.methods
            .binary_search_by(|(key, _)| key.cmp(symbol))
            .map_or_else(
                |_| symbol.name.clone(),
                |index| self.methods[index].1.clone(),
            )
    }
}

/// DEX symbols are ordered by owner, source spelling and descriptor. All
/// declarations competing for one source name are adjacent in this order.
fn allocate_names<T: Ord>(
    members: BTreeSet<T>,
    names: fn(&T) -> (&ArgType, &JavaIdentifier),
    same_parameters: fn(&T, &T) -> bool,
) -> Vec<(T, JavaIdentifier)> {
    let same_group = |a: &T, b: &T| names(a) == names(b) && same_parameters(a, b);
    let mut scopes = BTreeMap::<ArgType, JavaNameScope>::new();
    if members
        .iter()
        .zip(members.iter().skip(1))
        .any(|(a, b)| same_group(a, b))
    {
        for member in &members {
            let (owner, name) = names(member);
            scopes
                .entry(owner.clone())
                .or_default()
                .reserve(name.clone());
        }
    }
    let mut result = Vec::with_capacity(members.len());
    let mut members = members.into_iter().peekable();
    let mut occurrence = 1;
    while let Some(member) = members.next() {
        let (owner, name) = names(&member);
        let allocated = if occurrence == 1 {
            name.clone()
        } else {
            scopes
                .entry(owner.clone())
                .or_default()
                .claim(JavaIdentifier::from_dex(&format!("{name}${occurrence}")))
        };
        occurrence = if members.peek().is_some_and(|next| same_group(&member, next)) {
            occurrence + 1
        } else {
            1
        };
        result.push((member, allocated));
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::ir::ArgType;

    use super::{JavaFieldSymbol, JavaIdentifier, JavaMemberNames};

    #[test]
    fn hidden_field_does_not_reuse_a_declared_field_name() {
        let owner = ArgType::object("example/Owner");
        let declared = JavaFieldSymbol::new(
            owner.clone(),
            JavaIdentifier::from_dex("value"),
            ArgType::BOOLEAN,
        );
        let hidden = JavaFieldSymbol::new(
            owner,
            JavaIdentifier::from_dex("value"),
            ArgType::object("example/Outer"),
        );
        let names =
            JavaMemberNames::allocate([declared.clone()], []).with_hidden_fields([hidden.clone()]);

        assert_eq!(
            names.field_symbol(&declared),
            JavaIdentifier::from_dex("value")
        );
        assert_ne!(names.field_symbol(&hidden), names.field_symbol(&declared));
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/member_name_allocation.rs"]
mod allocation_tests;
