---
name: lhr-codesnap
description: >
  Instant semantic code intelligence for any codebase. Use when you need to
  find symbol definitions, trace call chains, analyze change impact, or
  understand unfamiliar code. Triggers on queries like "where is X defined",
  "who calls X", "what does X call", "impact of changing X", "how does X
  reach Y", "find the definition", "trace the call chain", "analyze the
  impact", "show me the callers", or any task requiring code exploration
  that would otherwise use grep/glob loops.
---

# CodeSnap — Semantic Code Intelligence

Use the `codesnap` CLI for instant code understanding via a pre-built disk
index with purpose-optimized data structures.

## Bootstrap (run first)

Before using any codesnap command, check if it is installed. If not, install
it automatically:

```bash
# Check availability
which codesnap 2>/dev/null || where codesnap 2>/dev/null || echo "NOT_INSTALLED"
```

If NOT installed:

```bash
# Primary: install from GitHub source (requires Rust/cargo)
cargo install --git https://github.com/AEcru/lhr-codesnap.git codesnap

# Fallback: clone & build manually if cargo-install-git is unavailable
git clone https://github.com/AEcru/lhr-codesnap.git /tmp/lhr-codesnap
cd /tmp/lhr-codesnap && cargo build --release
cp target/release/codesnap ~/.cargo/bin/
```

If Rust/cargo is NOT available, fall back to ripgrep for this session and
tell the user to install Rust: https://rustup.rs

If you are INSIDE the codesnap source repo itself, use `cargo run --` as
a shortcut (no global install needed):

```bash
cargo run --release -- <command> <args>
```

After installation, run `codesnap init` in the target project to build the
index. This is required once per project (a few seconds to 1-2 minutes).

## Quick Reference

| Command | Purpose | Example |
|---------|---------|---------|
| `codesnap init` | Build index (first time) | `codesnap init` |
| `codesnap find <name>` | Locate symbol definition | `codesnap find "UserService"` |
| `codesnap callers <name>` | Who calls this symbol | `codesnap callers "validateToken"` |
| `codesnap callees <name>` | What this symbol calls | `codesnap callees "login"` |
| `codesnap impact <name>` | Full change impact radius | `codesnap impact "TokenUtil"` |
| `codesnap trace <a> <b>` | Find call path from A to B | `codesnap trace "Order.create" "DB.save"` |
| `codesnap context <task>` | Build relevant code context | `codesnap context "fix login bug"` |
| `codesnap status` | Index health + coverage stats | `codesnap status` |
| `codesnap check` | Verify index freshness | After git pull or branch switch |

See [references/commands.md](references/commands.md) for full flags, options,
and output format details.

## Guiding Rules

### 1. Prefer `codesnap find` over grep for symbol definitions

Grep returns raw text matches (comments, strings, references). `codesnap find`
returns the canonical definition with type, visibility, and location.

```
# DO:
codesnap find "UserService"

# DON'T (unless codesnap index doesn't exist yet):
rg "class UserService"
```

### 2. Check impact before editing shared code

Before changing a utility function, service method, or base class, assess the
blast radius. The tool traces transitive callers and groups affected files by
module, marking test files.

```
codesnap impact "TokenUtil.generateToken"
```

### 3. Use `codesnap trace` for "how does X reach Y"

Instead of manually grepping each hop, trace the full call path in one command.
The engine walks the call graph at multiple granularity levels (module → file →
function) for efficient path finding.

```
codesnap trace "OrderController.create" "OrderRepository.save"
```

### 4. Use `codesnap context` to get oriented in unfamiliar code

Build a targeted context map faster than spawning Explore sub-agents. Returns
entry points, related symbols, route mappings, and key call edges.

```
codesnap context "user registration flow"
```

### 5. Trust the auto-sync — no manual refresh needed

Every query internally compares file mtimes with the index. If a file changed,
it's incrementally re-parsed before results return. No need to run
`codesnap check` before every query.

### 6. Install once, init per project

`codesnap` is installed once globally. Then run `codesnap init` once per
project. After that, all queries are instant.

## Limitations

- **Cold start**: Without an index, queries fall back to ripgrep (slower but
  always works). The CLI will tell you to run `codesnap init` first.
- **Definition only**: `codesnap find` returns the definition site, not all
  references. Use `codesnap callers` to find reference sites.
- **Static analysis**: Dynamic dispatch, reflection, and runtime DI are not
  traced. The analysis is AST-based.
- **Git-aware but not git-dependent**: Works in non-git projects, but
  `.gitignore` is honored for file exclusion.

## Architecture

See [references/architecture.md](references/architecture.md) for data structure
design, performance characteristics, and comparison with traditional MCP solutions.
