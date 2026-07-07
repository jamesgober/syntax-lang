//! The tree itself: [`Node`] interior nodes, [`Element`] children, and the
//! iterative traversals over them.

use alloc::vec;
use alloc::vec::Vec;
use core::slice;

use span_lang::Span;
use token_lang::Token;

/// One child of a [`Node`]: either a nested node or a leaf token.
///
/// A concrete syntax tree alternates between the two — a node groups a run of
/// children under a kind, and a [`Token`] is a leaf carrying a classified span of
/// source. Trivia (whitespace, comments) is not special: it rides as an ordinary
/// leaf token, which is what makes the tree lossless.
///
/// # Examples
///
/// ```
/// use syntax_lang::{Element, Node, Span, Token};
///
/// let leaf: Element<&str> = Element::Token(Token::new("ident", Span::new(0, 3)));
/// assert!(leaf.is_token());
/// assert_eq!(leaf.kind(), &"ident");
/// assert_eq!(leaf.span(), Span::new(0, 3));
///
/// let group = Element::Node(Node::new("expr", vec![leaf]));
/// assert!(group.is_node());
/// assert_eq!(group.span(), Span::new(0, 3));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element<K> {
    /// A nested interior node.
    Node(Node<K>),
    /// A leaf token: a classified span of source.
    Token(Token<K>),
}

impl<K> Element<K> {
    /// The span of source this child covers — the node's covering span or the
    /// token's own span.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Span, Token};
    ///
    /// let e = Element::Token(Token::new('x', Span::new(4, 5)));
    /// assert_eq!(e.span(), Span::new(4, 5));
    /// ```
    #[inline]
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Element::Node(node) => node.span(),
            Element::Token(token) => token.span(),
        }
    }

    /// Borrows the kind of this child — the node's kind or the token's kind.
    ///
    /// Node kinds and token kinds share one type `K` (the rowan model), so a
    /// caller can read a child's kind without first knowing whether it is a node
    /// or a leaf.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Span, Token};
    ///
    /// let e = Element::Token(Token::new("plus", Span::new(1, 2)));
    /// assert_eq!(e.kind(), &"plus");
    /// ```
    #[inline]
    #[must_use]
    pub fn kind(&self) -> &K {
        match self {
            Element::Node(node) => node.kind(),
            Element::Token(token) => token.kind(),
        }
    }

    /// Returns the nested node if this child is one, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let node = Element::Node(Node::new("n", vec![Element::Token(Token::new("t", Span::new(0, 1)))]));
    /// assert!(node.as_node().is_some());
    /// assert!(node.as_token().is_none());
    /// ```
    #[inline]
    #[must_use]
    pub fn as_node(&self) -> Option<&Node<K>> {
        match self {
            Element::Node(node) => Some(node),
            Element::Token(_) => None,
        }
    }

    /// Returns the leaf token if this child is one, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Span, Token};
    ///
    /// let e = Element::Token(Token::new("t", Span::new(0, 1)));
    /// assert_eq!(e.as_token().map(|t| *t.kind()), Some("t"));
    /// ```
    #[inline]
    #[must_use]
    pub fn as_token(&self) -> Option<&Token<K>> {
        match self {
            Element::Token(token) => Some(token),
            Element::Node(_) => None,
        }
    }

    /// Whether this child is a nested node.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Span, Token};
    ///
    /// assert!(!Element::Token(Token::new(0u8, Span::new(0, 1))).is_node());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_node(&self) -> bool {
        matches!(self, Element::Node(_))
    }

    /// Whether this child is a leaf token.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Span, Token};
    ///
    /// assert!(Element::Token(Token::new(0u8, Span::new(0, 1))).is_token());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_token(&self) -> bool {
        matches!(self, Element::Token(_))
    }
}

/// An interior node of a concrete syntax tree: a [`kind`](Node::kind), the
/// [`span`](Node::span) of source it covers, and its ordered [`children`](Node::children).
///
/// A node owns its children directly, so a whole tree is a single owned value with
/// no arena or handle bookkeeping. The children are in source order; a node's
/// covering span is the union of its children's spans, computed once when the node
/// is built. Because trivia rides as ordinary leaf tokens, the tree is *lossless*:
/// slicing the original source by a node's span — or concatenating its
/// [`tokens`](Node::tokens) — reproduces exactly the source that node came from.
///
/// The kind type `K` is shared by nodes and tokens alike (an `enum` a language
/// defines with both composite and lexical variants), following the model
/// [`token_lang`](token_lang) establishes. The node type itself is generic over any
/// `K` and needs no trait bound.
///
/// # Stack safety
///
/// Traversal ([`tokens`](Node::tokens), [`descendants`](Node::descendants)) and
/// teardown (`Drop`) are *iterative*: a tree tens of thousands of levels deep is
/// walked and freed without recursing on the call stack. `Clone`, `PartialEq`, and
/// `Debug` recurse with tree depth and so suit trees of realistic source depth.
///
/// # Examples
///
/// ```
/// use syntax_lang::{Element, Node, Span, Token};
///
/// // `1 + 2` as a tiny expression node with three leaf tokens.
/// let node = Node::new(
///     "add",
///     vec![
///         Element::Token(Token::new("num", Span::new(0, 1))),
///         Element::Token(Token::new("plus", Span::new(2, 3))),
///         Element::Token(Token::new("num", Span::new(4, 5))),
///     ],
/// );
///
/// assert_eq!(node.kind(), &"add");
/// assert_eq!(node.span(), Span::new(0, 5));
/// assert_eq!(node.text("1 + 2"), Some("1 + 2"));
/// assert_eq!(node.children().count(), 3);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node<K> {
    kind: K,
    span: Span,
    children: Vec<Element<K>>,
}

