use super::{Index, Symbol, SymbolKind, Visibility};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// Builds the CodeSnap index for a project.
pub struct Builder {
    path: String,
    force: bool,
    quiet: bool,
}

impl Builder {
    pub fn new(path: &str) -> Self {
        Self { path: path.to_string(), force: false, quiet: false }
    }

    pub fn force(mut self, v: bool) -> Self {
        self.force = v;
        self
    }

    pub fn quiet(mut self, v: bool) -> Self {
        self.quiet = v;
        self
    }

    /// Run the full index build.
    pub fn build(&self) -> Result<()> {
        let root = Path::new(&self.path);
        let codesnap_dir = root.join(".codesnap");
        let index_path = codesnap_dir.join("index.bin");

        if index_path.exists() && !self.force {
            anyhow::bail!(
                "Index already exists at {}. Use --force to rebuild.",
                index_path.display()
            );
        }

        fs::create_dir_all(&codesnap_dir)?;

        if !self.quiet {
            eprintln!("Scanning project files...");
        }

        let source_files = self.collect_files(root)?;

        if !self.quiet {
            eprintln!("Found {} source files. Building index...", source_files.len());
        }

        let mut index = Index {
            root: root.to_string_lossy().to_string(),
            strings: Vec::new(),
            symbols: Vec::new(),
            inverted: super::InvertedIndex::new(),
            trie: super::SymbolTrie::new(),
            call_graph: super::CallGraph::new(),
            roaring: super::RoaringIndex::new(),
            file_mtimes: Vec::new(),
        };

        let mut string_map: HashMap<String, u32> = HashMap::new();

        for (i, file_path) in source_files.iter().enumerate() {
            if !self.quiet && i % 50 == 0 {
                eprintln!("  [{}/{}] parsing...", i + 1, source_files.len());
            }
            self.index_file(file_path, &mut index, &mut string_map)?;
        }

        if !self.quiet {
            eprintln!(
                "Indexed {} symbols across {} files.",
                index.symbols.len(),
                index.file_mtimes.len()
            );
            eprintln!("Writing index to disk...");
        }

        let data = serialize(&index)?;
        fs::write(&index_path, &data)?;

        if !self.quiet {
            let size_kb = data.len() as f64 / 1024.0;
            eprintln!("Index written: {:.1} KB", size_kb);
        }

        Ok(())
    }

