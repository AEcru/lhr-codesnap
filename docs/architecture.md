# CodeSnap Architecture

> This document provides a high-level overview. For detailed data structure design,
> see [../.claude/skills/lhr-codesnap/references/architecture.md](../.claude/skills/lhr-codesnap/references/architecture.md).

## Overview

CodeSnap is a CLI tool that builds a disk-based semantic index of a codebase,
then serves queries through mmap'd data structures. It integrates with Claude
Code via a Skill file (not an MCP server).

## Key Design Decisions

### Skill over MCP Server

- **0MB when idle** — process exits after each query
- **Zero config** — no MCP JSON to edit
- **Disk persistence** — index survives Claude Code restarts
- **Simpler debugging** — stdout is directly visible

### Custom binary index over SQLite

- No SQL parsing overhead per query
- Purpose-built data structures (inverted index, CSR, trie, Roaring Bitmap)
- mmap'd binary has zero deserialization cost
- No WAL mode issues on network drives / WSL2

### Rust over Node.js

- Single static binary (~5MB vs 80MB bundled Node)
- Memory-safe without GC pauses
- tree-sitter and ripgrep have first-class Rust bindings

## Query Flow

```
User asks AI → AI loads CodeSnap skill → AI runs codesnap command
  → CLI starts (10ms)
  → Self-check: compare file mtimes (1ms)
  → if stale: incremental reparse changed files (50-200ms)
  → mmap index (5ms)
  → query (1-50ms depending on type)
  → structured output → AI reads result

Total: 12ms (clean) to 250ms (with reparse)
```

## Index Lifecycle

```
codesnap init → Full parse of all source files
  → Build: string table, symbol table, inverted indexes, CSR graph, Bloom filters
  → Write LSM Tree: MemTable → flush to SSTables
  → Index file: ~5% of project size

Daily use → codesnap find/callers/etc.
  → CLI auto-detects changed files via mtime
  → Incremental reparse (only changed files)
  → LSM Tree append-write new data

codesnap init --force → Rebuild from scratch (compaction)
```

## Data Flow

```
Source Files (.java, .py, .ts, ...)
  │
  ▼
tree-sitter (AST parsing, per-file, on-demand or batch)
  │
  ├─→ Symbol extraction (name, kind, visibility, location)
  ├─→ Edge extraction (calls, imports, extends, implements)
  └─→ Framework route detection (14 frameworks)
  │
  ▼
Index Builder
  ├─→ String table (interned paths & names)
  ├─→ Symbol table (sorted by ID)
  ├─→ 3D Inverted Index (name→loc, kind→name, file→name)
  ├─→ Radix Trie (prefix search)
  ├─→ CSR Call Graph (forward + reverse, multi-level)
  ├─→ Roaring Bitmaps (per-symbol transitive caller sets)
  └─→ Bloom Filters (per-file, per-directory, global)
  │
  ▼
LSM Tree Storage (mmap'd binary file)
  │
  ▼
Query Engine (find, callers, callees, impact, trace, context)
```
