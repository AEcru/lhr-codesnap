use ahash::HashMap;

/// Three-dimensional inverted index for symbol lookup.
///
/// D1: name_id → Vec<symbol_id>  (name to symbol locations)
/// D2: kind → Vec<name_id>       (kind to names)
/// D3: file_id → Vec<symbol_id>  (file to symbols)
pub struct InvertedIndex {
    name_to_symbols: HashMap<u32, Vec<u32>>,
    kind_to_names: HashMap<String, Vec<u32>>,
    file_to_symbols: HashMap<u32, Vec<u32>>,
    name_to_id: HashMap<String, u32>,
    id_to_name: Vec<String>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            name_to_symbols: HashMap::default(),
            kind_to_names: HashMap::default(),
            file_to_symbols: HashMap::default(),
            name_to_id: HashMap::default(),
            id_to_name: Vec::new(),
        }
    }

    /// Register a symbol name and return its name_id.
    #[allow(dead_code)]
    pub fn intern_name(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.name_to_id.get(name) {
            id
        } else {
            let id = self.name_to_id.len() as u32;
            self.name_to_id.insert(name.to_string(), id);
            self.id_to_name.push(name.to_string());
            id
        }
    }

    /// Look up a name_id by name string.
    pub fn get_name_id(&self, name: &str) -> Option<u32> {
        self.name_to_id.get(name).copied()
    }

    /// Force-register a name with a specific name_id (for deserialization).
    /// Unlike intern_name which assigns sequential IDs, this preserves the
    /// original name_id from the serialized index.
    pub fn register_name(&mut self, name: &str, name_id: u32) {
        self.name_to_id.insert(name.to_string(), name_id);
        let idx = name_id as usize;
        while self.id_to_name.len() <= idx {
            self.id_to_name.push(String::new());
        }
        self.id_to_name[idx] = name.to_string();
    }

    /// Look up a name string by name_id.
    pub fn get_name(&self, name_id: u32) -> Option<&str> {
        self.id_to_name.get(name_id as usize).map(|s| s.as_str())
    }

    /// Add a symbol to the name→symbols index.
    pub fn add_name(&mut self, name_id: u32, symbol_id: u32) {
        self.name_to_symbols.entry(name_id).or_default().push(symbol_id);
    }

    /// Add a name to the kind→names index.
    pub fn add_kind(&mut self, kind: &str, name_id: u32) {
        self.kind_to_names.entry(kind.to_string()).or_default().push(name_id);
    }

    /// Add a symbol to the file→symbols index.
    pub fn add_file(&mut self, file_id: u32, symbol_id: u32) {
        self.file_to_symbols.entry(file_id).or_default().push(symbol_id);
    }

    /// Find all symbol IDs for a given name (case-insensitive fallback).
    pub fn find_by_name(&self, name: &str) -> Vec<u32> {
        // Exact match first
        if let Some(&name_id) = self.name_to_id.get(name) {
            return self.name_to_symbols.get(&name_id).cloned().unwrap_or_default();
        }
        // Case-insensitive fallback for context/task-based searches
        let lower = name.to_lowercase();
        for (candidate, &name_id) in self.name_to_id.iter() {
            if candidate.to_lowercase() == lower {
                return self.name_to_symbols.get(&name_id).cloned().unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Find all name IDs of a given kind.
    pub fn find_by_kind(&self, kind: &str) -> Vec<u32> {
        self.kind_to_names.get(kind).cloned().unwrap_or_default()
    }

    /// Find all symbols in a given file.
    pub fn find_by_file(&self, file_id: u32) -> Vec<u32> {
        self.file_to_symbols.get(&file_id).cloned().unwrap_or_default()
    }

    /// Case-insensitive prefix search: find all name_ids starting with prefix.
    pub fn prefix_search(&self, prefix: &str) -> Vec<u32> {
        let lower = prefix.to_lowercase();
        self.name_to_id
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&lower))
            .map(|(_, &id)| id)
            .collect()
    }
}
