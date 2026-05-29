use crate::index::Index;
use anyhow::{Result, bail};

/// Call chain tracer — finds paths between two symbols through the call graph.
pub struct Tracer<'a> {
    index: &'a Index,
}

#[derive(Debug, Clone)]
pub struct TracePath {
    /// Ordered list of hops from `from` to `to`, inclusive.
    pub hops: Vec<TraceHop>,
    /// Total number of intermediate nodes.
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct TraceHop {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub kind: String,
}

impl<'a> Tracer<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index }
    }

    /// Trace the call path from `from` symbol to `to` symbol.
    pub fn trace(
        &self, from: &str, to: &str, max_depth: usize, all_paths: bool,
    ) -> Result<Vec<TracePath>> {
        let from_ids = self.index.inverted.find_by_name(from);
        let to_ids = self.index.inverted.find_by_name(to);

        if from_ids.is_empty() {
            bail!("Symbol '{}' not found in index", from);
        }
        if to_ids.is_empty() {
            bail!("Symbol '{}' not found in index", to);
        }

        let mut paths = Vec::new();

        for &from_id in &from_ids {
            for &to_id in &to_ids {
                let node_path = self.index.call_graph.bfs_trace(from_id, to_id, max_depth);
                if !node_path.is_empty() {
                    let hops: Vec<TraceHop> = node_path
                        .iter()
                        .map(|&node_id| {
                            if (node_id as usize) < self.index.symbols.len() {
                                let sym = &self.index.symbols[node_id as usize];
                                TraceHop {
                                    name: self.get_string(sym.name_id).to_string(),
                                    file: self.get_string(sym.file_id).to_string(),
                                    line: sym.line,
                                    kind: sym.kind.as_str().to_string(),
                                }
                            } else {
                                TraceHop {
                                    name: format!("<node:{}>", node_id),
                                    file: "<unknown>".to_string(),
                                    line: 0,
                                    kind: "unknown".to_string(),
                                }
                            }
                        })
                        .collect();

                    let depth = hops.len().saturating_sub(1);
                    paths.push(TracePath { hops, depth });

                    if !all_paths {
                        break;
                    }
                }
            }
            if !all_paths && !paths.is_empty() {
                break;
            }
        }

        if paths.is_empty() {
            bail!(
                "No call path found from '{}' to '{}' within depth {}",
                from, to, max_depth
            );
        }

        Ok(paths)
    }

    fn get_string(&self, id: u32) -> &str {
        self.index.strings.get(id as usize).map(|s| s.as_str()).unwrap_or("<unknown>")
    }
}
