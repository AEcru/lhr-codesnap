# Commands Reference

## `codesnap find <name>`

Locate the canonical definition of a symbol.

Options:
- `--kind <type>` — Filter by symbol kind: `class`, `function`, `method`,
  `interface`, `enum`, `type`, `variable`, `constant`
- `--file <pattern>` — Restrict search to files matching glob pattern
- `--json` — Output as JSON lines for programmatic consumption

Output format:
```
kind name visibility → file:line
class UserService public → src/auth/UserService.java:12
method login public → src/auth/UserService.java:45
```

## `codesnap callers <name>`

Find all symbols that call the given symbol.

Options:
- `--depth <n>` — How many levels of transitive callers to trace (default: 1)
- `--limit <n>` — Max results (default: 50)
- `--test-only` — Only show test files
- `--json` — Output as JSON lines

Output format:
```
caller_kind caller_name → file:line
method AuthController.handleLogin → src/controller/AuthController.java:34
function processLogin → src/services/login.js:12
```

## `codesnap callees <name>`

Find all symbols called by the given symbol.

Options:
- `--depth <n>` — Transitive callee depth (default: 1)
- `--limit <n>` — Max results (default: 50)
- `--external` — Include calls to external/third-party symbols
- `--json` — Output as JSON lines

Output format (same as callers):
```
callee_kind callee_name → file:line
method TokenUtil.validate → src/utils/TokenUtil.java:23
method UserRepository.findById → src/repo/UserRepository.java:8
```

## `codesnap impact <name>`

Analyze the full change impact radius. Traces transitive callers and groups
affected files by module.

Options:
- `--depth <n>` — Max call depth to trace (default: 3)
- `--test-only` — Only show affected test files
- `--json` — Output as JSON lines

Output format:
```
## Direct callers (N files)
src/controller/AuthController.java
src/services/UserService.java

## Transitive callers (M files)
src/api/middleware.ts
src/api/routes.ts

## Affected tests (K files)
src/__tests__/AuthController.test.java ⚠
src/__tests__/UserService.test.java ⚠
```

## `codesnap trace <from> <to>`

Find the shortest call path between two symbols.

Options:
- `--max-depth <n>` — Max search depth (default: 7)
- `--all-paths` — Show all paths instead of just the shortest
- `--json` — Output as JSON lines

Output format:
```
create (OrderController.java:34)
  → validateOrder (OrderValidator.java:12)
    → processPayment (PaymentService.java:56)
      → save (OrderRepository.java:23)

4 hops, 3 intermediate nodes
```

## `codesnap context <task>`

Build a targeted context map for a specific task. Uses the task description to
identify entry points, related symbols, route mappings, and key call edges.

Options:
- `--max-nodes <n>` — Max symbols to include (default: 30)
- `--include-code` — Include source code snippets
- `--format <markdown|json>` — Output format (default: markdown)

Output format:
```
## Entry Points
- POST /api/auth/login → AuthController.login (AuthController.java:34)

## Core Symbols
- AuthService.authenticate — validates credentials, returns JWT
- TokenUtil.generateToken — creates signed JWT with expiry
- UserRepository.findByEmail — database lookup by email

## Key Call Edges
- AuthController.login → AuthService.authenticate
- AuthService.authenticate → UserRepository.findByEmail
- AuthService.authenticate → TokenUtil.generateToken
```

## `codesnap status`

Show index health, coverage, and statistics.

Output format:
```
## Index Status
- State: Ready
- Symbols indexed: 2,847
- Files indexed: 312
- Disk size: 1.2 MB
- Last indexed: unknown

## Pending Files
- src/auth/NewService.java
```

## `codesnap check`

Verify index freshness. Returns exit code 0 if index is up to date, 3 if stale.

Options:
- `--fix` — Automatically re-index stale files instead of just reporting
- `--json` — Output as JSON lines

## `codesnap init [path]`

Build the full index. Must be run once per project.

Options:
- `--force` — Rebuild index even if one already exists
- `--quiet` — Suppress progress output

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success or no results found |
| 1 | Error (index not found, invalid args, etc.) |
| 3 | Index is stale — used by `check` for CI integration |
