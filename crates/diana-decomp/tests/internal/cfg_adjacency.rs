use super::*;
use crate::ir::CFG;
use std::collections::BTreeSet;

#[test]
fn block_rows_keep_sparse_targets_and_parallel_kinds_through_topology_edits() {
    let mut cfg = CFG::new("edges");
    let mut edges = BTreeMap::<BlockId, Vec<(BlockId, EdgeKind)>>::new();
    let ids = [13, 4, 8, 3, u32::MAX, 19, 0, 1, 9, 12, 15, 2, 70, 6, 16, 18].map(BlockId);
    let mut seed = 193_u32;
    for step in 0..800 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let from = ids[(seed >> 16) as usize % ids.len()];
        let to = ids[(seed >> 8) as usize % ids.len()];
        let kind = [
            EdgeKind::Normal,
            EdgeKind::True,
            EdgeKind::False,
            EdgeKind::Exception,
            EdgeKind::SwitchCase(-8),
        ][seed as usize % 5];
        match step % 19 {
            3 => {
                cfg.remove_block_from_graph(from);
                edges.remove(&from);
                for row in edges.values_mut() {
                    row.retain(|&(target, _)| target != from);
                }
            }
            7 => {
                cfg.remove_normal_edges_from(from);
                if let Some(row) = edges.get_mut(&from) {
                    row.retain(|&(_, k)| k == EdgeKind::Exception);
                }
            }
            10 => {
                cfg.remove_edge_with_kind(from, to, kind);
                if let Some(row) = edges.get_mut(&from) {
                    row.retain(|&e| e != (to, kind));
                }
            }
            14 if step % 3 == 0 => {
                cfg.remove_exception_edges();
                for row in edges.values_mut() {
                    row.retain(|&(_, k)| k != EdgeKind::Exception);
                }
            }
            _ => {
                cfg.add_edge(from, to, kind);
                let row = edges.entry(from).or_default();
                if !row.contains(&(to, kind)) {
                    row.push((to, kind));
                }
            }
        }
        assert_eq!(cfg.num_graph_nodes(), edges.len());
        assert_eq!(
            cfg.graph_node_ids(),
            edges.keys().copied().collect::<Vec<_>>()
        );
        for id in ids {
            let expected = edges.get(&id).map(Vec::as_slice).unwrap_or_default();
            assert_eq!(cfg.successors_with_kind(id), expected);
            let incoming: BTreeSet<_> = edges
                .iter()
                .flat_map(|(&s, row)| {
                    row.iter()
                        .filter(move |(target, _)| *target == id)
                        .map(move |&(_, k)| (s, k))
                })
                .collect();
            assert!(cfg.predecessors_with_kind(id).iter().copied().eq(incoming));
        }
    }
}
