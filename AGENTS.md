# AGENTS.md

See `CLAUDE.md` and per-crate `CLAUDE.md` files for all development guidelines. This project uses hierarchical CLAUDE.md files:

- `/CLAUDE.md` — Workspace-wide build, test, and architecture conventions
- `/crates/aliasman-core/CLAUDE.md` — Core library rules, error handling, trait patterns, provider implementation
- `/crates/aliasman-cli/CLAUDE.md` — CLI binary structure, clap patterns, command dispatch
- `/crates/aliasman-web/CLAUDE.md` — Web frontend with Axum, Askama templates, HTMX patterns
