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
    if data.len() < 12 || &data[..7] != b"CSNAP01" {
        anyhow::bail!("Invalid or corrupted index file");
    }
    // Placeholder: full deserialization would parse each section
    Ok(Index {
        root: String::new(),
        strings: Vec::new(),
        symbols: Vec::new(),
        inverted: InvertedIndex::new(),
        trie: SymbolTrie::new(),
        call_graph: CallGraph::new(),
        roaring: RoaringIndex::new(),
        file_mtimes: Vec::new(),
    })
}