impl<K> Node<K> {
    /// Builds a node from a kind and its ordered children, computing the covering
    /// span as the union of the children's spans.
    ///
    /// Children are taken in source order. A node with no children reports an empty
    /// span at offset `0`; build such nodes through a [`Builder`](crate::Builder)
    /// instead, which places an empty node at the current stream position.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new(
    ///     "paren",
    ///     vec![
    ///         Element::Token(Token::new("(", Span::new(0, 1))),
    ///         Element::Token(Token::new(")", Span::new(1, 2))),
    ///     ],
    /// );
    /// assert_eq!(n.span(), Span::new(0, 2));
    /// ```
    #[must_use]
    pub fn new(kind: K, children: Vec<Element<K>>) -> Self {
        let span = cover(&children, Span::empty(0));
        Self {
            kind,
            span,
            children,
        }
    }

    /// Builds a node with a caller-supplied span, used by the
    /// [`Builder`](crate::Builder) to place an empty node at the current stream
    /// cursor rather than at offset `0`. For a node with children the span still
    /// equals the union of their spans; this only differs for the childless case.
    #[must_use]
    pub(crate) fn with_span(kind: K, children: Vec<Element<K>>, empty: Span) -> Self {
        let span = cover(&children, empty);
        Self {
            kind,
            span,
            children,
        }
    }

    /// Borrows this node's kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::Node;
    ///
    /// let n: Node<&str> = Node::new("root", vec![]);
    /// assert_eq!(n.kind(), &"root");
    /// ```
    #[inline]
    #[must_use]
    pub fn kind(&self) -> &K {
        &self.kind
    }

    /// Returns the span of source this node covers: the union of its children's
    /// spans, or an empty span if it has none.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new("n", vec![Element::Token(Token::new("t", Span::new(3, 8)))]);
    /// assert_eq!(n.span(), Span::new(3, 8));
    /// ```
    #[inline]
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    /// Whether this node has no children.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::Node;
    ///
    /// let n: Node<&str> = Node::new("empty", vec![]);
    /// assert!(n.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// The number of direct children (nodes and tokens) this node has.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new("n", vec![Element::Token(Token::new("t", Span::new(0, 1)))]);
    /// assert_eq!(n.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Iterates this node's direct children — nodes and tokens interleaved in
    /// source order.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new(
    ///     "n",
    ///     vec![
    ///         Element::Token(Token::new("a", Span::new(0, 1))),
    ///         Element::Node(Node::new("inner", vec![Element::Token(Token::new("b", Span::new(1, 2)))])),
    ///     ],
    /// );
    /// let kinds: Vec<_> = n.children().map(Element::kind).copied().collect();
    /// assert_eq!(kinds, ["a", "inner"]);
    /// ```
    #[inline]
    pub fn children(&self) -> impl Iterator<Item = &Element<K>> {
        self.children.iter()
    }

    /// Iterates only the direct children that are nested nodes, skipping leaf
    /// tokens.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new(
    ///     "n",
    ///     vec![
    ///         Element::Token(Token::new("a", Span::new(0, 1))),
    ///         Element::Node(Node::new("inner", vec![Element::Token(Token::new("b", Span::new(1, 2)))])),
    ///     ],
    /// );
    /// assert_eq!(n.child_nodes().count(), 1);
    /// ```
    #[inline]
    pub fn child_nodes(&self) -> impl Iterator<Item = &Node<K>> {
        self.children.iter().filter_map(Element::as_node)
    }

    /// Iterates only the direct children that are leaf tokens, skipping nested
    /// nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new(
    ///     "n",
    ///     vec![
    ///         Element::Token(Token::new("a", Span::new(0, 1))),
    ///         Element::Node(Node::new("inner", vec![Element::Token(Token::new("b", Span::new(1, 2)))])),
    ///     ],
    /// );
    /// let direct: Vec<_> = n.child_tokens().map(|t| *t.kind()).collect();
    /// assert_eq!(direct, ["a"]);
    /// ```
    #[inline]
    pub fn child_tokens(&self) -> impl Iterator<Item = &Token<K>> {
        self.children.iter().filter_map(Element::as_token)
    }

