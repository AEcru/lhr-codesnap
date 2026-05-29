use crate::index::Index;
use anyhow::{Result, bail};

/// Impact analysis — determine what code is affected by changing a symbol.
pub struct Impacter<'a> {
    index: &'a Index,
}

#[derive(Debug, Clone)]
pub struct ImpactReport {
    pub symbol: String,
    pub direct_files: Vec<ImpactedFile>,
    pub transitive_files: Vec<ImpactedFile>,
    pub affected_tests: Vec<ImpactedFile>,
    pub total_files: usize,
}

#[derive(Debug, Clone)]
pub struct ImpactedFile {
    pub path: String,
    pub symbols: Vec<String>,
    pub is_test: bool,
}

impl<'a> Impacter<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index }
    }

    /// Analyze the impact radius of changing `symbol_name`.
    pub fn analyze(
        &self, symbol_name: &str, depth: usize, test_only: bool,
    ) -> Result<ImpactReport> {
        let symbol_ids = self.index.inverted.find_by_name(symbol_name);
        if symbol_ids.is_empty() {
            bail!("Symbol '{}' not found in index", symbol_name);
        }

        let mut all_affected_file_ids: Vec<u32> = Vec::new();
        let mut all_affected_symbols: Vec<(u32, u32)> = Vec::new();

        for &sym_id in &symbol_ids {
            // Use roaring cache for direct callers (fast bitmap lookup)
            let direct = self.index.roaring.direct_callers_of(sym_id);
            let direct_vec: Vec<u32> = direct.to_vec();
            for &file_id in &direct_vec {
                if !all_affected_file_ids.contains(&file_id) {
                    all_affected_file_ids.push(file_id);
                }
            }

            // Map caller symbol IDs to file IDs
            let callers = self.index.call_graph.callers(sym_id);
            for &caller_id in callers {
                if (caller_id as usize) < self.index.symbols.len() {
                    let sym = &self.index.symbols[caller_id as usize];
                    all_affected_symbols.push((sym.file_id, sym.name_id));
                }
            }

            // Use roaring transitive cache for depth > 1
            if depth > 1 {
                let transitive = self.index.roaring.get_transitive(sym_id);
                let transitive_vec: Vec<u32> = transitive.to_vec();
                for &file_id in &transitive_vec {
                    if !all_affected_file_ids.contains(&file_id) {
                        all_affected_file_ids.push(file_id);
                    }
                }
            }
        }

        // Classify affected files
        let mut direct_files: Vec<ImpactedFile> = Vec::new();
        let transitive_files: Vec<ImpactedFile> = Vec::new();
        let mut affected_tests: Vec<ImpactedFile> = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for &(file_id, name_id) in &all_affected_symbols {
            let file_path = self.get_string(file_id).to_string();
            let sym_name = self.get_string(name_id).to_string();
            let is_test = file_path.contains("test") || file_path.contains("spec");

            if !seen_files.insert(file_path.clone()) {
                let target_list = if is_test { &mut affected_tests } else { &mut direct_files };
                if let Some(entry) = target_list.iter_mut().find(|f| f.path == file_path) {
                    if !entry.symbols.contains(&sym_name) {
                        entry.symbols.push(sym_name);
                    }
                }
                continue;
            }

            let entry = ImpactedFile { path: file_path, symbols: vec![sym_name], is_test };
            if is_test {
                affected_tests.push(entry);
            } else {
                direct_files.push(entry);
            }
        }

        // Use Roaring set operations for test-only filtering
        if test_only {
            let test_file_ids: Vec<u32> = affected_tests
                .iter()
                .filter_map(|f| self.index.inverted.get_name_id(&f.path))
                .collect();
            let intersected = crate::index::bitmap::RoaringIndex::intersect(
                &all_affected_file_ids,
                &test_file_ids,
            );
            let test_count = intersected.len();
            return Ok(ImpactReport {
                symbol: symbol_name.to_string(),
                direct_files: Vec::new(),
                transitive_files: Vec::new(),
                affected_tests,
                total_files: test_count,
            });
        }

        let total_files = direct_files.len() + transitive_files.len() + affected_tests.len();

        Ok(ImpactReport {
            symbol: symbol_name.to_string(),
            direct_files,
            transitive_files,
            affected_tests,
            total_files,
        })
    }

    fn get_string(&self, id: u32) -> &str {
        self.index.strings.get(id as usize).map(|s| s.as_str()).unwrap_or("<unknown>")
    }
}
