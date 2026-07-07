<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>syntax-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [0.2.0] - 2026-07-01

The core, front-loaded: the scaffold becomes a working lossless CST. A parser drives a `Builder` to assemble a tree of nodes and leaf tokens; trivia is preserved as ordinary leaves, so the source can be reproduced from the tree. This was the hard part of the roadmap and the roadmap put it first.

### Added

- `Node<K>` — an interior node: kind, covering span (the union of its children's spans), and ordered `Element` children. Owns its children directly; no arena or handles.
- `Element<K>` — one child, either a nested `Node` or a leaf `Token`, with `span`, `kind`, `as_node`, `as_token`, `is_node`, `is_token` accessors.
- `Builder<K>` — assembles a tree from `start_node` / `token` / `finish_node`, promoting the outermost node to the root at `finish`.
- `BuildError` — non-panicking report of builder misuse (`EmptyTree`, `UnclosedNodes`, `UnbalancedFinish`, `TokenOutsideNode`, `MultipleRoots`), returned from `finish`.
- Iterative traversal: `Node::tokens` (every leaf in source order — the lossless stream), `Node::descendants` (every node, pre-order), and `Node::children` / `child_nodes` / `child_tokens` (the direct level).
- `Node::text` — zero-copy reconstruction of a node's source by slicing the original string with its span.
- Iterative `Drop` for `Node`, so a pathologically deep tree is freed without overflowing the call stack.
- Wires `token-lang` (`Token`, `TokenKind`, `Symbol`) and `span-lang` (`Span`, `Spanned`), re-exported for downstream use.
- Behavioural, property, and doc tests covering losslessness, covering-span, tiling, containment, error paths, and stack safety at depth; Criterion benchmarks for build and traversal.

### Changed

- The reserved, unimplemented `serde` feature and its optional dependency are removed rather than carried unused toward the frozen 1.0 surface (addable later without a breaking change).

### Fixed

- `Cargo.toml` keyword and category arrays were unquoted (invalid TOML); the crate now parses and builds. `clippy.toml` MSRV aligned with `Cargo.toml` (1.85).

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/syntax-lang/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jamesgober/syntax-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/syntax-lang/releases/tag/v0.1.0