    fn collect_files(&self, root: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let extensions: &[&str] = &[
            "rs", "py", "java", "go", "ts", "tsx", "js", "jsx", "mjs", "cs", "php",
            "rb", "c", "h", "cpp", "hpp", "cc", "swift", "kt", "kts", "dart",
            "vue", "svelte", "lua", "luau",
        ];

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !self.is_excluded_dir(&name)
            })
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            if let Some(ext) = entry.path().extension() {
                if extensions.contains(&ext.to_string_lossy().as_ref()) {
                    if let Ok(relative) = entry.path().strip_prefix(root) {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        if size < 1_000_000 {
                            files.push(relative.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    fn is_excluded_dir(&self, name: &str) -> bool {
        let excluded = [
            "node_modules", "target", "dist", "build", ".git", ".codesnap",
            "__pycache__", ".venv", "venv", "vendor", "Pods", ".next", ".nuxt",
            "bazel-bin", "bazel-out", "bazel-testlogs",
        ];
        excluded.contains(&name) || name.starts_with('.')
    }

    fn index_file(
        &self,
        relative_path: &str,
        index: &mut Index,
        string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        let full_path = Path::new(&index.root).join(relative_path);
        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };

        let metadata = fs::metadata(&full_path)?;
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let file_id = Self::intern_string(relative_path, &mut index.strings, string_map);

        // Extract symbols using simple regex-based parsing
        self.extract_symbols(relative_path, file_id, &content, index, string_map)?;

        index.file_mtimes.push((relative_path.to_string(), mtime));

        Ok(())
    }

    fn extract_symbols(
        &self,
        path: &str,
        file_id: u32,
        content: &str,
        index: &mut Index,
        string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "rs" => self.extract_rust(path, file_id, content, index, string_map),
            "py" => self.extract_python(path, file_id, content, index, string_map),
            "java" => self.extract_java(path, file_id, content, index, string_map),
            "go" => self.extract_go(path, file_id, content, index, string_map),
            "ts" | "tsx" | "js" | "jsx" | "mjs" => {
                self.extract_typescript(path, file_id, content, index, string_map)
            }
            _ => self.extract_generic(path, file_id, content, index, string_map),
        }
    }

    fn extract_rust(
        &self, _path: &str, file_id: u32, content: &str,
        index: &mut Index, string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        // Match: pub fn name(...), fn name(...), pub struct Name, struct Name
        let fn_re = regex_lite::Regex::new(
            r"(?m)^\s*(pub(?:\s*\(\s*crate\s*\))?\s+)?fn\s+(\w+)\s*[<(]"
        ).unwrap();
        let struct_re = regex_lite::Regex::new(
            r"(?m)^\s*(pub(?:\s*\(\s*crate\s*\))?\s+)?struct\s+(\w+)"
        ).unwrap();
        let trait_re = regex_lite::Regex::new(
            r"(?m)^\s*(pub(?:\s*\(\s*crate\s*\))?\s+)?trait\s+(\w+)"
        ).unwrap();
        let impl_re = regex_lite::Regex::new(
            r"(?m)^\s*impl\s+(?:<\w+>\s+)?(\w+)"
        ).unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = fn_re.captures(line) {
                let visibility = if caps.get(1).is_some() && line.contains("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let name = caps.get(2).unwrap().as_str();
                if Self::is_common_keyword(name) {
                    continue;
                }
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let sym = Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Function, visibility, parent_id: None,
                };
                index.symbols.push(sym);
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = struct_re.captures(line) {
                let visibility = if caps.get(1).is_some() && line.contains("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let name = caps.get(2).unwrap().as_str();
                if Self::is_common_keyword(name) { continue; }
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let sym = Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Class, visibility, parent_id: None,
                };
                index.symbols.push(sym);
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = trait_re.captures(line) {
                let name = caps.get(2).unwrap().as_str();
                if Self::is_common_keyword(name) { continue; }
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let sym = Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Interface, visibility: Visibility::Public,
                    parent_id: None,
                };
                index.symbols.push(sym);
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            // Track call edges via function calls
            if let Some(caps) = impl_re.captures(line) {
                let target = caps.get(1).unwrap().as_str();
                if let Some(&target_name_id) = string_map.get(target) {
                    index.call_graph.add_edge(file_id, target_name_id);
                }
            }
        }
        Ok(())
    }

    fn extract_python(
        &self, _path: &str, file_id: u32, content: &str,
        index: &mut Index, string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        let class_re = regex_lite::Regex::new(r"(?m)^class\s+(\w+)").unwrap();
        let def_re = regex_lite::Regex::new(r"(?m)^\s*def\s+(\w+)\s*\(").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = class_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Class, visibility: Visibility::Public,
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = def_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                if name.starts_with('_') && name != "__init__" { continue; }
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let kind = if line.contains("self") || name == "__init__" {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                };
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind, visibility: Visibility::Public, parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
        }
        Ok(())
    }

    fn extract_java(
        &self, _path: &str, file_id: u32, content: &str,
        index: &mut Index, string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        let class_re = regex_lite::Regex::new(
            r"(?m)^\s*(public|private|protected)?\s*(static)?\s*(class|interface|enum)\s+(\w+)"
        ).unwrap();
        let method_re = regex_lite::Regex::new(
            r"(?m)^\s*(public|private|protected)?\s*(static)?\s*\w+\s+(\w+)\s*\([^)]*\)\s*\{?"
        ).unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = class_re.captures(line) {
                let vis = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let kind_str = caps.get(3).unwrap().as_str();
                let name = caps.get(4).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let kind = match kind_str {
                    "interface" => SymbolKind::Interface,
                    "enum" => SymbolKind::Enum,
                    _ => SymbolKind::Class,
                };
                let visibility = match vis {
                    "public" => Visibility::Public,
                    "private" => Visibility::Private,
                    "protected" => Visibility::Protected,
                    _ => Visibility::Internal,
                };
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind, visibility, parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = method_re.captures(line) {
                if line.contains(';') || line.contains("class") || line.contains("interface") {
                    continue;
                }
                let vis = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let name = caps.get(3).unwrap().as_str();
                if Self::is_common_keyword(name) { continue; }
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let visibility = match vis {
                    "public" => Visibility::Public,
                    "private" => Visibility::Private,
                    "protected" => Visibility::Protected,
                    _ => Visibility::Internal,
                };
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Method, visibility, parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
        }
        Ok(())
    }

    fn extract_go(
        &self, _path: &str, file_id: u32, content: &str,
        index: &mut Index, string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        let func_re = regex_lite::Regex::new(r"(?m)^func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)\s*\(").unwrap();
        let type_re = regex_lite::Regex::new(r"(?m)^type\s+(\w+)\s+struct").unwrap();
        let iface_re = regex_lite::Regex::new(r"(?m)^type\s+(\w+)\s+interface").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = func_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                let is_public = name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Function,
                    visibility: if is_public { Visibility::Public } else { Visibility::Private },
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = type_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Class, visibility: Visibility::Public,
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = iface_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Interface, visibility: Visibility::Public,
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
        }
        Ok(())
    }

    fn extract_typescript(
        &self, _path: &str, file_id: u32, content: &str,
        index: &mut Index, string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        let func_re = regex_lite::Regex::new(
            r"(?m)(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*[<(]"
        ).unwrap();
        let class_re = regex_lite::Regex::new(
            r"(?m)(?:export\s+)?class\s+(\w+)"
        ).unwrap();
        let arrow_re = regex_lite::Regex::new(
            r"(?m)(?:export\s+)?(?:const|let|var)\s+(\w+)\s*[:=]\s*(?:async\s+)?\([^)]*\)\s*(?::\s*\w+)?\s*=>"
        ).unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = func_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Function,
                    visibility: if line.contains("export") { Visibility::Public } else { Visibility::Private },
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = class_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Class,
                    visibility: if line.contains("export") { Visibility::Public } else { Visibility::Private },
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
            if let Some(caps) = arrow_re.captures(line) {
                let name = caps.get(1).unwrap().as_str();
                let name_id = Self::intern_string(name, &mut index.strings, string_map);
                index.symbols.push(Symbol {
                    name_id, file_id, line: (line_num + 1) as u32,
                    kind: SymbolKind::Function, visibility: Visibility::Public,
                    parent_id: None,
                });
                index.trie.insert(name, name_id);
                index.inverted.add_name(name_id, index.symbols.len() as u32 - 1);
            }
        }
        Ok(())
    }

    fn extract_generic(
        &self, _path: &str, _file_id: u32, _content: &str,
        _index: &mut Index, _string_map: &mut HashMap<String, u32>,
    ) -> Result<()> {
        Ok(())
    }

    fn intern_string(s: &str, strings: &mut Vec<String>, map: &mut HashMap<String, u32>) -> u32 {
        if let Some(&id) = map.get(s) {
            id
        } else {
            let id = strings.len() as u32;
            strings.push(s.to_string());
            map.insert(s.to_string(), id);
            id
        }
    }

    fn is_common_keyword(name: &str) -> bool {
        matches!(
            name, "if" | "else" | "for" | "while" | "loop" | "match" | "switch"
            | "case" | "return" | "break" | "continue" | "let" | "const" | "var"
            | "new" | "true" | "false" | "null" | "nil" | "None" | "self"
            | "main" | "use" | "mod" | "import" | "export" | "from" | "as"
            | "type" | "in" | "is" | "not" | "and" | "or"
        )
    }
}

fn serialize(index: &Index) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    // Header
    buf.extend_from_slice(b"CSNAP01");
    buf.extend_from_slice(&1u32.to_le_bytes()); // version
    buf.extend_from_slice(&(index.symbols.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(index.strings.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(index.file_mtimes.len() as u32).to_le_bytes());
    // String table
    for s in &index.strings {
        let bytes = s.as_bytes();
        buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytes);
    }
    // Symbols
    for sym in &index.symbols {
        buf.extend_from_slice(&sym.name_id.to_le_bytes());
        buf.extend_from_slice(&sym.file_id.to_le_bytes());
        buf.extend_from_slice(&sym.line.to_le_bytes());
        buf.push(sym.kind.as_str().len() as u8);
        buf.extend_from_slice(sym.kind.as_str().as_bytes());
        buf.push(sym.visibility.as_str().len() as u8);
        buf.extend_from_slice(sym.visibility.as_str().as_bytes());
    }
    // File mtimes
    for (path, mtime) in &index.file_mtimes {
        let bytes = path.as_bytes();
        buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytes);
        buf.extend_from_slice(&mtime.to_le_bytes());
    }
    Ok(buf)
}
