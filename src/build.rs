//! Tree construction: the parser-facing [`Builder`] and the [`BuildError`] it
//! reports on misuse.

use alloc::vec::Vec;
use core::fmt;

use span_lang::Span;
use token_lang::Token;

use crate::tree::{Element, Node};

/// Why a [`Builder`] could not produce a tree.
///
/// The builder never panics on misuse; it records the first fault and surfaces it
/// from [`Builder::finish`]. Each variant names a specific imbalance between the
/// `start_node` / `token` / `finish_node` calls and what a caller should change.
///
/// # Examples
///
/// ```
/// use syntax_lang::{BuildError, Builder};
///
/// // Closing a node that was never opened.
/// let mut b = Builder::<&str>::new();
/// b.finish_node();
/// assert_eq!(b.finish(), Err(BuildError::UnbalancedFinish));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// [`finish`](Builder::finish) was called before any node was started, so
    /// there is no tree to return. Start a root node before finishing.
    EmptyTree,
    /// [`finish`](Builder::finish) was called while nodes were still open. Balance
    /// every [`start_node`](Builder::start_node) with a
    /// [`finish_node`](Builder::finish_node) before finishing.
    UnclosedNodes,
    /// [`finish_node`](Builder::finish_node) was called with no open node. Remove
    /// the extra close, or add the missing [`start_node`](Builder::start_node).
    UnbalancedFinish,
    /// [`token`](Builder::token) was called before any node was started. A tree's
    /// root must be a node; open one before pushing tokens.
    TokenOutsideNode,
    /// A second root node was started after the first root was already closed. A
    /// tree has exactly one root; wrap the roots in an enclosing node instead.
    MultipleRoots,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            BuildError::EmptyTree => "no node was started before finishing the tree",
            BuildError::UnclosedNodes => "the tree was finished with nodes still open",
            BuildError::UnbalancedFinish => "a node was closed with no open node to close",
            BuildError::TokenOutsideNode => "a token was pushed before any node was started",
            BuildError::MultipleRoots => "a second root node was started after the first closed",
        };
        f.write_str(message)
    }
}

impl core::error::Error for BuildError {}

/// A node whose kind is fixed but whose children are still being collected.
struct Partial<K> {
    kind: K,
    children: Vec<Element<K>>,
}

/// Assembles a [`Node`] tree from a stream of `start_node` / `token` /
/// `finish_node` calls — the shape a parser drives as it recognises the grammar.
///
/// The builder keeps a stack of open nodes. [`start_node`](Builder::start_node)
/// opens one, [`token`](Builder::token) appends a leaf to whichever node is
/// innermost, and [`finish_node`](Builder::finish_node) closes the innermost node
/// and folds it into its parent. When the outermost node closes it becomes the
/// root, and [`finish`](Builder::finish) hands it back.
///
/// Misuse is never a panic. The first structural fault — an unbalanced close, a
/// token at the root, a second root — is remembered and returned from
/// [`finish`](Builder::finish) as a [`BuildError`], so a parser can drive the
/// builder on its hot path without wrapping every call in a `Result`.
///
/// # Examples
///
/// Build `(1)` — a parenthesised literal — and read it back losslessly:
///
/// ```
/// use syntax_lang::{Builder, Span, Token};
///
/// let mut b = Builder::new();
/// b.start_node("paren");
/// b.token(Token::new("(", Span::new(0, 1)));
/// b.start_node("lit");
/// b.token(Token::new("num", Span::new(1, 2)));
/// b.finish_node(); // close "lit"
/// b.token(Token::new(")", Span::new(2, 3)));
/// b.finish_node(); // close "paren"
///
/// let root = b.finish().expect("balanced");
/// assert_eq!(root.kind(), &"paren");
/// assert_eq!(root.span(), Span::new(0, 3));
/// assert_eq!(root.text("(1)"), Some("(1)"));
/// let kinds: Vec<_> = root.tokens().map(|t| *t.kind()).collect();
/// assert_eq!(kinds, ["(", "num", ")"]);
/// ```
pub struct Builder<K> {
    stack: Vec<Partial<K>>,
    root: Option<Node<K>>,
    cursor: u32,
    error: Option<BuildError>,
}

