# syntax-lang - Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
> Master plan: ../../_strategy/LANG_COLLECTION.md
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

## v0.1.0 - Scaffold (DONE)
Compiles, CI green, structure correct, no domain logic.
- [x] Manifest, README, CHANGELOG, REPS, dual license, CI, deny, clippy, rustfmt.

## v0.2.0 - Core (THE HARD PART, NOT DEFERRED) (DONE)
A lossless CST with trivia - the substrate for formatters, LSP, and tree-sitter generation.
Dependencies (wires token 1, span 0.4) wired here. `Node`/`Element` tree, non-panicking
`Builder`, iterative `tokens`/`descendants`/`Drop`, zero-copy `text`. Reserved `serde`
feature dropped (addable later without a break).
Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested (losslessness, covering span, tiling, containment).

## v1.0.0 - API freeze
Public surface stable and frozen until 2.0.
- [ ] docs/API.md marked stable; SemVer promise recorded.
- [ ] Full test + benchmark suite green on all three platforms.
