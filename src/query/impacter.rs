use crate::index::Index;
use anyhow::{Result, bail};

/// Impact analysis — determine what code is affected by changing a symbol.
pub struct Impacter<'a> {
    index: &'a Index,
}

#[derive(Debug, Clone)]
pub struct ImpactReport {
    pub symbol: String,
    /// Files directly calling the symbol
    pub direct_files: Vec<ImpactedFile>,
    /// Files transitively calling the symbol (depth > 1)
    pub transitive_files: Vec<ImpactedFile>,
    /// Test files among the affected set
    pub affected_tests: Vec<ImpactedFile>,
    /// Total number of unique files affected
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

        let mut direct_files: Vec<ImpactedFile> = Vec::new();
        let mut transitive_files: Vec<ImpactedFile> = Vec::new();
        let mut affected_tests: Vec<ImpactedFile> = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for &sym_id in &symbol_ids {
            // Direct callers (depth 1)
            let callers = self.index.call_graph.callers(sym_id);
            for &caller_id in callers {
                if caller_id as usize >= self.index.symbols.len() {
                    continue;
                }
                let sym = &self.index.symbols[caller_id as usize];
                let file_path = self.get_string(sym.file_id).to_string();
                let sym_name = self.get_string(sym.name_id).to_string();
                let is_test = file_path.contains("test") || file_path.contains("spec");

                if !seen_files.insert(file_path.clone()) {
                    // Already saw this file, just append symbol
                    if let Some(df) = direct_files.iter_mut().find(|f| f.path == file_path) {
                        if !df.symbols.contains(&sym_name) {
                            df.symbols.push(sym_name);
                        }
                    }
                    continue;
                }

                let entry = ImpactedFile {
                    path: file_path,
                    symbols: vec![sym_name],
                    is_test,
                };
                if is_test {
                    affected_tests.push(entry);
                } else {
                    direct_files.push(entry);
                }
            }

            // Transitive callers (depth > 1)
            if depth > 1 {
                let transitive = self.index.call_graph.transitive_callers(sym_id, depth);
                for &caller_id in &transitive {
                    // Skip direct callers (already handled)
                    if callers.contains(&caller_id) {
                        continue;
                    }
                    if caller_id as usize >= self.index.symbols.len() {
                        continue;
                    }
                    let sym = &self.index.symbols[caller_id as usize];
                    let file_path = self.get_string(sym.file_id).to_string();
                    let sym_name = self.get_string(sym.name_id).to_string();
                    let is_test = file_path.contains("test") || file_path.contains("spec");

                    if !seen_files.insert(file_path.clone()) {
                        if let Some(tf) = transitive_files.iter_mut().find(|f| f.path == file_path) {
                            if !tf.symbols.contains(&sym_name) {
                                tf.symbols.push(sym_name);
                            }
                        }
                        continue;
                    }

                    let entry = ImpactedFile {
                        path: file_path,
                        symbols: vec![sym_name],
                        is_test,
                    };
                    if is_test {
                        affected_tests.push(entry);
                    } else {
                        transitive_files.push(entry);
                    }
                }
            }
        }

        let total_files = direct_files.len() + transitive_files.len() + affected_tests.len();

        if test_only {
            let test_count = affected_tests.len();
            return Ok(ImpactReport {
                symbol: symbol_name.to_string(),
                direct_files: Vec::new(),
                transitive_files: Vec::new(),
                affected_tests,
                total_files: test_count,
            });
        }

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
