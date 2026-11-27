//! The exact original method identities at one caller depth.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use diana_decomp::input::jvm::{ClassFile, Member, MemberReference};

use super::{QueryError, Relevance};
use crate::analysis::{filter::best_first, input::InputError};

#[derive(Eq, Hash, PartialEq)]
pub(crate) struct Caller {
    owner: Vec<u8>,
    name: Vec<u8>,
    descriptor: Vec<u8>,
}

impl Caller {
    pub(super) fn read(class: &ClassFile<'_>, member: Member<'_>) -> Result<Self, QueryError> {
        Ok(Self {
            owner: copy(class.name()?.bytes())?,
            name: copy(member.name(class)?.bytes())?,
            descriptor: copy(member.descriptor(class)?.bytes())?,
        })
    }
}

fn copy(bytes: &[u8]) -> Result<Vec<u8>, QueryError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(bytes.len())
        .map_err(InputError::Allocation)?;
    result.extend_from_slice(bytes);
    Ok(result)
}

type Signatures = HashMap<Vec<u8>, Option<Relevance>>;
type MethodNames = HashMap<Vec<u8>, Signatures>;

#[derive(Default)]
pub(super) struct Frontier {
    owners: HashMap<Vec<u8>, MethodNames>,
}

impl Frontier {
    pub(super) fn matches(&self, member: MemberReference<'_>) -> Option<Option<Relevance>> {
        self.owners
            .get(member.owner.bytes())?
            .get(member.name.bytes())?
            .get(member.descriptor.bytes())
            .copied()
    }
}

#[derive(Default)]
pub(super) struct Callers {
    seen: HashSet<Arc<Caller>>,
    next: HashMap<Arc<Caller>, Option<Relevance>>,
}

impl Callers {
    pub(super) fn reach(
        &mut self,
        caller: Arc<Caller>,
        relevance: Option<Relevance>,
    ) -> Result<(), QueryError> {
        if self.seen.contains(&caller) {
            return Ok(());
        }
        self.next.try_reserve(1).map_err(InputError::Allocation)?;
        let best = self.next.entry(caller).or_insert(relevance);
        if best_first(relevance, *best).is_lt() {
            *best = relevance;
        }
        Ok(())
    }

    pub(super) fn advance(&mut self) -> Result<Option<Frontier>, QueryError> {
        if self.next.is_empty() {
            return Ok(None);
        }
        self.seen
            .try_reserve(self.next.len())
            .map_err(InputError::Allocation)?;
        let mut frontier = Frontier::default();
        for (caller, relevance) in self.next.drain() {
            frontier
                .owners
                .try_reserve(1)
                .map_err(InputError::Allocation)?;
            let names = frontier.owners.entry(copy(&caller.owner)?).or_default();
            names.try_reserve(1).map_err(InputError::Allocation)?;
            let methods = names.entry(copy(&caller.name)?).or_default();
            methods.try_reserve(1).map_err(InputError::Allocation)?;
            methods.insert(copy(&caller.descriptor)?, relevance);
            self.seen.insert(caller);
        }
        Ok(Some(frontier))
    }
}
