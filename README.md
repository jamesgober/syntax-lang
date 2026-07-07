<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>syntax-lang</b>
    <br>
    <sub><sup>CONCRETE SYNTAX TREE</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/syntax-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/syntax-lang"></a>
    <a href="https://crates.io/crates/syntax-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/syntax-lang?color=%230099ff"></a>
    <a href="https://docs.rs/syntax-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/syntax-lang"></a>
    <a href="https://github.com/jamesgober/syntax-lang/actions"><img alt="CI" src="https://github.com/jamesgober/syntax-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        <strong>syntax-lang</strong> is a lossless <b>concrete syntax tree</b> (CST) for the <code>-lang</code> language-construction family &mdash; the substrate a <b>formatter</b>, a <b>language server</b>, or a tree-sitter-style generator builds on. Unlike an abstract syntax tree, a CST keeps <em>everything</em>: whitespace and comments ride in the tree as ordinary leaf tokens, so the source can be reproduced from the tree byte for byte.
    </p>
    <p>
        The crate owns no grammar. A language brings its own kind type; syntax-lang supplies the tree shape, the <code>Builder</code> a parser drives, and the traversals over the result. Leaves store <b>spans</b>, not copies of the text, so the tree stays compact and reconstruction is a <b>zero-copy</b> borrow of the original source. Building, walking, and freeing a tree are all <b>iterative</b>, so a pathologically deep tree never overflows the call stack.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition). <code>no_std</code> with <code>alloc</code>; <code>#![forbid(unsafe_code)]</code>.
    </p>
    <blockquote>
        <strong>Status: pre-1.0, in active development.</strong> The public API is being designed across the 0.x series and frozen at <code>1.0.0</code>. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

## Performance First

Latest local Criterion means (`cargo bench`, Windows x86_64, Rust stable). A tree of `n` one-byte tokens grouped in runs of eight:

- **Build** (parser drive → finished tree): ~14.6 ns per token (`build/4096` ≈ 59.7 µs).
- **Token walk** (`tokens()` over every leaf): a single allocation-free pass, linear in leaf count.
- **Node walk** (`descendants()` over every node): a single allocation-free pass, linear in node count.

Both traversals keep their work stack on the heap and borrow the tree, so neither allocates per element and neither recurses. Reconstructing a node's text (`node.text(source)`) is a single sub-slice of the source string — no allocation, no copy.

<br>
<hr>
<br>

## Features

- **Lossless.** Trivia (whitespace, comments) is preserved as leaf tokens in source order; the tree reproduces the source exactly.
- **Grammar-agnostic.** One kind type `K` covers both node and token kinds (the rowan model); the tree is generic over any `K` and needs no trait bound.
- **Zero-copy text.** Tokens carry spans, not owned strings. `node.text(source)` borrows a sub-slice of the source.
- **Stack-safe at any depth.** `Builder`, `tokens()`, `descendants()`, and `Drop` are all iterative — a 200,000-level tree is built, walked, and freed without a stack overflow.
- **Non-panicking builder.** Structural misuse (unbalanced close, token at the root, a second root) is reported as a `BuildError` from `finish()`, never a panic.
- **`no_std`.** Needs only `alloc`. The default `std` feature forwards to the token and span crates.

<br>
<hr>

## Installation

```toml
[dependencies]
syntax-lang = "0.2"
```

Or from the terminal:

```bash
cargo add syntax-lang
```

<br>

## Example

A language defines one kind type for both nodes and tokens, then a parser drives the `Builder` to produce the tree.

```rust
use syntax_lang::{Builder, Span, Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    // node kinds
    Root,
    Sum,
    // token kinds
    Num,
    Plus,
    Space,
}

impl TokenKind for Kind {
    fn is_trivia(&self) -> bool {
        matches!(self, Kind::Space)
    }
}

// Build the CST for `1 + 2`, keeping the spaces as trivia.
let mut b = Builder::new();
b.start_node(Kind::Root);
b.start_node(Kind::Sum);
b.token(Token::new(Kind::Num, Span::new(0, 1)));
b.token(Token::new(Kind::Space, Span::new(1, 2)));
b.token(Token::new(Kind::Plus, Span::new(2, 3)));
b.token(Token::new(Kind::Space, Span::new(3, 4)));
b.token(Token::new(Kind::Num, Span::new(4, 5)));
b.finish_node(); // close Sum
b.finish_node(); // close Root

let root = b.finish().expect("balanced");

// Lossless: the tree reproduces the source, trivia and all.
assert_eq!(root.text("1 + 2"), Some("1 + 2"));
assert_eq!(root.tokens().count(), 5);

// A formatter walks the significant tokens and skips the trivia.
let significant = root.tokens().filter(|t| !t.is_trivia()).count();
assert_eq!(significant, 3);
```

<br>

## Walking a tree

- `node.tokens()` — every leaf token in source order (the lossless stream, trivia included).
- `node.descendants()` — every node in pre-order (a parent before its descendants).
- `node.children()` / `child_nodes()` / `child_tokens()` — the direct level.
- `node.text(source)` — the exact source slice this node covers, or `None` if the span lies outside `source`.

All of these are iterative and allocation-free.

<br>
<hr>

## API Overview

For the complete reference with parameter notes and multiple examples per item, see [`docs/API.md`](./docs/API.md).

- [`Node`](./docs/API.md#node) — an interior node: kind, covering span, ordered children, and the traversals.
- [`Element`](./docs/API.md#element) — one child: a nested `Node` or a leaf `Token`.
- [`Builder`](./docs/API.md#builder) — assembles a tree from `start_node` / `token` / `finish_node` calls.
- [`BuildError`](./docs/API.md#builderror) — why a builder could not produce a tree.
- Re-exports: [`Token`, `TokenKind`, `Symbol`](./docs/API.md#re-exports) (token-lang) and [`Span`, `Spanned`](./docs/API.md#re-exports) (span-lang).

<br>
<hr>

## Design notes

- **Why spans, not text.** A CST that owns copies of every lexeme duplicates the whole source. syntax-lang stores each token's byte range instead; the source string is the single source of truth, so the tree is compact and `node.text(source)` is a borrow. This is the highest-performance option and matches what [`token-lang`](https://crates.io/crates/token-lang) already carries on a token.
- **Why one kind type.** Nodes and tokens share `K` so generic tooling can ask a child's kind without first branching on node-vs-token, and a language declares its whole vocabulary in one `enum`.
- **Why iterative everything.** A parser can emit an arbitrarily deep tree from adversarial input (a long run of open delimiters). Recursive drop glue would overflow the stack on such a tree, turning untrusted input into a crash. Traversal and teardown keep their stack on the heap so depth is bounded by memory, not the call stack.

<br>
<hr>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
