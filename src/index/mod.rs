mod bitmap;
mod builder;
mod csr;
mod inverted;
mod trie;

pub use bitmap::RoaringIndex;
pub use builder::Builder;
pub use csr::CallGraph;
pub use inverted::InvertedIndex;
pub use trie::SymbolTrie;

use anyhow::{Context, Result};
use std::path::Path;

/// In-memory representation of the full code index.
pub struct Index {
    /// Project root path
    pub root: String,
    /// Interned string table: id → string
    pub strings: Vec<String>,
    /// Symbol table: all indexed symbols
    pub symbols: Vec<Symbol>,
    /// 3D inverted index
    pub inverted: InvertedIndex,
    /// Prefix trie for symbol name search
    pub trie: SymbolTrie,
    /// Compressed call graph (forward + reverse)
    pub call_graph: CallGraph,
    /// Roaring bitmap for impact analysis
    pub roaring: RoaringIndex,
    /// Per-file mtime cache for freshness checks
    pub file_mtimes: Vec<(String, u64)>,
}

/// A single indexed symbol.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// String table ID for the symbol name
    pub name_id: u32,
    /// String table ID for the file path
    pub file_id: u32,
    /// 1-based line number
    pub line: u32,
    /// Symbol kind: class, function, method, interface, etc.
    pub kind: SymbolKind,
    /// Visibility: public, private, protected, internal
    pub visibility: Visibility,
    /// String table ID for the parent scope (class/module name)
    pub parent_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Class,
    Function,
    Method,
    Interface,
    Enum,
    TypeAlias,
    Variable,
    Constant,
    Unknown,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Class => "class",
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Interface => "interface",
            SymbolKind::Enum => "enum",
            SymbolKind::TypeAlias => "type",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "class" => SymbolKind::Class,
            "function" => SymbolKind::Function,
            "method" => SymbolKind::Method,
            "interface" => SymbolKind::Interface,
            "enum" => SymbolKind::Enum,
            "type" => SymbolKind::TypeAlias,
            "variable" => SymbolKind::Variable,
            "constant" => SymbolKind::Constant,
            _ => SymbolKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Unknown,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
            Visibility::Protected => "protected",
            Visibility::Internal => "internal",
            Visibility::Unknown => "",
        }
    }
}

/// Open the index for the current directory.
pub fn open_current() -> Result<Index> {
    open(".")
}

/// Open the index at the given project path.
pub fn open(path: &str) -> Result<Index> {
    let index_path = Path::new(path).join(".codesnap").join("index.bin");
    if !index_path.exists() {
        anyhow::bail!(
            "Index not found at {}. Run `codesnap init` first.",
            index_path.display()
        );
    }
    let data = std::fs::read(&index_path)
        .with_context(|| format!("Failed to read index at {}", index_path.display()))?;
    deserialize(&data)
}

/// Show index status for a project path.
pub fn status(path: &str) -> Result<StatusReport> {
    let index_path = Path::new(path).join(".codesnap").join("index.bin");
    if !index_path.exists() {
        return Ok(StatusReport {
            state: "Not initialized".into(),
            symbols_count: 0,
            files_count: 0,
            disk_size_bytes: 0,
            last_indexed: String::new(),
            pending_files: vec![],
        });
    }
    let metadata = std::fs::metadata(&index_path)?;
    let data = std::fs::read(&index_path)?;
    let idx = deserialize(&data)?;

    Ok(StatusReport {
        state: "Ready".into(),
        symbols_count: idx.symbols.len(),
        files_count: idx.file_mtimes.len(),
        disk_size_bytes: metadata.len(),
        last_indexed: "unknown".into(),
        pending_files: vec![],
    })
}

#[derive(Debug)]
pub struct StatusReport {
    pub state: String,
    pub symbols_count: usize,
    pub files_count: usize,
    pub disk_size_bytes: u64,
    pub last_indexed: String,
    pub pending_files: Vec<String>,
}

