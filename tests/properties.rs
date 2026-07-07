//! Property tests: the core CST invariants must hold for arbitrary tree shapes.
//!
//! A tree is generated from a random [`Shape`], its leaves laid out contiguously
//! over a source string, and each invariant checked against that source.

mod common;

use common::{Kind, Shape, build, leaf_count};
use proptest::prelude::*;
use syntax_lang::Node;

/// Generates an arbitrary tree shape: leaves weighted against branches, branches
/// holding 1..=4 children, bounded in depth so the space stays enumerable.
fn shape() -> impl Strategy<Value = Shape> {
    let leaf = (any::<bool>(), 1u32..6).prop_map(|(trivia, len)| {
        let kind = if trivia { Kind::Space } else { Kind::Word };
        Shape::Leaf(kind, len)
    });
    // Root and every interior node is a branch with at least one child, so no
    // empty nodes arise (those have their own dedicated unit tests).
    leaf.prop_recursive(6, 128, 4, |inner| {
        prop::collection::vec(inner, 1..4).prop_map(Shape::Branch)
    })
}

/// Wraps the generated shape so the root is always a branch (a tree's root is a
/// node, never a bare leaf).
fn root_shape() -> impl Strategy<Value = Shape> {
    prop::collection::vec(shape(), 1..4).prop_map(Shape::Branch)
}

/// Reconstructs a node's source by concatenating its leaves' slices.
fn concat_tokens(node: &Node<Kind>, source: &str) -> String {
    let mut out = String::new();
    for token in node.tokens() {
        let lo = token.span().start().to_usize();
        let hi = token.span().end().to_usize();
        out.push_str(&source[lo..hi]);
    }
    out
}

proptest! {
    /// Slicing the source by the root span reproduces the whole source.
    #[test]
    fn prop_root_span_covers_whole_source(shape in root_shape()) {
        let (root, len) = build(&shape);
        let source = "x".repeat(len as usize);
        prop_assert_eq!(root.span().start().to_u32(), 0);
        prop_assert_eq!(root.span().end().to_u32(), len);
        prop_assert_eq!(root.text(&source), Some(source.as_str()));
    }

    /// The leaf-token stream reproduces the source exactly — losslessness.
    #[test]
    fn prop_tokens_reconstruct_source(shape in root_shape()) {
        let (root, len) = build(&shape);
        let source = "y".repeat(len as usize);
        prop_assert_eq!(concat_tokens(&root, &source), source);
    }

    /// The tree holds exactly as many leaves as the shape describes.
    #[test]
    fn prop_token_count_matches_shape(shape in root_shape()) {
        let (root, _) = build(&shape);
        prop_assert_eq!(root.tokens().count(), leaf_count(&shape));
    }

    /// Every node's covering span equals the range its own leaves tile: the node's
    /// source slice equals the concatenation of its leaves' slices.
    #[test]
    fn prop_covering_span_tiles_children(shape in root_shape()) {
        let (root, len) = build(&shape);
        let source = "z".repeat(len as usize);
        for node in root.descendants() {
            let sliced = node.text(&source).expect("span within source");
            prop_assert_eq!(sliced, concat_tokens(node, &source));
        }
    }

    /// Tokens come out in non-decreasing span order, with no gaps or overlaps —
    /// consecutive leaves are exactly adjacent.
    #[test]
    fn prop_tokens_tile_without_gaps(shape in root_shape()) {
        let (root, _) = build(&shape);
        let mut prev_end = 0u32;
        for token in root.tokens() {
            prop_assert_eq!(token.span().start().to_u32(), prev_end);
            prev_end = token.span().end().to_u32();
        }
    }

    /// Every descendant node's span sits within the root's span (containment).
    #[test]
    fn prop_descendant_spans_within_root(shape in root_shape()) {
        let (root, _) = build(&shape);
        let (lo, hi) = (root.span().start().to_u32(), root.span().end().to_u32());
        for node in root.descendants() {
            prop_assert!(node.span().start().to_u32() >= lo);
            prop_assert!(node.span().end().to_u32() <= hi);
        }
    }
}
