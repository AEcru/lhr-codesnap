# CodeSnap — Zero-Config Semantic Code Intelligence for AI Coding Agents

> Instant code understanding via Claude Code Skill — no MCP server, no background process, no warmup.

## What It Is

CodeSnap is a lightweight CLI tool + Claude Code Skill that gives AI coding agents instant
semantic understanding of any codebase. It uses a pre-built disk index (mmap'd, zero-copy)
with data structures optimized for code analysis: inverted indexes, compressed sparse row
(CSR) call graphs, tries, and Roaring Bitmaps.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| CLI | Rust (musl static binary, ~5MB) |
| Index storage | Custom binary format, mmap'd read |
| AST parsing | tree-sitter (on-demand, per-file) |
| Data structures | Inverted index + Trie + CSR + Roaring Bitmap |
| Fallback search | ripgrep-core |
| AI integration | Claude Code Skill (SKILL.md, skill-creator format) |

## Project Structure

```
codesnap/
├── README.md                   # Chinese (primary display)
├── README.en.md                # English
├── LICENSE                     # MIT
├── CLAUDE.md                   # This file
├── .claude/
│   ├── rules/
│   │   └── project-rules.md    # Development rules
│   └── skills/
│       └── lhr-codesnap/
│           ├── SKILL.md         # Skill entry point (triggers AI)
│           └── references/      # Loaded on-demand
│               ├── commands.md  # Full CLI reference
│               └── architecture.md  # Data structure design
├── src/                        # CLI source code (Rust)
│   ├── main.rs
│   ├── index/
│   │   ├── builder.rs          # Index construction
│   │   ├── inverted.rs         # 3D inverted index
│   │   ├── trie.rs             # Prefix tree
│   │   ├── csr.rs              # Compressed call graph
│   │   └── bitmap.rs           # Roaring Bitmap impact analysis
│   ├── query/
│   │   ├── finder.rs           # Symbol search
│   │   ├── tracer.rs           # Call chain tracing
│   │   └── impacter.rs         # Impact analysis
│   ├── sync/
│   │   └── checker.rs          # mtime-based auto-sync
│   └── output/
│       └── format.rs           # Structured output
└── docs/
    └── architecture.md         # Architecture design doc
```

## Commands

```bash
codesnap init [path]            # Build full index
codesnap find <name>            # Search symbol definition
codesnap callers <name>         # Find callers
codesnap callees <name>         # Find callees
codesnap impact <name>          # Impact analysis
codesnap trace <from> <to>      # Trace call path
codesnap context <task>         # Build context for a task
codesnap check                  # Index health + auto-sync
codesnap status                 # Index stats + coverage
```

## Core Design Principles

1. **Zero-config**: No MCP JSON, no background daemon, no API keys.
2. **Lazy-first**: Index on disk, mmap on-demand, process exits when done.
3. **Self-healing**: Every query auto-checks file mtime and incrementally re-indexes.
4. **Tiny footprint**: <5MB binary, <5% of project size for index, 0MB RAM when idle.
5. **Best data structure per query type**: Not one database for everything.

## Development Rules

### CRITICAL

1. **Binary must stay under 8MB** — strip symbols, LTO, opt-level="z"
2. **Index format must be backward-compatible** — versioned binary format
3. **Every query must self-check freshness** — mtime comparison before returning results
4. **No network calls, no telemetry, no phone-home** — 100% local always
5. **Cross-platform** — Windows/macOS/Linux, x64 + arm64

### Code Standards

- Rust 2024 edition, no unsafe except for mmap
- Single binary, no dynamic linking (musl on Linux)
- Error messages must be actionable ("Run `codesnap init` first" not "Index not found")
- Output format: JSON lines for AI consumption, pretty-print for humans
- All public functions documented with doc comments

### Index Design Rules

- Index file is append-only for durability; compaction runs on `codesnap init --force`
- Checksum per section for corruption detection
- File paths stored as interned integers (u32), mapped via a string table
- Symbol names use a shared prefix trie for compression

### SKILL.md Rules

- Keep SKILL.md under 150 lines
- Reference files under 500 lines each, with table of contents
- Commands table must match CLI exactly
- Use concise English for AI comprehension
- Update SKILL.md when CLI interface changes
- Follow skill-creator progressive disclosure: core in SKILL.md, details in references/

## Architecture Decision Records

### Why Skill instead of MCP Server?

- Zero background memory (0MB when not in use)
- No configuration files to edit
- Cross-session index persistence (disk survives Claude Code restarts)
- Simpler debugging (stdout visible directly)
- Users only pay cost when they need it

### Why Rust instead of Node.js?

- Single static binary, no runtime dependency
- 5MB vs 80MB (typical MCP servers bundle a runtime)
- Memory-safe without GC pauses
- tree-sitter and ripgrep have first-class Rust bindings

### Why custom binary index instead of SQLite?

- SQLite has per-query SQL parsing overhead
- Custom structures (CSR, Trie, Roaring) are 10-1000x faster for specific queries
- mmap'd binary has zero deserialization cost
- SQLite WAL mode breaks on network drives / WSL2 `/mnt`

### Why inverted index + trie + CSR instead of just ripgrep?

- ripgrep is perfect for cold start (no index needed)
- But 5 consecutive queries = 5 full scans
- With index: 5 queries = 5 mmap lookups (~60ms total vs ~1000ms)
- The disk index "warms up" over the session via OS page cache
