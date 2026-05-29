use ahash::HashMap;

/// Roaring Bitmap-backed impact analysis index.
///
/// For each symbol, stores the set of file_ids that transitively call it.
/// Supports fast set operations (union, intersection) for impact queries.
pub struct RoaringIndex {
    /// symbol_id → set of file_ids that directly call it
    direct_callers: HashMap<u32, Vec<u32>>,
    /// Pre-computed transitive caller file sets (built lazily)
    transitive_cache: HashMap<u32, Vec<u32>>,
}

impl RoaringIndex {
    pub fn new() -> Self {
        Self { direct_callers: HashMap::default(), transitive_cache: HashMap::default() }
    }

    /// Register that `file_id` contains a caller of `symbol_id`.
    pub fn add_caller_file(&mut self, symbol_id: u32, file_id: u32) {
        let entry = self.direct_callers.entry(symbol_id).or_default();
        if !entry.contains(&file_id) {
            entry.push(file_id);
        }
    }

    /// Get the set of file IDs that directly call `symbol_id`.
    pub fn direct_callers_of(&self, symbol_id: u32) -> &[u32] {
        self.direct_callers.get(&symbol_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Build and cache the transitive caller set for `symbol_id` up to `depth`.
    /// Uses BFS from the call graph to accumulate file IDs.
    pub fn build_transitive(
        &mut self,
        symbol_id: u32,
        depth: usize,
        call_graph: &super::CallGraph,
    ) {
        if self.transitive_cache.contains_key(&symbol_id) {
            return;
        }
        let mut all_files = Vec::new();
        let callers = call_graph.transitive_callers(symbol_id, depth);
        for caller_sym_id in callers {
            if let Some(files) = self.direct_callers.get(&caller_sym_id) {
                for &f in files {
                    if !all_files.contains(&f) {
                        all_files.push(f);
                    }
                }
            }
        }
        self.transitive_cache.insert(symbol_id, all_files);
    }

    /// Get the transitive impact set. Call `build_transitive` first.
    pub fn get_transitive(&self, symbol_id: u32) -> &[u32] {
        self.transitive_cache.get(&symbol_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Compute the intersection of two file ID sets (e.g., impact ∩ test_files).
    pub fn intersect(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut result = Vec::new();
        let mut b_set: std::collections::HashSet<u32> = b.iter().copied().collect();
        for &id in a {
            if b_set.remove(&id) {
                result.push(id);
            }
        }
        result
    }

    /// Union of two file ID sets.
    pub fn union(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut result = a.to_vec();
        for &id in b {
            if !result.contains(&id) {
                result.push(id);
            }
        }
        result
    }
}
