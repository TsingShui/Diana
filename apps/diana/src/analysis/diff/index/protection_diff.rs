//! DEX protection applies to operations inside half-open ranges. Mapping two
//! interval endpoints is insufficient when aligned blocks change physical order.
use super::Body;
use super::compare::{clause, region};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Write;

pub(in crate::analysis) struct ProtectionChange {
    pub old: Option<(Option<u32>, String)>,
    pub new: Option<(Option<u32>, String)>,
}

struct Range<'a> {
    start: usize,
    end: usize,
    clauses: &'a [u8],
    first_report: Cell<Option<u32>>,
}

struct Table<'a> {
    body: &'a Body,
    ranges: Vec<Range<'a>>,
}

impl<'a> Table<'a> {
    fn new(body: &'a Body) -> Self {
        let mut input = body.protection.as_slice();
        let mut ranges = Vec::new();
        while let Some((start, end, count)) = region(&mut input) {
            let head = input;
            for _ in 0..count {
                // Only the private checked producer writes this encoding.
                if clause(&mut input).is_none() {
                    break;
                }
            }
            ranges.push(Range {
                start: start as usize,
                end: end as usize,
                clauses: &head[..head.len() - input.len()],
                first_report: Cell::new(None),
            });
        }
        Self { body, ranges }
    }

    fn covering(&self, op: usize) -> Option<usize> {
        let r = self.ranges.partition_point(|r| r.end <= op);
        self.ranges
            .get(r)
            .filter(|range| range.start <= op)
            .map(|_| r)
    }

    fn site(&self, i: usize, r: Option<usize>) -> (Option<u32>, String) {
        let Some(region) = r.map(|r| &self.ranges[r]) else {
            return (Some(self.body.code[i].pc), "unprotected".to_owned());
        };
        let pc = |ord: usize| {
            self.body
                .code
                .get(ord)
                .map_or(self.body.code_units, |op| op.pc)
        };
        let mut text = format!("try [0x{:x},0x{:x})", pc(region.start), pc(region.end));
        // A large catch list can serve every instruction in the method. Emit
        // its names once and retain the exact site/range on subsequent rows.
        if let Some(first) = region.first_report.get() {
            let _ = write!(text, " (ordered handlers shown at @0x{first:x})");
            return (Some(self.body.code[i].pc), text);
        }
        region.first_report.set(Some(self.body.code[i].pc));
        let mut input = region.clauses;
        while let Some((handler, tag, name)) = clause(&mut input) {
            let ty = if tag == b'*' {
                "catch-all"
            } else {
                std::str::from_utf8(name).unwrap_or("<unknown>")
            };
            let _ = write!(text, " {ty} -> @0x{:x}", pc(handler as usize));
        }
        (Some(self.body.code[i].pc), text)
    }
}

fn same_chain(
    mut a: &[u8],
    mut b: &[u8],
    symbols: &HashMap<String, String>,
    targets: &[Option<usize>],
) -> bool {
    while !a.is_empty() && !b.is_empty() {
        let (Some((ap, at, av)), Some((bp, bt, bv))) = (clause(&mut a), clause(&mut b)) else {
            return false;
        };
        let name = std::str::from_utf8(av)
            .ok()
            .and_then(|s| symbols.get(s))
            .map_or(av, String::as_bytes);
        if at != bt
            || name != bv
            || targets.get(ap as usize).copied().flatten() != Some(bp as usize)
        {
            return false;
        }
    }
    a.is_empty() && b.is_empty()
}

impl Body {
    pub fn protection_changes(
        &self,
        other: &Body,
        symbols: &HashMap<String, String>,
        aligned: &[(usize, usize)],
        targets: &[Option<usize>],
    ) -> Vec<ProtectionChange> {
        if self.protection.is_empty() && other.protection.is_empty() {
            return Vec::new();
        }
        let old = Table::new(self);
        let new = Table::new(other);
        let mut old_pair = vec![None; old.ranges.len()];
        let mut new_pair = vec![None; new.ranges.len()];
        let mut paired_new = vec![false; other.code.len()];
        let mut chains = HashMap::new();
        let mut sites = Vec::new();
        let mut different_ranges = false;
        for &(i, j) in aligned {
            paired_new[j] = true;
            let a = old.covering(i);
            let b = new.covering(j);
            let same = match (a, b) {
                (None, None) => true,
                (Some(left), Some(right)) => {
                    // Range order is physical. Two distinct ranges must not
                    // collapse into one merely because their clauses agree.
                    let previous_old = old_pair[left].replace(right);
                    let previous_new = new_pair[right].replace(left);
                    different_ranges |= previous_old.is_some_and(|r| r != right)
                        || previous_new.is_some_and(|l| l != left);
                    *chains.entry((left, right)).or_insert_with(|| {
                        same_chain(
                            old.ranges[left].clauses,
                            new.ranges[right].clauses,
                            symbols,
                            targets,
                        )
                    })
                }
                _ => false,
            };
            if !same {
                different_ranges = true;
                if self.code[i].flow.may_throw() || other.code[j].flow.may_throw() {
                    sites.push(ProtectionChange {
                        old: Some(old.site(i, a)),
                        new: Some(new.site(j, b)),
                    });
                }
            }
        }
        for (r, range) in old.ranges.iter().enumerate() {
            for (i, target) in (range.start..range.end).zip(&targets[range.start..range.end]) {
                if target.is_none() {
                    different_ranges = true;
                    if self.code[i].flow.may_throw() {
                        sites.push(ProtectionChange {
                            old: Some(old.site(i, Some(r))),
                            new: None,
                        });
                    }
                }
            }
        }
        for (r, range) in new.ranges.iter().enumerate() {
            for (j, &matched) in (range.start..range.end).zip(&paired_new[range.start..range.end]) {
                if !matched {
                    different_ranges = true;
                    if other.code[j].flow.may_throw() {
                        sites.push(ProtectionChange {
                            old: None,
                            new: Some(new.site(j, Some(r))),
                        });
                    }
                }
            }
        }
        if different_ranges {
            let a = self.describe_protection();
            let b = other.describe_protection();
            if sites.is_empty() || a != b {
                sites.insert(
                    0,
                    ProtectionChange {
                        old: Some((
                            None,
                            if a.is_empty() {
                                "unprotected".into()
                            } else {
                                a
                            },
                        )),
                        new: Some((
                            None,
                            if b.is_empty() {
                                "unprotected".into()
                            } else {
                                b
                            },
                        )),
                    },
                );
            }
        }
        sites
    }
}