    /// Iterates this node and every node beneath it in pre-order (a parent before
    /// its descendants, children in source order).
    ///
    /// The walk is iterative — its work stack lives on the heap — so a tree of any
    /// depth is traversed without overflowing the call stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let tree = Node::new(
    ///     "root",
    ///     vec![Element::Node(Node::new(
    ///         "inner",
    ///         vec![Element::Token(Token::new("t", Span::new(0, 1)))],
    ///     ))],
    /// );
    /// let kinds: Vec<_> = tree.descendants().map(Node::kind).copied().collect();
    /// assert_eq!(kinds, ["root", "inner"]);
    /// ```
    #[inline]
    pub fn descendants(&self) -> impl Iterator<Item = &Node<K>> {
        Descendants {
            root: Some(self),
            stack: Vec::new(),
        }
    }

    /// Iterates every leaf token in the tree in source order — the lossless token
    /// stream, trivia included.
    ///
    /// Concatenating these tokens' source slices reproduces the node's source
    /// exactly; the walk is iterative and safe on any depth.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let tree = Node::new(
    ///     "root",
    ///     vec![Element::Node(Node::new(
    ///         "inner",
    ///         vec![
    ///             Element::Token(Token::new("a", Span::new(0, 1))),
    ///             Element::Token(Token::new("b", Span::new(1, 2))),
    ///         ],
    ///     ))],
    /// );
    /// let leaves: Vec<_> = tree.tokens().map(|t| *t.kind()).collect();
    /// assert_eq!(leaves, ["a", "b"]);
    /// ```
    #[inline]
    pub fn tokens(&self) -> impl Iterator<Item = &Token<K>> {
        Tokens {
            stack: vec![self.children.iter()],
        }
    }

    /// Slices `source` by this node's covering span, returning the exact text the
    /// node came from, or `None` if the span lies outside `source`.
    ///
    /// This is zero-copy: it borrows a sub-slice of `source` rather than allocating.
    /// Pass the same source the tree was built from; the `None` case guards against
    /// a mismatched or truncated string rather than panicking.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Element, Node, Span, Token};
    ///
    /// let n = Node::new(
    ///     "call",
    ///     vec![
    ///         Element::Token(Token::new("id", Span::new(0, 1))),
    ///         Element::Token(Token::new("(", Span::new(1, 2))),
    ///         Element::Token(Token::new(")", Span::new(2, 3))),
    ///     ],
    /// );
    /// assert_eq!(n.text("f()"), Some("f()"));
    /// assert_eq!(n.text("f"), None); // span runs past the string
    /// ```
    #[inline]
    #[must_use]
    pub fn text<'s>(&self, source: &'s str) -> Option<&'s str> {
        let start = self.span.start().to_usize();
        let end = self.span.end().to_usize();
        source.get(start..end)
    }
}

impl<K> Drop for Node<K> {
    /// Frees the tree without recursion.
    ///
    /// The default drop glue would recurse once per level of nesting, overflowing
    /// the stack on a pathologically deep tree (a long chain of single-child
    /// nodes). This moves every descendant node onto an explicit heap worklist and
    /// drops them there, so teardown depth is bounded by heap, not stack.
    fn drop(&mut self) {
        // Fast path: a leaf-only node (or already-drained node) has no nested nodes
        // to recurse into, so the default glue is already non-recursive.
        if self.children.iter().all(Element::is_token) {
            return;
        }
        let mut stack: Vec<Node<K>> = Vec::new();
        drain_nodes(&mut self.children, &mut stack);
        while let Some(mut node) = stack.pop() {
            // Detaching `node`'s children before it drops means its own drop glue
            // runs against an empty vector — no further recursion.
            drain_nodes(&mut node.children, &mut stack);
        }
    }
}

/// Moves every nested node out of `children` onto `stack`; leaf tokens drop in
/// place as the source vector is cleared.
fn drain_nodes<K>(children: &mut Vec<Element<K>>, stack: &mut Vec<Node<K>>) {
    for element in children.drain(..) {
        if let Element::Node(node) = element {
            stack.push(node);
        }
    }
}

/// Folds the children's spans into their covering span, falling back to `empty`
/// for a childless node.
fn cover<K>(children: &[Element<K>], empty: Span) -> Span {
    let mut iter = children.iter();
    match iter.next() {
        None => empty,
        Some(first) => iter.fold(first.span(), |acc, child| acc.merge(child.span())),
    }
}

