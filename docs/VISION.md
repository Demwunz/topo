# Topo Vision & Roadmap

## Thesis

Topo is a codebase understanding engine. File selection for LLMs is the first
application — the same index can answer many other questions about your code.

## What Topo Understands Today

| Primitive | What it captures |
|-----------|-----------------|
| File scanner | Every file: language, role, size, SHA-256 hash |
| BM25F index | Term frequencies across filename, body, and symbol fields |
| AST chunker | Functions, types, impls, imports with names and line ranges |
| Import graph | Cross-file import/require/use relationships (18 languages) |
| PageRank | Structural centrality scores from the import graph |
| Incremental cache | SHA-256 fingerprinting — only re-index what changed |

## Current Applications

- **CLI** (`topo quick`, `query`, `explain`) — file selection for any task
- **Claude Code hooks** — automatic context injection on every prompt
- **MCP server** — exposes indexing and querying to any MCP client
- **AI assistant setup** (`topo init`) — generates instruction files for
  Claude Code, Cursor, Copilot

## Future Applications

### Near-term (leverage existing index)

1. **`topo impact <file>`** — walk the import graph outward to show which
   files are affected by a change. Use case: PR review scoping, smart test
   selection, refactoring safety nets.

2. **`topo search <symbol>`** — BM25F search with symbol-field boosting.
   Returns specific chunks (function at line 42, type at line 89), not just
   files. A fast local code search that works without an IDE.

3. **`topo map`** — render the import graph + PageRank as an architectural
   overview. Top-10 most central files, module clusters, entry points. Instant
   onboarding for new developers or LLMs.

### Medium-term (new index data)

4. **`topo diff <ref>`** — combine git diff with import-graph impact analysis.
   "These files changed, here are the ripple effects." Feed into CI for
   targeted test selection.

5. **`topo watch`** — keep the index hot with filesystem watching. Auto-update
   on file changes for real-time queries.

6. **Cross-repo index** — index multiple repos into a unified queryable store.
   Query across microservice or polyrepo architectures.

### Long-term (richer understanding)

7. **Full symbol cross-references** — tree-sitter enrichment pass on
   winning files to build definition/usage maps.

8. **Lightweight embeddings** — local embedding model for semantic similarity
   when BM25F keyword matching falls short.

## Roadmap

| Phase | Focus | Key deliverables |
|-------|-------|-----------------|
| Current | File selection | CLI, hooks, MCP, 18 languages |
| Next | Expose the engine | `topo impact`, `topo search`, `topo map` |
| Later | Richer intelligence | `topo diff`, `topo watch`, cross-repo |
| Future | Deep understanding | Symbol cross-refs, embeddings |

## Design Principles

- **Build primitives, not features** — the index is the product; commands
  are interfaces
- **Same index, new queries** — ship capabilities by writing new query types
  over existing data before adding new data structures
- **Humans and machines** — every capability should be useful from both a
  CLI and an MCP/API surface
- **Stay local** — no cloud, no API keys, no network calls. The codebase
  never leaves the machine
- **Incremental by default** — never re-process unchanged data
