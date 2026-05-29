---
paths: ["**/*.rs", "**/*.md", "**/*.sh", "**/*.ps1"]
---

# CodeSnap Development Rules

## CRITICAL

1. **Binary must stay under 8MB** — strip symbols, LTO, opt-level="z",
   panic="abort", single codegen unit for release
2. **Index format must be backward-compatible** — versioned binary format,
   new sections appended, old readers skip unknown sections
3. **Every query must self-check freshness** — mtime comparison before
   returning results, auto-trigger incremental reparse on mismatch
4. **No network, no telemetry, no phone-home** — 100% local always
5. **Cross-platform** — Windows 10+, macOS 12+, Linux kernel 5.x+, x64 + arm64

## Rust Code Standards

- Rust 2024 edition
- No unsafe except for mmap and FFI to tree-sitter C libraries
- Single binary, statically linked (musl on Linux)
- `anyhow` for application errors, `thiserror` for library errors
- `tracing` for structured logging, `env_logger` for CLI output
- All public functions must have doc comments with examples

## Skill File Standards

- SKILL.md must stay under 150 lines
- Reference files under 500 lines each, with table of contents
- Description frontmatter is the primary trigger — be thorough
- Use imperative/infinitive form in skill body
- Commands table must match CLI exactly
- Update skill when CLI interface changes

## Index Format Standards

- Versioned binary format: magic bytes + version + section table
- CRC32 per section for corruption detection
- File paths stored as interned u32 IDs via string table
- Symbol names use shared radix trie for prefix compression
- Append-only writes for durability; compaction on `init --force`
- Backward-compatible: new sections appended, old readers skip unknown

## Documentation Standards

- README.md (Chinese) is the primary display document
- README.en.md (English) is the secondary document
- Both READMEs must stay in sync — same structure, same information
- CLI changes must update: README.md, README.en.md, SKILL.md,
  references/commands.md
- Architecture changes must update: CLAUDE.md, references/architecture.md

## Testing Standards

- Unit tests for each data structure module
- Integration tests: index → query → verify result against known source
- Snapshot tests for output formatting
- Cross-platform CI: Windows, macOS, Linux on x64 and arm64
- Benchmark suite: compare against ripgrep baseline for each query type

## Commit Standards

- Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- Each commit must compile and pass tests
- No WIP commits on main branch