/// Pre-order iterator over a node and its descendants. Returned by
/// [`Node::descendants`]; iterative, so it never recurses on the call stack.
struct Descendants<'a, K> {
    root: Option<&'a Node<K>>,
    stack: Vec<slice::Iter<'a, Element<K>>>,
}

impl<'a, K> Iterator for Descendants<'a, K> {
    type Item = &'a Node<K>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(root) = self.root.take() {
            self.stack.push(root.children.iter());
            return Some(root);
        }
        loop {
            let top = self.stack.last_mut()?;
            match top.next() {
                None => {
                    let _ = self.stack.pop();
                }
                Some(Element::Node(node)) => {
                    self.stack.push(node.children.iter());
                    return Some(node);
                }
                Some(Element::Token(_)) => {}
            }
        }
    }
}

/// Source-order iterator over every leaf token in a tree. Returned by
/// [`Node::tokens`]; iterative, so it never recurses on the call stack.
struct Tokens<'a, K> {
    stack: Vec<slice::Iter<'a, Element<K>>>,
}

impl<'a, K> Iterator for Tokens<'a, K> {
    type Item = &'a Token<K>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let top = self.stack.last_mut()?;
            match top.next() {
                None => {
                    let _ = self.stack.pop();
                }
                Some(Element::Node(node)) => {
                    self.stack.push(node.children.iter());
                }
                Some(Element::Token(token)) => return Some(token),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn tok(kind: &'static str, lo: u32, hi: u32) -> Element<&'static str> {
        Element::Token(Token::new(kind, Span::new(lo, hi)))
    }

    #[test]
    fn test_new_covers_children_span() {
        let n = Node::new("n", vec![tok("a", 2, 4), tok("b", 4, 9)]);
        assert_eq!(n.span(), Span::new(2, 9));
    }

    #[test]
    fn test_new_empty_node_has_empty_span() {
        let n: Node<&str> = Node::new("n", vec![]);
        assert_eq!(n.span(), Span::empty(0));
        assert!(n.is_empty());
        assert_eq!(n.len(), 0);
    }

    #[test]
    fn test_children_iterators_split_nodes_and_tokens() {
        let inner = Element::Node(Node::new("inner", vec![tok("x", 1, 2)]));
        let n = Node::new("n", vec![tok("a", 0, 1), inner]);
        assert_eq!(n.children().count(), 2);
        assert_eq!(n.child_nodes().count(), 1);
        assert_eq!(
            n.child_tokens().map(|t| *t.kind()).collect::<Vec<_>>(),
            ["a"]
        );
    }

    #[test]
    fn test_descendants_preorder() {
        let tree = Node::new(
            "root",
            vec![
                Element::Node(Node::new("l", vec![tok("a", 0, 1)])),
                Element::Node(Node::new("r", vec![tok("b", 1, 2)])),
            ],
        );
        let kinds: Vec<_> = tree.descendants().map(Node::kind).copied().collect();
        assert_eq!(kinds, ["root", "l", "r"]);
    }

    #[test]
    fn test_tokens_source_order_includes_all_leaves() {
        let tree = Node::new(
            "root",
            vec![
                tok("a", 0, 1),
                Element::Node(Node::new("inner", vec![tok("b", 1, 2), tok("c", 2, 3)])),
                tok("d", 3, 4),
            ],
        );
        let leaves: Vec<_> = tree.tokens().map(|t| *t.kind()).collect();
        assert_eq!(leaves, ["a", "b", "c", "d"]);
    }

    #[test]
    fn test_text_slices_source_and_rejects_out_of_bounds() {
        let n = Node::new("n", vec![tok("a", 0, 2), tok("b", 2, 5)]);
        assert_eq!(n.text("hello"), Some("hello"));
        assert_eq!(n.text("hi"), None);
    }

    #[test]
    fn test_element_accessors() {
        let e = tok("a", 0, 1);
        assert!(e.is_token());
        assert!(!e.is_node());
        assert_eq!(e.kind(), &"a");
        assert_eq!(e.span(), Span::new(0, 1));
        assert!(e.as_token().is_some());
        assert!(e.as_node().is_none());
    }

    #[test]
    fn test_deep_tree_drops_without_stack_overflow() {
        // A 200_000-level left chain: recursive drop glue would overflow here.
        let mut node = Node::new("leaf", vec![tok("t", 0, 1)]);
        for _ in 0..200_000 {
            node = Node::new("link", vec![Element::Node(node)]);
        }
        drop(node);
    }

    #[test]
    fn test_deep_tree_tokens_and_descendants_are_iterative() {
        let mut node = Node::new("leaf", vec![tok("t", 0, 1)]);
        for _ in 0..100_000 {
            node = Node::new("link", vec![Element::Node(node)]);
        }
        assert_eq!(node.tokens().count(), 1);
        assert_eq!(node.descendants().count(), 100_001);
    }
}
