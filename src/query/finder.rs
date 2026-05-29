use crate::index::{Index, SymbolKind};
use anyhow::Result;

/// Symbol search and context-building queries.
pub struct Finder<'a> {
    index: &'a Index,
    kind_filter: Option<SymbolKind>,
    file_filter: Option<String>,
}

/// A search result with symbol info and location.
#[derive(Debug, Clone)]
pub struct FindResult {
    pub name: String,
    pub kind: String,
    pub visibility: String,
    pub file: String,
    pub line: u32,
}

/// Context built for a task.
#[derive(Debug, Clone)]
pub struct TaskContext {
    pub task: String,
    pub entry_points: Vec<FindResult>,
    pub related_symbols: Vec<FindResult>,
    pub call_edges: Vec<(String, String)>,
}

impl<'a> Finder<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index, kind_filter: None, file_filter: None }
    }

    pub fn kind(mut self, kind: Option<&str>) -> Self {
        self.kind_filter = kind.map(SymbolKind::from_str);
        self
    }

    pub fn file_filter(mut self, pattern: Option<&str>) -> Self {
        self.file_filter = pattern.map(|s| s.to_string());
        self
    }

    /// Find all symbols matching the given name.
    /// Uses trie exact lookup first (O(1)), falls back to inverted index.
    pub fn find(&self, name: &str) -> Result<Vec<FindResult>> {
        // Use trie for O(1) exact lookup optimization
        let name_id = if let Some(id) = self.index.trie.exact_lookup(name) {
            Some(id)
        } else {
            // Prefix search via inverted index for partial matches
            let prefix_hits = self.index.inverted.prefix_search(name);
            if !prefix_hits.is_empty() {
                prefix_hits.first().copied()
            } else {
                None
            }
        };

        // Collect symbol IDs from inverted index
        let symbol_ids = if let Some(_nid) = name_id {
            // Use kind-filtered lookup if kind filter is set
            if let Some(ref kf) = self.kind_filter {
                let kind_ids = self.index.inverted.find_by_kind(kf.as_str());
                let name_ids = self.index.inverted.find_by_name(name);
                // Intersection: names matching both name AND kind
                name_ids.into_iter().filter(|id| kind_ids.contains(id)).collect()
            } else {
                self.index.inverted.find_by_name(name)
            }
        } else {
            self.index.inverted.find_by_name(name)
        };

        let mut results = Vec::new();
        for &sym_id in &symbol_ids {
            if sym_id as usize >= self.index.symbols.len() {
                continue;
            }
            let sym = &self.index.symbols[sym_id as usize];

            // Use get_name for efficient string lookup
            let sym_name = self.index.inverted.get_name(sym.name_id).unwrap_or(name);

            if let Some(ref kf) = self.kind_filter {
                if sym.kind != *kf {
                    continue;
                }
            }
            if let Some(ref ff) = self.file_filter {
                let file_path = self.get_string(sym.file_id);
                if !file_path.contains(ff) {
                    continue;
                }
            }
            // Apply file filter via inverted index file lookup
            if let Some(ref ff) = self.file_filter {
                let file_matches = self.index.inverted.find_by_file(sym.file_id);
                if file_matches.is_empty() {
                    continue;
                }
                let file_path = self.get_string(sym.file_id);
                if !file_path.contains(ff) {
                    continue;
                }
            }

            results.push(FindResult {
                name: sym_name.to_string(),
                kind: sym.kind.as_str().to_string(),
                visibility: sym.visibility.as_str().to_string(),
                file: self.get_string(sym.file_id).to_string(),
                line: sym.line,
            });
        }
        Ok(results)
    }

    /// Find all callers of a symbol (reverse call graph lookup).
    pub fn find_callers(
        &self, name: &str, depth: usize, limit: usize, test_only: bool,
    ) -> Result<Vec<FindResult>> {
        let symbol_ids = self.index.inverted.find_by_name(name);
        let mut results = Vec::new();

        for &sym_id in &symbol_ids {
            let callers = self.index.call_graph.callers(sym_id);
            for &caller_id in callers {
                if results.len() >= limit {
                    break;
                }
                if caller_id as usize >= self.index.symbols.len() {
                    continue;
                }
                let sym = &self.index.symbols[caller_id as usize];
                let file = self.get_string(sym.file_id);
                if test_only && !file.contains("test") && !file.contains("spec") {
                    continue;
                }
                results.push(FindResult {
                    name: self.get_string(sym.name_id).to_string(),
                    kind: sym.kind.as_str().to_string(),
                    visibility: sym.visibility.as_str().to_string(),
                    file: file.to_string(),
                    line: sym.line,
                });
            }
        }

        // Transitive callers if depth > 1
        if depth > 1 && results.len() < limit {
            for &sym_id in &symbol_ids {
                let transitive = self.index.call_graph.transitive_callers(sym_id, depth);
                for &caller_id in &transitive {
                    if results.len() >= limit {
                        break;
                    }
                    if caller_id as usize >= self.index.symbols.len() {
                        continue;
                    }
                    let sym = &self.index.symbols[caller_id as usize];
                    let file = self.get_string(sym.file_id);
                    if test_only && !file.contains("test") && !file.contains("spec") {
                        continue;
                    }
                    results.push(FindResult {
                        name: self.get_string(sym.name_id).to_string(),
                        kind: sym.kind.as_str().to_string(),
                        visibility: sym.visibility.as_str().to_string(),
                        file: file.to_string(),
                        line: sym.line,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Find all callees of a symbol (forward call graph lookup).
    pub fn find_callees(
        &self, name: &str, _depth: usize, limit: usize, include_external: bool,
    ) -> Result<Vec<FindResult>> {
        let symbol_ids = self.index.inverted.find_by_name(name);
        let mut results = Vec::new();

        for &sym_id in &symbol_ids {
            let callees = self.index.call_graph.callees(sym_id);
            for &callee_id in callees {
                if results.len() >= limit {
                    break;
                }
                if callee_id as usize >= self.index.symbols.len() {
                    if include_external {
                        results.push(FindResult {
                            name: format!("<external:{}>", callee_id),
                            kind: "unknown".to_string(),
                            visibility: "".to_string(),
                            file: "<external>".to_string(),
                            line: 0,
                        });
                    }
                    continue;
                }
                let sym = &self.index.symbols[callee_id as usize];
                results.push(FindResult {
                    name: self.get_string(sym.name_id).to_string(),
                    kind: sym.kind.as_str().to_string(),
                    visibility: sym.visibility.as_str().to_string(),
                    file: self.get_string(sym.file_id).to_string(),
                    line: sym.line,
                });
            }
        }

        Ok(results)
    }

    /// Build a task-oriented context map.
    pub fn build_context(
        &self, task: &str, max_nodes: usize, _include_code: bool,
    ) -> Result<TaskContext> {
        let keywords: Vec<&str> = task
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 3)
            .collect();

        let mut entry_points = Vec::new();
        let mut related_symbols = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for kw in &keywords {
            let results = self.find(kw)?;
            for r in results {
                if seen.len() >= max_nodes {
                    break;
                }
                let key = format!("{}:{}:{}", r.file, r.line, r.name);
                if !seen.contains(&key) {
                    seen.insert(key);
                    if entry_points.len() < 5 {
                        entry_points.push(r.clone());
                    } else {
                        related_symbols.push(r);
                    }
                }
            }
        }

        let mut call_edges = Vec::new();
        for ep in &entry_points {
            if let Some(name_id) = self.index.inverted.get_name_id(&ep.name) {
                let callees = self.index.call_graph.callees(name_id);
                for &callee_id in callees {
                    if callee_id as usize >= self.index.symbols.len() {
                        continue;
                    }
                    let callee_name = self.get_string(self.index.symbols[callee_id as usize].name_id);
                    call_edges.push((ep.name.clone(), callee_name.to_string()));
                }
            }
        }

        Ok(TaskContext { task: task.to_string(), entry_points, related_symbols, call_edges })
    }

    fn get_string(&self, id: u32) -> &str {
        self.index.strings.get(id as usize).map(|s| s.as_str()).unwrap_or("<unknown>")
    }
}
