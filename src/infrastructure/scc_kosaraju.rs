use crate::domain::graph::{Graph, SccResult};
use crate::domain::traits::SccDetector;

pub struct KosarajuSccDetector;

impl SccDetector for KosarajuSccDetector {
    fn compute_scc(&self, graph: &Graph) -> SccResult {
        kosaraju_scc(graph)
    }
}

fn kosaraju_scc(graph: &Graph) -> SccResult {
    let n = graph.node_count();
    let mut rev: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (u, outs) in graph.edges.iter().enumerate() {
        for &v in outs {
            rev[v].push(u);
        }
    }

    for outs in rev.iter_mut() {
        outs.sort_unstable();
        outs.dedup();
    }

    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut seen = vec![false; n];

    for start in 0..n {
        if seen[start] {
            continue;
        }
        iterative_finish_order(start, &graph.edges, &mut seen, &mut order);
    }

    let mut component_of = vec![usize::MAX; n];
    let mut components: Vec<Vec<usize>> = Vec::new();

    for &v in order.iter().rev() {
        if component_of[v] != usize::MAX {
            continue;
        }

        let mut stack = vec![v];
        component_of[v] = components.len();
        let mut comp = Vec::new();

        while let Some(x) = stack.pop() {
            comp.push(x);
            for &p in rev[x].iter() {
                if component_of[p] == usize::MAX {
                    component_of[p] = components.len();
                    stack.push(p);
                }
            }
        }

        comp.sort_unstable();
        components.push(comp);
    }

    let mut cyclic_component = vec![false; components.len()];

    for (cid, comp) in components.iter().enumerate() {
        if comp.len() > 1 {
            cyclic_component[cid] = true;
            continue;
        }
        let only = comp[0];
        if graph.edges[only].iter().any(|&v| v == only) {
            cyclic_component[cid] = true;
        }
    }

    SccResult {
        component_of,
        components,
        cyclic_component,
    }
}

fn iterative_finish_order(
    start: usize,
    edges: &[Vec<usize>],
    seen: &mut [bool],
    order: &mut Vec<usize>,
) {
    let mut stack: Vec<(usize, usize)> = Vec::new();
    stack.push((start, 0));

    while let Some((v, next_i)) = stack.pop() {
        if !seen[v] {
            seen[v] = true;
        }

        if next_i < edges[v].len() {
            let to = edges[v][next_i];
            stack.push((v, next_i + 1));
            if !seen[to] {
                stack.push((to, 0));
            }
            continue;
        }

        order.push(v);
    }
}