impl<K> Builder<K> {
    /// Creates an empty builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::Builder;
    ///
    /// let b = Builder::<&str>::new();
    /// assert!(b.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stack: Vec::new(),
            root: None,
            cursor: 0,
            error: None,
        }
    }

    /// Whether nothing has been built yet — no open nodes and no completed root.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Builder, Span, Token};
    ///
    /// let mut b = Builder::new();
    /// assert!(b.is_empty());
    /// b.start_node("root");
    /// assert!(!b.is_empty());
    /// b.token(Token::new("t", Span::new(0, 1)));
    /// b.finish_node();
    /// let _ = b.finish();
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty() && self.root.is_none()
    }

    /// Opens a new node of `kind`. Tokens and child nodes added until the matching
    /// [`finish_node`](Builder::finish_node) become its children.
    ///
    /// Starting a second node once the root has already closed records
    /// [`BuildError::MultipleRoots`]; the call is otherwise infallible.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Builder, Span, Token};
    ///
    /// let mut b = Builder::new();
    /// b.start_node("root");
    /// b.token(Token::new("t", Span::new(0, 1)));
    /// b.finish_node();
    /// assert!(b.finish().is_ok());
    /// ```
    pub fn start_node(&mut self, kind: K) {
        if self.error.is_some() {
            return;
        }
        if self.root.is_some() && self.stack.is_empty() {
            self.fail(BuildError::MultipleRoots);
            return;
        }
        self.stack.push(Partial {
            kind,
            children: Vec::new(),
        });
    }

    /// Appends `token` as a leaf of the innermost open node.
    ///
    /// Pushing a token with no node open records [`BuildError::TokenOutsideNode`],
    /// since a tree's root must be a node rather than a bare token.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{BuildError, Builder, Span, Token};
    ///
    /// let mut b = Builder::new();
    /// b.token(Token::new("t", Span::new(0, 1))); // no node open yet
    /// assert_eq!(b.finish(), Err(BuildError::TokenOutsideNode));
    /// ```
    pub fn token(&mut self, token: Token<K>) {
        if self.error.is_some() {
            return;
        }
        match self.stack.last_mut() {
            Some(top) => {
                self.cursor = self.cursor.max(token.span().end().to_u32());
                top.children.push(Element::Token(token));
            }
            None => self.fail(BuildError::TokenOutsideNode),
        }
    }

    /// Closes the innermost open node, folding it into its parent — or promoting it
    /// to the root when it is the outermost node.
    ///
    /// Closing with no node open records [`BuildError::UnbalancedFinish`].
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{Builder, Span, Token};
    ///
    /// let mut b = Builder::new();
    /// b.start_node("outer");
    /// b.start_node("inner");
    /// b.token(Token::new("t", Span::new(0, 1)));
    /// b.finish_node(); // close "inner"
    /// b.finish_node(); // close "outer"
    /// let root = b.finish().expect("balanced");
    /// assert_eq!(root.child_nodes().count(), 1);
    /// ```
    pub fn finish_node(&mut self) {
        if self.error.is_some() {
            return;
        }
        let Some(partial) = self.stack.pop() else {
            self.fail(BuildError::UnbalancedFinish);
            return;
        };
        let node = self.build(partial);
        match self.stack.last_mut() {
            Some(parent) => parent.children.push(Element::Node(node)),
            None => {
                if self.root.is_some() {
                    self.fail(BuildError::MultipleRoots);
                } else {
                    self.root = Some(node);
                }
            }
        }
    }

    /// Consumes the builder, returning the finished root node.
    ///
    /// Returns the first recorded [`BuildError`] if the call sequence was
    /// unbalanced, [`BuildError::UnclosedNodes`] if nodes are still open, or
    /// [`BuildError::EmptyTree`] if no node was ever started.
    ///
    /// # Examples
    ///
    /// ```
    /// use syntax_lang::{BuildError, Builder};
    ///
    /// let b = Builder::<&str>::new();
    /// assert_eq!(b.finish(), Err(BuildError::EmptyTree));
    /// ```
    pub fn finish(self) -> Result<Node<K>, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if !self.stack.is_empty() {
            return Err(BuildError::UnclosedNodes);
        }
        self.root.ok_or(BuildError::EmptyTree)
    }

    /// Turns a completed [`Partial`] into a [`Node`], placing an empty node at the
    /// current stream cursor.
    fn build(&self, partial: Partial<K>) -> Node<K> {
        if partial.children.is_empty() {
            Node::with_span(partial.kind, partial.children, Span::empty(self.cursor))
        } else {
            Node::new(partial.kind, partial.children)
        }
    }

    /// Records the first fault; later faults do not overwrite it.
    fn fail(&mut self, error: BuildError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

impl<K> Default for Builder<K> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: fmt::Debug> fmt::Debug for Builder<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder")
            .field("open_nodes", &self.stack.len())
            .field("has_root", &self.root.is_some())
            .field("error", &self.error)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn t(kind: &'static str, lo: u32, hi: u32) -> Token<&'static str> {
        Token::new(kind, Span::new(lo, hi))
    }

    #[test]
    fn test_builds_flat_tree() {
        let mut b = Builder::new();
        b.start_node("root");
        b.token(t("a", 0, 1));
        b.token(t("b", 1, 2));
        b.finish_node();
        let root = b.finish().expect("balanced");
        assert_eq!(root.kind(), &"root");
        assert_eq!(root.span(), Span::new(0, 2));
        assert_eq!(root.child_tokens().count(), 2);
    }

    #[test]
    fn test_builds_nested_tree_with_covering_spans() {
        let mut b = Builder::new();
        b.start_node("outer");
        b.token(t("(", 0, 1));
        b.start_node("inner");
        b.token(t("x", 1, 2));
        b.finish_node();
        b.token(t(")", 2, 3));
        b.finish_node();
        let root = b.finish().expect("balanced");
        assert_eq!(root.span(), Span::new(0, 3));
        let inner = root.child_nodes().next().expect("one inner node");
        assert_eq!(inner.span(), Span::new(1, 2));
        let leaves: Vec<_> = root.tokens().map(|tok| *tok.kind()).collect();
        assert_eq!(leaves, ["(", "x", ")"]);
    }

    #[test]
    fn test_empty_node_takes_cursor_position() {
        let mut b = Builder::new();
        b.start_node("root");
        b.token(t("a", 0, 3));
        b.start_node("gap"); // no tokens
        b.finish_node();
        b.finish_node();
        let root = b.finish().expect("balanced");
        let gap = root.child_nodes().next().expect("gap node");
        assert_eq!(gap.span(), Span::empty(3));
        assert!(gap.is_empty());
    }

    #[test]
    fn test_finish_empty_is_error() {
        let b = Builder::<&str>::new();
        assert_eq!(b.finish(), Err(BuildError::EmptyTree));
    }

    #[test]
    fn test_unclosed_node_is_error() {
        let mut b = Builder::new();
        b.start_node("root");
        b.token(t("a", 0, 1));
        assert_eq!(b.finish(), Err(BuildError::UnclosedNodes));
    }

    #[test]
    fn test_unbalanced_finish_is_error() {
        let mut b = Builder::<&str>::new();
        b.finish_node();
        assert_eq!(b.finish(), Err(BuildError::UnbalancedFinish));
    }

    #[test]
    fn test_token_outside_node_is_error() {
        let mut b = Builder::new();
        b.token(t("a", 0, 1));
        assert_eq!(b.finish(), Err(BuildError::TokenOutsideNode));
    }

    #[test]
    fn test_multiple_roots_is_error() {
        let mut b = Builder::new();
        b.start_node("first");
        b.token(t("a", 0, 1));
        b.finish_node();
        b.start_node("second");
        b.token(t("b", 1, 2));
        b.finish_node();
        assert_eq!(b.finish(), Err(BuildError::MultipleRoots));
    }

    #[test]
    fn test_first_error_wins() {
        let mut b = Builder::<&str>::new();
        b.finish_node(); // UnbalancedFinish recorded
        b.token(t("a", 0, 1)); // would be TokenOutsideNode, but ignored
        assert_eq!(b.finish(), Err(BuildError::UnbalancedFinish));
    }

    #[test]
    fn test_deeply_nested_build_and_finish() {
        let mut b = Builder::new();
        for _ in 0..10_000 {
            b.start_node("link");
        }
        b.token(t("leaf", 0, 1));
        for _ in 0..10_000 {
            b.finish_node();
        }
        let root = b.finish().expect("balanced");
        assert_eq!(root.descendants().count(), 10_000);
        assert_eq!(root.tokens().count(), 1);
    }
}
