<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>syntax-lang</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
        <span>&nbsp;│&nbsp;</span>
        <a href="../dev/ROADMAP.md" title="Roadmap"><b>ROADMAP</b></a>
    </sup>
</div>
<br>

> **Status: stable as of `1.0.0`.** The surface below is frozen under Semantic Versioning — no breaking change before `2.0`, additions in minor releases, MSRV (Rust 1.85) rising only in a minor. See [Stability & SemVer](#stability--semver).

A lossless concrete syntax tree (CST) with trivia — the substrate for formatters, language servers, and tree-sitter-style generation.

## Table of Contents

- **[Installation](#installation)**
- **[Model](#model)**
- **[Quick Start](#quick-start)**
- **[Public API](#public-api)**
  - [`Node`](#node)
  - [`Element`](#element)
  - [`Builder`](#builder)
  - [`BuildError`](#builderror)
  - [Re-exports](#re-exports)
- **[Invariants](#invariants)**
- **[Feature Flags](#feature-flags)**
- **[Stability & SemVer](#stability--semver)**

<br><br>

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
syntax-lang = "0.2"
```

Or from the terminal:

```bash
cargo add syntax-lang
```

MSRV: Rust 1.85 (2024 edition). The crate is `no_std` (needs `alloc`); the default `std` feature forwards to the token and span crates.

<hr>
<br>

## Model

A tree is [`Node`](#node)s down to [`Token`](#re-exports) leaves. Each node carries a **kind**, the **[`Span`](#re-exports)** of source it covers, and an ordered list of [`Element`](#element) children — each child a nested node or a leaf token.

Node kinds and token kinds share one type `K`: a language defines a single `enum` with both composite variants (`Expr`, `Root`) and lexical ones (`Ident`, `Plus`, `Whitespace`), and implements [`TokenKind`](#re-exports) on it to mark trivia and the end marker. The tree is generic over any `K` and needs no trait bound; `TokenKind` is only required when a caller wants the trivia queries.

**Lossless** means nothing is discarded. Trivia rides as ordinary leaf tokens in source order, so slicing the source by a node's span — or concatenating its [`tokens`](#nodetokens) — reproduces exactly the source that node came from. Tokens store spans rather than copies of the text, so reconstruction is a zero-copy borrow.

<hr>
<br>

## Quick Start

```rust
use syntax_lang::{Builder, Span, Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Root,
    Sum,
    Num,
    Plus,
    Space,
}

impl TokenKind for Kind {
    fn is_trivia(&self) -> bool {
        matches!(self, Kind::Space)
    }
}

let mut b = Builder::new();
b.start_node(Kind::Root);
b.start_node(Kind::Sum);
b.token(Token::new(Kind::Num, Span::new(0, 1)));
b.token(Token::new(Kind::Space, Span::new(1, 2)));
b.token(Token::new(Kind::Plus, Span::new(2, 3)));
b.token(Token::new(Kind::Space, Span::new(3, 4)));
b.token(Token::new(Kind::Num, Span::new(4, 5)));
b.finish_node();
b.finish_node();

let root = b.finish().expect("balanced");
assert_eq!(root.text("1 + 2"), Some("1 + 2"));
assert_eq!(root.tokens().filter(|t| !t.is_trivia()).count(), 3);
```

<hr>
<br>
<a href="#top">&uarr; <b>TOP</b></a>
<br>

## Public API

### `Node`

`Node<K>` — an interior node of the tree: a kind, the covering span, and ordered children.

A node owns its children directly, so a whole tree is a single owned value with no arena or handle bookkeeping. Its covering span is the union of its children's spans, computed once when the node is built. Derives `Clone`, `Debug`, `PartialEq`, `Eq` (these recurse with tree depth); traversal and `Drop` are iterative and safe at any depth.

**Constructors**

- `Node::new(kind: K, children: Vec<Element<K>>) -> Node<K>`
  - `kind` — the node's kind (shared kind type `K`).
  - `children` — the ordered children, in source order. A childless node reports an empty span at offset `0`; build empty nodes through a [`Builder`](#builder) instead, which positions them at the current stream cursor.
  - Returns a node whose span is the union of the children's spans.

```rust
use syntax_lang::{Element, Node, Span, Token};

let node = Node::new(
    "add",
    vec![
        Element::Token(Token::new("num", Span::new(0, 1))),
        Element::Token(Token::new("plus", Span::new(2, 3))),
        Element::Token(Token::new("num", Span::new(4, 5))),
    ],
);
assert_eq!(node.kind(), &"add");
assert_eq!(node.span(), Span::new(0, 5));
```

**Accessors**

- `kind(&self) -> &K` — borrows the node's kind.
- `span(&self) -> Span` — the covering span (union of children, or empty for a childless node).
- `is_empty(&self) -> bool` — whether the node has no children.
- `len(&self) -> usize` — the number of direct children (nodes and tokens).

```rust
use syntax_lang::{Element, Node, Span, Token};

let n = Node::new("n", vec![Element::Token(Token::new("t", Span::new(3, 8)))]);
assert_eq!(n.span(), Span::new(3, 8));
assert_eq!(n.len(), 1);
assert!(!n.is_empty());

let empty: Node<&str> = Node::new("empty", vec![]);
assert!(empty.is_empty());
```

<h4 id="nodechildren">Direct children</h4>

- `children(&self) -> impl Iterator<Item = &Element<K>>` — direct children, nodes and tokens interleaved in source order.
- `child_nodes(&self) -> impl Iterator<Item = &Node<K>>` — direct children that are nested nodes.
- `child_tokens(&self) -> impl Iterator<Item = &Token<K>>` — direct children that are leaf tokens.

```rust
use syntax_lang::{Element, Node, Span, Token};

let n = Node::new(
    "n",
    vec![
        Element::Token(Token::new("a", Span::new(0, 1))),
        Element::Node(Node::new("inner", vec![Element::Token(Token::new("b", Span::new(1, 2)))])),
    ],
);
assert_eq!(n.children().count(), 2);
assert_eq!(n.child_nodes().count(), 1);
assert_eq!(n.child_tokens().map(|t| *t.kind()).collect::<Vec<_>>(), ["a"]);
```

<h4 id="nodedescendants">Whole-tree traversal</h4>

- `descendants(&self) -> impl Iterator<Item = &Node<K>>` — this node and every node beneath it in pre-order (a parent before its descendants, children in source order). Iterative; safe on any depth.

```rust
use syntax_lang::{Element, Node, Span, Token};

let tree = Node::new(
    "root",
    vec![Element::Node(Node::new(
        "inner",
        vec![Element::Token(Token::new("t", Span::new(0, 1)))],
    ))],
);
let kinds: Vec<_> = tree.descendants().map(Node::kind).copied().collect();
assert_eq!(kinds, ["root", "inner"]);
```

<h4 id="nodetokens">Leaf token stream</h4>

- `tokens(&self) -> impl Iterator<Item = &Token<K>>` — every leaf token in the tree in source order — the lossless stream, trivia included. Iterative; safe on any depth.

```rust
use syntax_lang::{Element, Node, Span, Token};

let tree = Node::new(
    "root",
    vec![Element::Node(Node::new(
        "inner",
        vec![
            Element::Token(Token::new("a", Span::new(0, 1))),
            Element::Token(Token::new("b", Span::new(1, 2))),
        ],
    ))],
);
let leaves: Vec<_> = tree.tokens().map(|t| *t.kind()).collect();
assert_eq!(leaves, ["a", "b"]);
```

<h4 id="nodetext">Source reconstruction</h4>

- `text<'s>(&self, source: &'s str) -> Option<&'s str>` — slices `source` by this node's covering span, returning the exact text the node came from, or `None` if the span lies outside `source`.
  - `source` — the same source string the tree was built from.
  - Zero-copy: borrows a sub-slice rather than allocating. The `None` case guards against a mismatched or truncated string rather than panicking.

```rust
use syntax_lang::{Element, Node, Span, Token};

let n = Node::new(
    "call",
    vec![
        Element::Token(Token::new("id", Span::new(0, 1))),
        Element::Token(Token::new("(", Span::new(1, 2))),
        Element::Token(Token::new(")", Span::new(2, 3))),
    ],
);
assert_eq!(n.text("f()"), Some("f()"));
assert_eq!(n.text("f"), None); // span runs past the string
```

<br>
<a href="#top">&uarr; <b>TOP</b></a>
<br>

### `Element`

`Element<K>` — one child of a [`Node`](#node): either a nested node or a leaf token.

```rust,ignore
pub enum Element<K> {
    Node(Node<K>),
    Token(Token<K>),
}
```

Trivia is not a distinct variant — it rides as an ordinary `Token`, which is what makes the tree lossless. Derives `Clone`, `Debug`, `PartialEq`, `Eq`.

**Accessors**

- `span(&self) -> Span` — the node's covering span or the token's span.
- `kind(&self) -> &K` — the node's kind or the token's kind, without branching on which it is.
- `as_node(&self) -> Option<&Node<K>>` — the nested node, or `None` for a token.
- `as_token(&self) -> Option<&Token<K>>` — the leaf token, or `None` for a node.
- `is_node(&self) -> bool` / `is_token(&self) -> bool` — which variant this is.

```rust
use syntax_lang::{Element, Node, Span, Token};

let leaf: Element<&str> = Element::Token(Token::new("ident", Span::new(0, 3)));
assert!(leaf.is_token());
assert_eq!(leaf.kind(), &"ident");
assert_eq!(leaf.span(), Span::new(0, 3));
assert!(leaf.as_token().is_some());

let group = Element::Node(Node::new("expr", vec![leaf]));
assert!(group.is_node());
assert_eq!(group.span(), Span::new(0, 3));
assert!(group.as_node().is_some());
```

<br>
<a href="#top">&uarr; <b>TOP</b></a>
<br>

### `Builder`

`Builder<K>` — assembles a [`Node`](#node) tree from a stream of `start_node` / `token` / `finish_node` calls, the shape a parser drives as it recognises the grammar.

The builder keeps a stack of open nodes. `start_node` opens one, `token` appends a leaf to the innermost open node, and `finish_node` closes the innermost node and folds it into its parent. When the outermost node closes it becomes the root, which `finish` hands back.

Misuse is never a panic: the first structural fault is recorded and returned from `finish` as a [`BuildError`](#builderror), so a parser can drive the builder on its hot path without wrapping every call in a `Result`. Implements `Default`; `Debug` reports the open-node count, whether a root exists, and any recorded error.

**Construction**

- `Builder::new() -> Builder<K>` — an empty builder (`const`).
- `is_empty(&self) -> bool` — whether nothing has been built yet (no open nodes, no completed root).

**Building**

- `start_node(&mut self, kind: K)` — opens a new node of `kind`. Children added until the matching `finish_node` become its children. Starting a second node after the root has closed records [`BuildError::MultipleRoots`](#builderror).
- `token(&mut self, token: Token<K>)` — appends `token` as a leaf of the innermost open node. Pushing a token with no node open records [`BuildError::TokenOutsideNode`](#builderror).
- `finish_node(&mut self)` — closes the innermost open node, folding it into its parent, or promoting it to the root when it is outermost. Closing with no node open records [`BuildError::UnbalancedFinish`](#builderror).

**Finishing**

- `finish(self) -> Result<Node<K>, BuildError>` — consumes the builder and returns the finished root, or the first recorded error, or [`BuildError::UnclosedNodes`](#builderror) / [`BuildError::EmptyTree`](#builderror).

```rust
use syntax_lang::{Builder, Span, Token};

// Build `(1)` — a parenthesised literal — and read it back losslessly.
let mut b = Builder::new();
b.start_node("paren");
b.token(Token::new("(", Span::new(0, 1)));
b.start_node("lit");
b.token(Token::new("num", Span::new(1, 2)));
b.finish_node(); // close "lit"
b.token(Token::new(")", Span::new(2, 3)));
b.finish_node(); // close "paren"

let root = b.finish().expect("balanced");
assert_eq!(root.kind(), &"paren");
assert_eq!(root.span(), Span::new(0, 3));
assert_eq!(root.text("(1)"), Some("(1)"));
let kinds: Vec<_> = root.tokens().map(|t| *t.kind()).collect();
assert_eq!(kinds, ["(", "num", ")"]);
```

A deep tree is built and finished iteratively — no recursion, no stack overflow:

```rust
use syntax_lang::{Builder, Span, Token};

let mut b = Builder::new();
for _ in 0..100_000 {
    b.start_node("link");
}
b.token(Token::new("leaf", Span::new(0, 1)));
for _ in 0..100_000 {
    b.finish_node();
}
let root = b.finish().expect("balanced");
assert_eq!(root.descendants().count(), 100_000);
```

<br>
<a href="#top">&uarr; <b>TOP</b></a>
<br>

### `BuildError`

`BuildError` — why a [`Builder`](#builder) could not produce a tree. `#[non_exhaustive]`; derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`; implements `Display` and `core::error::Error`.

| Variant | Meaning | What to change |
|---|---|---|
| `EmptyTree` | `finish` called before any node was started | Start a root node before finishing. |
| `UnclosedNodes` | `finish` called while nodes were still open | Balance every `start_node` with a `finish_node`. |
| `UnbalancedFinish` | `finish_node` called with no open node | Remove the extra close, or add the missing `start_node`. |
| `TokenOutsideNode` | `token` called before any node was started | A tree's root must be a node; open one first. |
| `MultipleRoots` | a second root started after the first closed | A tree has one root; wrap them in an enclosing node. |

```rust
use syntax_lang::{BuildError, Builder};

// Closing a node that was never opened.
let mut b = Builder::<&str>::new();
b.finish_node();
assert_eq!(b.finish(), Err(BuildError::UnbalancedFinish));

// Nothing built at all.
assert_eq!(Builder::<&str>::new().finish(), Err(BuildError::EmptyTree));
```

<br>
<a href="#top">&uarr; <b>TOP</b></a>
<br>

### Re-exports

So a downstream defining and walking a CST can name the token and position types this crate is built on without also depending on `token-lang` and `span-lang` directly:

- **`Token<K>`** — a classified span; the tree's leaves. From [`token-lang`](https://crates.io/crates/token-lang).
- **`TokenKind`** — the trait a kind type implements to answer `is_trivia`, `is_eof`, and `symbol`. From `token-lang`.
- **`Symbol`** — an interned lexeme handle a token kind may carry. From `token-lang` (originally `intern-lang`).
- **`Span`** — a half-open byte range `start..end`. From [`span-lang`](https://crates.io/crates/span-lang).
- **`Spanned<T>`** — a value paired with its span. From `span-lang`.

```rust
use syntax_lang::{Span, Spanned, Token, TokenKind};

// `Span::merge` folds child spans into a covering span; the tree does this for you.
assert_eq!(Span::new(0, 3).merge(Span::new(5, 8)), Span::new(0, 8));

// A kind implements `TokenKind` to mark trivia.
#[derive(Clone, Copy)]
enum K { Word, Space }
impl TokenKind for K {
    fn is_trivia(&self) -> bool { matches!(self, K::Space) }
}
assert!(Token::new(K::Space, Span::new(0, 1)).is_trivia());
```

<hr>
<br>
<a href="#top">&uarr; <b>TOP</b></a>
<br>

## Invariants

The following hold for any tree built from a gapless token stream (a lossless lexer emits one). They are covered by the property tests in `tests/properties.rs`.

- **Losslessness.** Concatenating a node's `tokens()` slices, or slicing the source by the node's `span()`, reproduces the node's source exactly.
- **Covering span.** A node's span equals the union of its children's spans: its own source slice equals the concatenation of its leaves' slices.
- **Tiling.** Consecutive leaf tokens are exactly adjacent — no gaps, no overlaps — so the leaf stream tiles the root span.
- **Containment.** Every descendant node's span sits within its ancestor's span.
- **Stack safety.** `Builder`, `tokens()`, `descendants()`, and `Drop` are iterative; a tree of any depth is built, walked, and freed without overflowing the call stack.

<hr>
<br>

## Feature Flags

| Feature | Default | Effect |
|---|---|---|
| `std` | yes | Pulls in the standard library; forwarded to `token-lang/std` and `span-lang/std`. Without it the crate is `no_std` (it always needs `alloc` for the child vectors). |

<hr>
<br>

## Stability & SemVer

As of `1.0.0` the surface above is **frozen**: no breaking change ships before a
`2.0`, additions arrive only in minor releases, and the MSRV (Rust 1.85) rises only
in a minor. The frozen surface is exactly:

- **`Node<K>`** — `new`, `kind`, `span`, `is_empty`, `len`, `children`,
  `child_nodes`, `child_tokens`, `descendants`, `tokens`, `text`.
- **`Element<K>`** — the `Node` / `Token` variants and `span`, `kind`, `as_node`,
  `as_token`, `is_node`, `is_token`.
- **`Builder<K>`** — `new`, `is_empty`, `start_node`, `token`, `finish_node`,
  `finish`, plus `Default`.
- **`BuildError`** — the five variants (`#[non_exhaustive]`, so matching must
  include a wildcard arm), `Display`, and `core::error::Error`.
- **Re-exports** — `Token`, `TokenKind`, `Symbol`, `Span`, `Spanned`.

What is deliberately left out, and can be added later without a breaking change:

- A `serde` feature deriving `Serialize` / `Deserialize` on the tree.
- A mutable or fallible traversal, or a checkpoint/wrap-retroactively builder move.
- Convenience trivia filters gated on `K: TokenKind` (today a caller writes
  `tokens().filter(|t| !t.is_trivia())`).

Because `BuildError` is `#[non_exhaustive]`, adding a variant is not a breaking
change; a downstream `match` on it must already carry a `_` arm.

<hr>

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
