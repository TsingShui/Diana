#[test]
fn tied_left_offsets_prefer_the_earliest_reachable_right_across_signatures() {
    let mut cfg = CFG::new("tied-fragments");
    for id in 0u32..10 {
        cfg.add_block(Block::new(id));
    }
    cfg.add_edge(BlockId::new(0), BlockId::new(3), EdgeKind::Normal);
    cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
    let predecessors = cfg.normal_predecessor_snapshot();
    let coalescing = ExceptionScopeCoalescing::new(&cfg, &predecessors);
    let mut regions = vec![
        try_region(40, 10, 15, &[0], vec![catch_handler(8, &[8])]),
        try_region(41, 10, 16, &[1], vec![catch_handler(9, &[9])]),
        try_region(50, 16, 18, &[2], vec![catch_handler(9, &[9])]),
        try_region(51, 20, 22, &[3], vec![catch_handler(8, &[8])]),
    ];
    for _ in 0..regions.len() {
        assert_eq!(
            coalescing.connected_fragments(&regions, &RegionIndex::of(&regions)),
            Some((41, 50))
        );
        regions.rotate_left(1);
    }
}

#[test]
fn isolated_early_fragments_allow_a_later_overlapping_pair() {
    let mut cfg = CFG::new("overlapping-fragments");
    for id in 0u32..9 {
        cfg.add_block(Block::new(id));
    }
    let predecessors = cfg.normal_predecessor_snapshot();
    let coalescing = ExceptionScopeCoalescing::new(&cfg, &predecessors);
    let regions = vec![
        try_region(6, 1, 4, &[0], vec![catch_handler(8, &[8])]),
        try_region(4, 10, 14, &[1, 2], vec![catch_handler(8, &[8])]),
        try_region(3, 12, 16, &[2, 3], vec![catch_handler(8, &[8])]),
    ];
    assert_eq!(
        coalescing.connected_fragments(&regions, &RegionIndex::of(&regions)),
        Some((4, 3))
    );
}
