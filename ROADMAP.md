# Roadmap

Planned features for Topo, roughly in priority order.

## Next Up

### `topo impact <file>` — Change Impact Analysis

Show all files affected by a change, using the import graph.

Given a file path, walk the import graph outward to find every file that directly or transitively depends on it. Each impacted file is annotated with its PageRank score (a measure of how central it is to the codebase).

```
$ topo impact src/auth/session.rs
src/auth/session.rs (changed)
├── src/auth/middleware.rs      PR: 0.034
├── src/api/routes.rs           PR: 0.028
│   ├── src/api/handlers.rs     PR: 0.021
│   └── src/api/tests.rs        PR: 0.009
├── src/main.rs                 PR: 0.041
└── 12 more files...
```

### `topo map` — Architecture Overview

High-level architecture overview from the import graph.

Shows the most central files in a codebase, module clusters, and dependency structure — all derived from the same import graph that powers Topo's scoring.

```
$ topo map
Top 10 most central files:
  1. src/main.rs                    PR: 0.041
  2. src/lib.rs                     PR: 0.038
  3. src/auth/session.rs            PR: 0.034
  ...

Module clusters:
  [auth] src/auth/*.rs (6 files, 4 internal edges)
  [api]  src/api/*.rs  (8 files, 12 internal edges)
  ...
```

## Planned

### `topo diff <ref>` — Git Diff + Import Impact

Combine git diff with import graph analysis. For a given git ref, identify all changed files and their transitive impact through the import graph. Produces a risk-ranked summary showing which changes touch hub modules.

```
$ topo diff main
Changed files:
  src/auth/session.rs      PR: 0.034  impact: 15 files
  src/utils/logger.rs      PR: 0.008  impact: 2 files

Risk: MEDIUM — 1 hub file changed (session.rs imported by 15 files)
```

### Cross-Repo Indexing

Index multiple repositories and build a unified import graph across them. Useful for microservice architectures where imports cross repo boundaries (shared libraries, API contracts, proto files).

### Temporal Analysis

Track how a codebase evolves over time: which files are becoming more central, which modules are growing in coupling, and how the import graph changes across commits and releases.

## Contributing

All features above are open for contributions. See the linked issues for details and discussion.
