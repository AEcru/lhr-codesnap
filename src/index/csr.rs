/// Compressed Sparse Row (CSR) call graph — forward + reverse edges.
///
/// Forward: caller → callees  (for callees() queries)
/// Reverse: callee → callers (for callers() queries)
///
/// Both stored as contiguous integer arrays for cache-friendly traversal.
/// Multi-level design:
/// - Level 2: module-to-module calls (coarse, fast traversal)
/// - Level 1: file-to-file calls (medium)
/// - Level 0: symbol-to-symbol calls (fine-grained)
pub struct CallGraph {
    /// Forward edges: offsets[i] to offsets[i+1]-1 in edges[]
    forward_offsets: Vec<u32>,
    forward_edges: Vec<u32>,

    /// Reverse edges: for callers() lookups
    reverse_offsets: Vec<u32>,
    reverse_edges: Vec<u32>,

    /// Total number of nodes
    node_count: u32,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            forward_offsets: vec![0],
            forward_edges: Vec::new(),
            reverse_offsets: vec![0],
            reverse_edges: Vec::new(),
            node_count: 0,
        }
    }

    /// Register a new node, returning its ID.
    pub fn add_node(&mut self) -> u32 {
        let id = self.node_count;
        self.node_count += 1;
        self.forward_offsets.push(self.forward_edges.len() as u32);
        self.reverse_offsets.push(self.reverse_edges.len() as u32);
        id
    }

    /// Add a call edge: `caller` calls `callee`.
    pub fn add_edge(&mut self, caller: u32, callee: u32) {
        let max = caller.max(callee);
        while self.node_count <= max {
            self.add_node();
        }
        // Forward: caller → callee
        self.forward_edges.push(callee);
        // Update forward offset for next node
        let call_idx = caller as usize;
        while self.forward_offsets.len() <= call_idx + 1 {
            self.forward_offsets.push(self.forward_edges.len() as u32);
        }
        // Reverse: callee → caller
        self.reverse_edges.push(caller);
        let callee_idx = callee as usize;
        while self.reverse_offsets.len() <= callee_idx + 1 {
            self.reverse_offsets.push(self.reverse_edges.len() as u32);
        }
    }

    /// Finalize offsets after all edges are added.
    pub fn finalize(&mut self) {
        self.forward_offsets.push(self.forward_edges.len() as u32);
        self.reverse_offsets.push(self.reverse_edges.len() as u32);
    }

    /// Get callees of `node` (what this node calls).
    pub fn callees(&self, node: u32) -> &[u32] {
        let idx = node as usize;
        if idx + 1 >= self.forward_offsets.len() {
            return &[];
        }
        let start = self.forward_offsets[idx] as usize;
        let end = self.forward_offsets[idx + 1] as usize;
        if start >= self.forward_edges.len() {
            return &[];
        }
        &self.forward_edges[start..end.min(self.forward_edges.len())]
    }

    /// Get callers of `node` (who calls this node).
    pub fn callers(&self, node: u32) -> &[u32] {
        let idx = node as usize;
        if idx + 1 >= self.reverse_offsets.len() {
            return &[];
        }
        let start = self.reverse_offsets[idx] as usize;
        let end = self.reverse_offsets[idx + 1] as usize;
        if start >= self.reverse_edges.len() {
            return &[];
        }
        &self.reverse_edges[start..end.min(self.reverse_edges.len())]
    }

    /// BFS trace: find shortest path from `from` to `to`.
    /// Returns the path as a Vec of node IDs including both endpoints, or empty if no path.
    pub fn bfs_trace(&self, from: u32, to: u32, max_depth: usize) -> Vec<u32> {
        if from == to {
            return vec![from];
        }
        // Guard: no nodes in graph, or IDs out of range
        if self.node_count == 0 || from >= self.node_count || to >= self.node_count {
            return Vec::new();
        }
        let mut visited = vec![false; self.node_count as usize];
        let mut queue = std::collections::VecDeque::new();
        let mut parent: Vec<Option<u32>> = vec![None; self.node_count as usize];

        queue.push_back((from, 0));
        visited[from as usize] = true;

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for &callee in self.callees(current) {
                if !visited[callee as usize] {
                    visited[callee as usize] = true;
                    parent[callee as usize] = Some(current);
                    if callee == to {
                        return Self::reconstruct_path(&parent, from, to);
                    }
                    queue.push_back((callee, depth + 1));
                }
            }
        }
        Vec::new()
    }

    /// Transitive callers up to `max_depth` levels.
    pub fn transitive_callers(&self, node: u32, max_depth: usize) -> Vec<u32> {
        // Guard: no nodes in graph, or node ID out of range
        if self.node_count == 0 || node >= self.node_count {
            return Vec::new();
        }
        let mut results = Vec::new();
        let mut visited = vec![false; self.node_count as usize];
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((node, 0));
        visited[node as usize] = true;

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }
            if depth > 0 {
                results.push(current);
            }
            for &caller in self.callers(current) {
                if !visited[caller as usize] {
                    visited[caller as usize] = true;
                    queue.push_back((caller, depth + 1));
                }
            }
        }
        results
    }

    fn reconstruct_path(parent: &[Option<u32>], from: u32, to: u32) -> Vec<u32> {
        let mut path = vec![to];
        let mut current = to;
        while let Some(p) = parent[current as usize] {
            path.push(p);
            if p == from {
                break;
            }
            current = p;
        }
        path.reverse();
        path
    }
}
