# Architecture Reference

## Data Structure Design

### 3D Inverted Index

Three-dimensional inverted index mapping symbol names, types, and files.

```
Dimension 1 (name → locations):
  "UserService" → [(file_id=5, line=12, kind=class, visibility=public)]
  "login"       → [(file_id=5, line=45, kind=method),
                   (file_id=12, line=8, kind=method)]

Dimension 2 (kind → names):
  class     → ["UserService", "AuthManager", "TokenStore"]
  interface → ["IAuthenticator", "ITokenProvider"]

Dimension 3 (file → symbols):
  file_id=5 → ["UserService", "login", "logout", "refreshToken"]
```

Storage: Hash map for D1/D2, B-Tree for D3 (enables range scans by file path).
Memory: ~50MB per 10k files (symbol names are interned via string table).

### Radix Trie (Prefix Tree)

Compressed prefix tree for symbol name auto-completion and prefix search.

```
         getUser
        /       \
  getUserById   getUserByName
```

Nodes store compressed edge labels. Query "getUser*" walks the path in O(k)
where k is the prefix length. Supports infix search via suffix links for
fuzzy matching.

### CSR (Compressed Sparse Row) Call Graph

Two CSR matrices (forward + reverse) stored as contiguous integer arrays:

```rust
// Forward: caller → callees
caller_offsets: [0, 3, 5, 8, 12, ...]   // offset in edges array per node
edges:          [5,12,99, 3,7, 1,8,15, ...]  // callee node IDs

// Reverse: callee → callers (for callers() queries)
callee_offsets: [0, 2, 4, 7, ...]
reverse_edges:  [...]
```

Multi-level CSR:
- Level 2: Module-to-module calls (~100 nodes, fastest traversal)
- Level 1: File-to-file calls (~5000 nodes)
- Level 0: Function-to-function calls (~50000 nodes, most granular)

BFS trace starts at Level 2, descends only if needed. 80% of queries resolve at
Level 1 or 2.

Memory: ~400KB per 10k nodes / 50k edges (u32 IDs, 8 bytes per edge).

### Roaring Bitmap

Used for impact analysis set operations. Each symbol's transitive caller set is
a Roaring Bitmap of file IDs.

```
transitive_callers[TokenUtil] → RoaringBitmap{file_1, file_5, file_12, ...}
test_files                   → RoaringBitmap{file_3, file_5, file_18, ...}

affected_tests = transitive_callers[TokenUtil] ∩ test_files  // bitwise AND
```

Set operations (∪, ∩, difference) execute in O(n/64) via bulk bitwise ops.

### LSM Tree Storage Engine

Append-only write path with leveled compaction:

```
Write:
  edit detected → parse changed file → write to MemTable (in-memory HashMap)
  MemTable full → flush to Level 0 SSTable (sorted, immutable)
  Background compaction: merge L0 → L1 → L2 → ... → Ln

Read:
  MemTable (newest) → Level 0 → Level 1 → ... → Level N
  Bloom Filter per level: skip level if key not present
  90% of reads hit MemTable or L0
```

### Bloom Filter (Cold Start)

Per-file Bloom filters for fast exclusion when the full index isn't loaded yet.

```
Query "UserService" during cold start:
  1. Check Project-level BF → if no, abort (symbol doesn't exist)
  2. Check Directory-level BFs → narrow to 2-3 directories
  3. Check File-level BFs in candidate dirs → identify 3-5 files
  4. ripgrep only those 3-5 files
  Total: ~5ms vs ~200ms for full ripgrep scan
```

Size: ~1KB per file at 1% false positive rate. 10k files = 10MB total.

## Performance Characteristics

| Query Type | Cold (no index) | Warm (index on disk) | Hot (mmap'd) |
|-----------|-----------------|---------------------|--------------|
| find | 200-500ms (rg scan) | 10-15ms | <1ms |
| callers | 300-800ms (rg ×2) | 12-18ms | <1ms |
| callees | 300-800ms | 12-18ms | <1ms |
| impact (depth 3) | 2-5s | 20-50ms | 5-10ms |
| trace | 3-10s (manual grep) | 30-100ms | 10-30ms |
| context | 5-15s (multi-rg) | 50-200ms | 20-50ms |

## Comparison with Traditional MCP Solutions

| Component | Traditional MCP | CodeSnap |
|-----------|-----------|----------|
| Search | SQLite FTS5 (B-Tree) | 3D Inverted Index + Trie |
| Call graph | Adjacency list in SQLite | CSR multi-level |
| Impact analysis | SQL recursive CTE | Roaring Bitmap |
| Storage | SQLite B-Tree | LSM Tree |
| Cold start | Blocked until index built | Bloom + ripgrep fallback |
| Binary | ~80MB (Node bundled) | ~5MB (Rust musl static) |
| Memory idle | 100-300MB | 0MB |
| Memory active | 100-300MB | 30-80MB |

## Index File Format

```
Header (64 bytes):
  - Magic: b"CSNAP001"
  - Version: u32
  - Created: i64 (unix timestamp)
  - Section offsets: [u64; 8]

Section 1: String Table
  - Interned strings (file paths, symbol names)
  - Sorted for binary search

Section 2: Symbol Table
  - [(name_id, kind, visibility, file_id, line, checksum); N]

Section 3: Inverted Index D1
  - HashMap<name_id, Vec<symbol_id>>

Section 4: Inverted Index D2
  - HashMap<kind, Vec<symbol_id>>

Section 5: Inverted Index D3
  - BTree<file_id, Vec<symbol_id>>

Section 6: Call Graph CSR (Forward)
  - [offsets; N+1], [edges; E]

Section 7: Call Graph CSR (Reverse)
  - [offsets; N+1], [edges; E]

Section 8: File Bloom Filters
  - [(file_id, bloom_filter_bytes); N]

Section 9: Checksums
  - CRC32 per section for corruption detection
```

### Version Compatibility

- Major version bump: incompatible format change
- Minor version bump: additive changes (new sections appended)
- Old readers skip unknown sections
- `codesnap init --force` rebuilds from scratch for major version upgrades
