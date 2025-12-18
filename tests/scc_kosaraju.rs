use microsoft_edge_bookmark_sorter_flattener::domain::graph::Graph;
use microsoft_edge_bookmark_sorter_flattener::domain::traits::SccDetector;
use microsoft_edge_bookmark_sorter_flattener::infrastructure::scc_kosaraju::KosarajuSccDetector;

#[test]
fn kosaraju_detects_self_loop_as_cyclic() {
    // 0 -> 0
    let mut g = Graph::new(1);
    g.edges[0].push(0);

    let det = KosarajuSccDetector;
    let scc = det.compute_scc(&g);

    assert_eq!(scc.components.len(), 1);
    assert_eq!(scc.components[0], vec![0]);
    assert_eq!(scc.cyclic_component, vec![true]);
}

#[test]
fn kosaraju_detects_two_node_cycle() {
    // 0 <-> 1
    let mut g = Graph::new(2);
    g.edges[0].push(1);
    g.edges[1].push(0);

    let det = KosarajuSccDetector;
    let scc = det.compute_scc(&g);

    assert_eq!(scc.components.len(), 1);
    assert_eq!(scc.components[0], vec![0, 1]);
    assert_eq!(scc.cyclic_component, vec![true]);
}

#[test]
fn kosaraju_produces_stable_partition_for_dag() {
    // 0 -> 1 -> 2
    let mut g = Graph::new(3);
    g.edges[0].push(1);
    g.edges[1].push(2);

    let det = KosarajuSccDetector;
    let scc = det.compute_scc(&g);

    assert_eq!(scc.components.len(), 3);
    // Each node should be its own SCC.
    for comp in scc.components.iter() {
        assert_eq!(comp.len(), 1);
    }
    assert!(scc.cyclic_component.iter().all(|&b| !b));
}
