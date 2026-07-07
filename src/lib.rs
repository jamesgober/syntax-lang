//! # syntax_lang
//!
//! A lossless concrete syntax tree (CST) with trivia — the substrate a formatter,
//! a language server, or a tree-sitter-style generator builds on. It owns no
//! grammar: a language brings its own kind type, and syntax-lang supplies the tree
//! shape, the builder that assembles it, and the traversals over it.
//!
//! ## Model
//!
//! A tree is [`Node`]s all the way down to [`Token`] leaves. Each [`Node`] carries
//! a kind, the [`Span`] of source it covers, and an ordered list of [`Element`]
//! children — each child being a nested node or a leaf token. Node kinds and token
//! kinds share one type `K` (the rowan model): a language defines a single `enum`
//! with both composite variants (`Expr`, `Root`) and lexical ones (`Ident`,
//! `Plus`, `Whitespace`), and implements [`TokenKind`] on it to mark trivia and the
//! end marker.
//!
//! *Lossless* means nothing is discarded. Trivia — whitespace and comments — is not
//! filtered out; it rides as ordinary leaf tokens in source order. So the tree is a
//! faithful record of the source: slicing the original text by a node's span, or
//! concatenating the node's [`tokens`](Node::tokens), reproduces exactly the source
//! that node came from. Tokens store spans rather than copies of the text, so the
//! tree stays compact and the reconstruction is a zero-copy borrow.
//!
//! ## Building a tree
//!
//! A parser drives a [`Builder`] with three moves — open a node, push a token,
//! close a node:
//!
//! ```
//! use syntax_lang::{Builder, Span, Token, TokenKind};
//!
//! // One kind type for both nodes and tokens.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum Kind {
//!     // nodes
//!     Root,
//!     Sum,
//!     // tokens
//!     Num,
//!     Plus,
//!     Space,
//! }
//!
//! impl TokenKind for Kind {
//!     fn is_trivia(&self) -> bool {
//!         matches!(self, Kind::Space)
//!     }
//! }
//!
//! // Build the CST for `1 + 2`, keeping the spaces as trivia tokens.
//! let mut b = Builder::new();
//! b.start_node(Kind::Root);
//! b.start_node(Kind::Sum);
//! b.token(Token::new(Kind::Num, Span::new(0, 1)));
//! b.token(Token::new(Kind::Space, Span::new(1, 2)));
//! b.token(Token::new(Kind::Plus, Span::new(2, 3)));
//! b.token(Token::new(Kind::Space, Span::new(3, 4)));
//! b.token(Token::new(Kind::Num, Span::new(4, 5)));
//! b.finish_node(); // close Sum
//! b.finish_node(); // close Root
//!
//! let root = b.finish().expect("balanced");
//!
//! // Lossless: the tree reproduces the source, trivia and all.
//! assert_eq!(root.text("1 + 2"), Some("1 + 2"));
//! assert_eq!(root.tokens().count(), 5);
//!
//! // A formatter can now walk the significant tokens and skip the trivia.
//! let significant = root.tokens().filter(|t| !t.is_trivia()).count();
//! assert_eq!(significant, 3);
//! ```
//!
//! ## Walking a tree
//!
//! [`Node::tokens`] yields every leaf in source order (the lossless stream);
//! [`Node::descendants`] yields every node in pre-order; [`Node::children`],
//! [`Node::child_nodes`], and [`Node::child_tokens`] step over the direct level.
//! All of these are iterative — a tree tens of thousands of levels deep is walked,
//! and freed, without overflowing the call stack.
//!
//! ## Features
//!
//! - `std` (default) — the standard library; without it the crate is `no_std` (it
//!   always needs `alloc` for the child vectors). Forwards to `token-lang/std` and
//!   `span-lang/std`.
//!
//! ## Stability
//!
//! The public surface is being designed across the 0.x series and freezes at
//! `1.0.0`, after which it follows Semantic Versioning: no breaking change before
//! `2.0`, additions arrive in minor releases, and the MSRV (Rust 1.85) only rises
//! in a minor. The surface is catalogued in
//! [`docs/API.md`](https://github.com/jamesgober/syntax-lang/blob/main/docs/API.md).

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

mod build;
mod tree;

pub use build::{BuildError, Builder};
pub use tree::{Element, Node};

// Re-exported so a downstream defining and walking a CST can name the token and
// position types this crate's API is built on without also depending on
// `token-lang` and `span-lang` directly.
pub use span_lang::{Span, Spanned};
pub use token_lang::{Symbol, Token, TokenKind};
