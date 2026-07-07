//! Behavioural tests: build small trees the way a parser would and assert the
//! observable results — losslessness, token order, trivia handling, error paths.

mod common;

use common::{Kind, Shape, build};
use syntax_lang::{BuildError, Builder, Element, Node, Span, Token};

/// `foo (bar)` with the spaces kept as trivia, built by hand.
fn sample() -> (Node<Kind>, &'static str) {
    let source = "foo (bar)";
    let mut b = Builder::new();
    b.start_node(Kind::Group); // root
    b.token(Token::new(Kind::Word, Span::new(0, 3))); // foo
    b.token(Token::new(Kind::Space, Span::new(3, 4))); // space
    b.start_node(Kind::Group); // (bar)
    b.token(Token::new(Kind::Word, Span::new(4, 5))); // (
    b.token(Token::new(Kind::Word, Span::new(5, 8))); // bar
    b.token(Token::new(Kind::Word, Span::new(8, 9))); // )
    b.finish_node();
    b.finish_node();
    (b.finish().expect("balanced"), source)
}

#[test]
fn test_tree_reconstructs_source_losslessly() {
    let (root, source) = sample();
    assert_eq!(root.text(source), Some(source));

    // Concatenating every leaf's own slice reproduces the source too.
    let mut rebuilt = String::new();
    for token in root.tokens() {
        let lo = token.span().start().to_usize();
        let hi = token.span().end().to_usize();
        rebuilt.push_str(&source[lo..hi]);
    }
    assert_eq!(rebuilt, source);
}

#[test]
fn test_tokens_are_in_source_order() {
    let (root, _) = sample();
    let spans: Vec<_> = root.tokens().map(|t| t.span()).collect();
    let mut sorted = spans.clone();
    sorted.sort();
    assert_eq!(spans, sorted);
    assert_eq!(root.tokens().count(), 5);
}

#[test]
fn test_trivia_is_preserved_but_filterable() {
    let (root, _) = sample();
    // Trivia is in the tree...
    assert_eq!(root.tokens().count(), 5);
    // ...and a parser-style walk can skip it.
    let significant = root.tokens().filter(|t| !t.is_trivia()).count();
    assert_eq!(significant, 4);
}

#[test]
fn test_covering_span_equals_child_range_at_every_node() {
    let (root, source) = sample();
    for node in root.descendants() {
        if let Some(text) = node.text(source) {
            // A node's own slice equals the concatenation of its leaves' slices,
            // which is what "the covering span tiles the children" means.
            let mut rebuilt = String::new();
            for token in node.tokens() {
                let lo = token.span().start().to_usize();
                let hi = token.span().end().to_usize();
                rebuilt.push_str(&source[lo..hi]);
            }
            assert_eq!(text, rebuilt, "node {:?} span mismatch", node.kind());
        }
    }
}

#[test]
fn test_nested_group_span() {
    let (root, _) = sample();
    let inner = root.child_nodes().next().expect("one nested group");
    assert_eq!(inner.span(), Span::new(4, 9));
    assert_eq!(inner.tokens().count(), 3);
}

#[test]
fn test_shape_helper_builds_expected_leaf_count() {
    let shape = Shape::Branch(vec![
        Shape::Leaf(Kind::Word, 2),
        Shape::Branch(vec![
            Shape::Leaf(Kind::Space, 1),
            Shape::Leaf(Kind::Word, 3),
        ]),
    ]);
    let (root, len) = build(&shape);
    assert_eq!(len, 6);
    assert_eq!(root.span(), Span::new(0, 6));
    assert_eq!(root.tokens().count(), 3);
    assert_eq!(root.descendants().count(), 2); // root + one branch
}

#[test]
fn test_direct_children_split_correctly() {
    let (root, _) = sample();
    assert_eq!(root.len(), 3); // foo, space, group
    assert_eq!(root.child_tokens().count(), 2);
    assert_eq!(root.child_nodes().count(), 1);
    let kinds: Vec<_> = root.children().map(Element::kind).copied().collect();
    assert_eq!(kinds, [Kind::Word, Kind::Space, Kind::Group]);
}

#[test]
fn test_error_paths_end_to_end() {
    // Unclosed node.
    let mut b = Builder::new();
    b.start_node(Kind::Group);
    b.token(Token::new(Kind::Word, Span::new(0, 1)));
    assert_eq!(b.finish(), Err(BuildError::UnclosedNodes));

    // Token before any node.
    let mut b = Builder::new();
    b.token(Token::new(Kind::Word, Span::new(0, 1)));
    assert_eq!(b.finish(), Err(BuildError::TokenOutsideNode));

    // Extra close.
    let mut b = Builder::<Kind>::new();
    b.finish_node();
    assert_eq!(b.finish(), Err(BuildError::UnbalancedFinish));

    // Two roots.
    let mut b = Builder::new();
    b.start_node(Kind::Group);
    b.finish_node();
    b.start_node(Kind::Group);
    b.finish_node();
    assert_eq!(b.finish(), Err(BuildError::MultipleRoots));

    // Nothing built.
    assert_eq!(Builder::<Kind>::new().finish(), Err(BuildError::EmptyTree));
}

#[test]
fn test_clone_and_eq_roundtrip() {
    let (root, _) = sample();
    let clone = root.clone();
    assert_eq!(root, clone);
}

#[test]
fn test_deep_chain_builds_walks_and_drops() {
    // 50_000 nested single-child groups around one leaf.
    let mut b = Builder::new();
    for _ in 0..50_000 {
        b.start_node(Kind::Group);
    }
    b.token(Token::new(Kind::Word, Span::new(0, 1)));
    for _ in 0..50_000 {
        b.finish_node();
    }
    let root = b.finish().expect("balanced");
    assert_eq!(root.descendants().count(), 50_000);
    assert_eq!(root.tokens().count(), 1);
    assert_eq!(root.span(), Span::new(0, 1));
    drop(root); // iterative Drop must not overflow
}
