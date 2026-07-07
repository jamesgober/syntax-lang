//! Shared fixtures for the integration tests: a minimal unified kind type and a
//! helper that builds a tree from a declarative shape.

#![allow(dead_code)]

use syntax_lang::{Builder, Node, Span, Token, TokenKind};

/// A single kind type covering both nodes and tokens, as a real language would
/// define. `Space` is trivia; everything else is significant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Composite node grouping a run of children.
    Group,
    /// A significant word-like token.
    Word,
    /// Whitespace trivia — preserved in the tree, skipped by a parser.
    Space,
}

impl TokenKind for Kind {
    fn is_trivia(&self) -> bool {
        matches!(self, Kind::Space)
    }
}

/// A declarative tree shape: a leaf token of a given kind and byte length, or a
/// group of nested shapes. Used to build varied trees from both fixed fixtures and
/// property-generated data.
#[derive(Clone, Debug)]
pub enum Shape {
    /// A leaf token with a kind and the byte length it occupies in source.
    Leaf(Kind, u32),
    /// A `Group` node wrapping child shapes.
    Branch(Vec<Shape>),
}

/// Builds a tree from `shape`, laying leaves out contiguously starting at byte 0.
/// The root shape must be a `Branch` (a tree's root is a node). Returns the root
/// node and the total source length its leaves tile.
#[must_use]
pub fn build(shape: &Shape) -> (Node<Kind>, u32) {
    let mut builder = Builder::new();
    let mut cursor = 0u32;
    emit(shape, &mut builder, &mut cursor);
    let root = builder.finish().expect("shape produces a balanced tree");
    (root, cursor)
}

fn emit(shape: &Shape, builder: &mut Builder<Kind>, cursor: &mut u32) {
    match shape {
        Shape::Leaf(kind, len) => {
            let start = *cursor;
            *cursor += *len;
            builder.token(Token::new(*kind, Span::new(start, *cursor)));
        }
        Shape::Branch(children) => {
            builder.start_node(Kind::Group);
            for child in children {
                emit(child, builder, cursor);
            }
            builder.finish_node();
        }
    }
}

/// Counts the leaf tokens a shape contains.
#[must_use]
pub fn leaf_count(shape: &Shape) -> usize {
    match shape {
        Shape::Leaf(..) => 1,
        Shape::Branch(children) => children.iter().map(leaf_count).sum(),
    }
}
