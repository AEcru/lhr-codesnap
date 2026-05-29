# CodeSnap — Zero-Config Semantic Code Intelligence for AI Coding Agents

> Instant code understanding via Claude Code Skill — no MCP server, no background daemon, no warmup.
> One skill file + one CLI binary. Zero overhead when idle, instant when needed.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![Binary Size](https://img.shields.io/badge/binary-~5MB-brightgreen.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue.svg)]()

[中文](README.md) | English

---

## What It Is

When an AI coding agent explores a codebase, it spawns sub-agents that grep, glob,
and Read files — burning tokens on every tool call.

CodeSnap takes a different approach: a lightweight CLI tool + Claude Code Skill
that gives AI agents instant semantic understanding of any codebase. No MCP server,
no background process, no configuration files.

How it works:
- **Pre-built disk index** — mmap'd binary format, zero-copy reads, 5ms startup
- **Purpose-built data structures** — inverted index for search, CSR for call graphs,
  trie for prefix matching, Roaring Bitmap for impact analysis
- **Self-healing** — every query compares file mtime with index, auto re-parses changes
- **ripgrep fallback** — when index isn't ready yet, degrades gracefully

## Key Advantages

### vs Traditional MCP Server Approach

| Dimension | Traditional MCP | CodeSnap |
|-----------|----------------|----------|
| Architecture | MCP Server (persistent daemon) | Skill + CLI (on-demand) |
| First use | Wait for pre-built index | Instant, progressive indexing |
| Memory (idle) | 100-300MB (always on) | **0MB** |
| Memory (active) | 100-300MB | 30-80MB |
| Binary size | ~80MB (bundled runtime) | **~5MB** (Rust musl static) |
| Search engine | Generic full-text engine | **3D Inverted Index + Trie** (code-aware) |
| Call graph | Generic relational DB + JOIN | **CSR compressed graph** (L3 cache resident) |
| Impact analysis | Recursive SQL queries | **Roaring Bitmap** bitwise ops |
| Storage engine | Generic B-Tree database | **LSM Tree** (write-optimized) |
| Index freshness | File watcher + debounce | **mtime self-check**, always current |
| Cross-session | In-memory, re-warm on restart | **Disk mmap**, instant on reconnect |
| Setup | Edit JSON config files | **Zero config** — copy skill directory |
| Best for | Heavy continuous use, multi-tool | **On-demand**, exits when done |

## Quick Start

### 1. Install CLI

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.ps1 | iex

# Or via Cargo
cargo install codesnap
```

### 2. Install the Skill

Copy the `.claude/skills/lhr-codesnap/` directory to your project's
`.claude/skills/` directory:

```
your-project/
└── .claude/
    └── skills/
        └── lhr-codesnap/
            ├── SKILL.md              # AI trigger entry point
            └── references/            # Detailed docs (loaded on-demand)
                ├── commands.md        # Full command reference
                └── architecture.md    # Architecture design details
```

### 3. Initialize the Index

```bash
cd your-project
codesnap init
```

A few seconds for small projects, 1-2 minutes for large ones. After init, all
queries are instant (sub-millisecond).

### 4. Start Using

In your Claude Code session, the AI will automatically invoke CodeSnap when it
needs to understand code. Or ask directly:

> "Find where UserService is defined"
> "Trace the call path from login() to saveToDatabase()"
> "What breaks if I change TokenUtil?"

## Commands

| Command | What it does | Example |
|---------|-------------|---------|
| `codesnap init [path]` | Build full index | `codesnap init` |
| `codesnap find <name>` | Locate symbol definition | `codesnap find "UserService"` |
| `codesnap callers <name>` | Find callers of a symbol | `codesnap callers "validateToken"` |
| `codesnap callees <name>` | Find what a symbol calls | `codesnap callees "login"` |
| `codesnap impact <name>` | Full change impact radius | `codesnap impact "TokenUtil"` |
| `codesnap trace <a> <b>` | Find call path from A to B | `codesnap trace "Order.create" "DB.save"` |
| `codesnap context <task>` | Build relevant code context | `codesnap context "fix login bug"` |
| `codesnap status` | Index health + statistics | `codesnap status` |
| `codesnap check` | Verify index freshness | `codesnap check` |

## Supported Languages

| Language | Extensions | Support Level |
|----------|-----------|---------------|
| TypeScript / JavaScript | `.ts` `.tsx` `.js` `.jsx` `.mjs` | Full |
| Python | `.py` | Full |
| Go | `.go` | Full |
| Rust | `.rs` | Full |
| Java | `.java` | Full |
| C# | `.cs` | Full |
| PHP | `.php` | Full |
| Ruby | `.rb` | Full |
| C / C++ | `.c` `.h` `.cpp` `.hpp` `.cc` | Full |
| Swift | `.swift` | Full |
| Kotlin | `.kt` `.kts` | Full |
| Dart | `.dart` | Full |
| Vue | `.vue` | Full |
| Svelte | `.svelte` | Full |
| Lua | `.lua` `.luau` | Full |

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Claude Code Skill                          │
│   AI encounters code question → loads skill → runs codesnap   │
├──────────────────────────────────────────────────────────────┤
│                    codesnap CLI (Rust)                        │
│                                                              │
│   ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│   │ Trie     │ Inverted │ CSR Call │ Roaring  │ Bloom    │  │
│   │ Prefix   │ Index    │ Graph    │ Bitmap   │ Filter   │  │
│   │ Matching │ 3D Symbol│ Multi-   │ Impact   │ Cold     │  │
│   │          │ Search   │ Level    │ Analysis │ Start    │  │
│   └──────────┴──────────┴──────────┴──────────┴──────────┘  │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │      LSM Tree Disk Index (mmap zero-copy)             │  │
│   │   MemTable → L0 → L1 → ... → Ln                      │  │
│   │   Self-check: mtime comparison → incremental reparse  │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │   ripgrep Fallback (when index not yet ready)         │  │
│   └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### Core Data Structures

| Structure | Purpose | Query Complexity | vs SQLite |
|-----------|---------|-----------------|-----------|
| **3D Inverted Index** | Symbol→location, Type→symbols, File→symbols | O(1) | 50-100x faster |
| **Radix Trie** | Prefix/infix symbol search | O(k), k=prefix len | 5-10x faster |
| **CSR Graph** | Call graph storage & traversal | O(degree) sequential | 1000x faster |
| **Roaring Bitmap** | Impact analysis set operations | O(n/64) bitwise | 1000-10000x faster |
| **LSM Tree** | Incremental index writes | O(1) append | 10-50x write speed |
| **Bloom Filter** | Cold start file exclusion | O(1) | N/A |

Detailed architecture: [docs/architecture.md](docs/architecture.md).

## Why Skill Instead of MCP Server?

| MCP Server | Skill + CLI |
|------------|-------------|
| Edit `~/.claude.json` config | **Zero config** — copy skill file |
| Persistent daemon, 100MB+ RAM | **On-demand**, 0MB when idle |
| In-memory cache, lost on restart | **Disk mmap**, survives sessions |
| MCP logs hard to debug | **stdout visible**, easy debugging |
| Connection drop, process hang risks | **Exits when done**, zero management |
| Cross-tool (Cursor, Codex, etc.) | Claude Code only (for now) |

## Project Structure

```
codesnap/
├── README.md                   # Chinese (primary display)
├── README.en.md                # English
├── LICENSE                     # MIT
├── CLAUDE.md                   # Development guide
├── .gitignore
├── .claude/
│   ├── rules/
│   │   └── project-rules.md    # Development rules
│   └── skills/
│       └── lhr-codesnap/
│           ├── SKILL.md         # Skill entry point
│           └── references/      # Reference docs
│               ├── commands.md
│               └── architecture.md
├── src/                        # CLI source (Rust)
│   ├── main.rs
│   ├── index/                  # Index construction
│   ├── query/                  # Query engine
│   ├── sync/                   # Incremental sync
│   └── output/                 # Formatting
├── docs/
│   └── architecture.md         # Architecture design
└── tests/                      # Tests
```

## Roadmap

- [x] Architecture design & data structure selection
- [ ] Rust CLI core implementation
- [x] Skill file (SKILL.md + references/)
- [ ] One-line install script
- [ ] 20+ language tree-sitter support
- [ ] Cross-platform CI/CD builds
- [ ] Benchmark framework

## Contributing

Issues and PRs welcome! Read [CLAUDE.md](CLAUDE.md) for development guidelines.

## License

MIT License — see [LICENSE](LICENSE).

---

**Made for AI coding agents — zero overhead when idle, instant when needed.**