fn deserialize(data: &[u8]) -> Result<Index> {
    if data.len() < 23 || &data[..7] != b"CSNAP01" {
        anyhow::bail!("Invalid or corrupted index file");
    }

    let mut pos: usize = 7;

    let version = read_u32(data, &mut pos)?;
    if version != 1 {
        anyhow::bail!("Unsupported index version: {}", version);
    }

    let symbol_count = read_u32(data, &mut pos)? as usize;
    let string_count = read_u32(data, &mut pos)? as usize;
    let mtime_count = read_u32(data, &mut pos)? as usize;

    // Read string table
    let mut strings: Vec<String> = Vec::with_capacity(string_count);
    for _ in 0..string_count {
        let len = read_u32(data, &mut pos)? as usize;
        if pos + len > data.len() {
            anyhow::bail!("Index file truncated at string table");
        }
        let s = String::from_utf8(data[pos..pos + len].to_vec())
            .with_context(|| "Invalid UTF-8 in string table")?;
        pos += len;
        strings.push(s);
    }

    // Rebuild string→id map for inverted index intern
    let _string_map: std::collections::HashMap<String, u32> = strings
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i as u32))
        .collect();

    // Read symbols
    let mut symbols: Vec<Symbol> = Vec::with_capacity(symbol_count);
    let mut inverted = InvertedIndex::new();
    let mut trie = SymbolTrie::new();

    for sym_idx in 0..symbol_count {
        let name_id = read_u32(data, &mut pos)?;
        let file_id = read_u32(data, &mut pos)?;
        let line = read_u32(data, &mut pos)?;
        let kind_len = read_u8(data, &mut pos)? as usize;
        let kind_str = read_str(data, &mut pos, kind_len)?;
        let vis_len = read_u8(data, &mut pos)? as usize;
        let vis_str = read_str(data, &mut pos, vis_len)?;

        let kind = SymbolKind::from_str(kind_str);
        let visibility = match vis_str {
            "public" => Visibility::Public,
            "private" => Visibility::Private,
            "protected" => Visibility::Protected,
            "internal" => Visibility::Internal,
            _ => Visibility::Unknown,
        };

        let sym = Symbol { name_id, file_id, line, kind, visibility, parent_id: None };
        symbols.push(sym);

        // Populate inverted index and trie
        let sym_id = sym_idx as u32;
        inverted.add_name(name_id, sym_id);
        inverted.add_kind(kind_str, name_id);
        inverted.add_file(file_id, sym_id);

        if let Some(name) = strings.get(name_id as usize) {
            trie.insert(name, name_id);
            inverted.intern_name(name);
        }
    }

    // Read file mtimes
    let mut file_mtimes: Vec<(String, u64)> = Vec::with_capacity(mtime_count);
    for _ in 0..mtime_count {
        let len = read_u32(data, &mut pos)? as usize;
        let path = read_str(data, &mut pos, len)?.to_string();
        let mtime = read_u64(data, &mut pos)?;
        file_mtimes.push((path, mtime));
    }

    Ok(Index {
        root: String::new(),
        strings,
        symbols,
        inverted,
        trie,
        call_graph: CallGraph::new(),
        roaring: RoaringIndex::new(),
        file_mtimes,
    })
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > data.len() {
        anyhow::bail!("Index file truncated at position {}", *pos);
    }
    let val = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(val)
}

fn read_u64(data: &[u8], pos: &mut usize) -> Result<u64> {
    if *pos + 8 > data.len() {
        anyhow::bail!("Index file truncated at position {}", *pos);
    }
    let val = u64::from_le_bytes([
        data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3],
        data[*pos + 4], data[*pos + 5], data[*pos + 6], data[*pos + 7],
    ]);
    *pos += 8;
    Ok(val)
}

fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8> {
    if *pos >= data.len() {
        anyhow::bail!("Index file truncated at position {}", *pos);
    }
    let val = data[*pos];
    *pos += 1;
    Ok(val)
}

fn read_str<'a>(data: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a str> {
    if *pos + len > data.len() {
        anyhow::bail!("Index file truncated at string at position {}", *pos);
    }
    let s = std::str::from_utf8(&data[*pos..*pos + len])
        .with_context(|| "Invalid UTF-8 in index")?;
    *pos += len;
    Ok(s)
}
