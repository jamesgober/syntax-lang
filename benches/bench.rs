//! Benchmarks for the build and traversal paths as a tree grows.
//!
//! Run with `cargo bench`. Each group sweeps the token count so a regression shows
//! up as a change in the per-element slope, not just one point.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use syntax_lang::{Builder, Node, Span, Token, TokenKind};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Group,
    Word,
    Space,
}

impl TokenKind for Kind {
    fn is_trivia(&self) -> bool {
        matches!(self, Kind::Space)
    }
}

/// Builds a tree of `leaves` tokens grouped in runs of eight, alternating
/// significant and trivia tokens, each one byte wide.
fn build_tree(leaves: u32) -> Node<Kind> {
    let mut b = Builder::new();
    b.start_node(Kind::Group);
    let mut open = false;
    for i in 0..leaves {
        if i % 8 == 0 {
            if open {
                b.finish_node();
            }
            b.start_node(Kind::Group);
            open = true;
        }
        let kind = if i % 2 == 0 { Kind::Word } else { Kind::Space };
        // Each leaf is one byte wide, so its start offset is its index.
        b.token(Token::new(kind, Span::new(i, i + 1)));
    }
    if open {
        b.finish_node();
    }
    b.finish_node();
    b.finish().expect("balanced")
}

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    for &leaves in &[64u32, 512, 4096] {
        group.bench_with_input(
            BenchmarkId::from_parameter(leaves),
            &leaves,
            |bencher, &n| {
                bencher.iter(|| build_tree(black_box(n)));
            },
        );
    }
    group.finish();
}

fn bench_tokens(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokens");
    for &leaves in &[64u32, 512, 4096] {
        let tree = build_tree(leaves);
        group.bench_with_input(
            BenchmarkId::from_parameter(leaves),
            &tree,
            |bencher, tree| {
                bencher.iter(|| tree.tokens().map(|t| t.span().len()).sum::<u32>());
            },
        );
    }
    group.finish();
}

fn bench_descendants(c: &mut Criterion) {
    let mut group = c.benchmark_group("descendants");
    for &leaves in &[64u32, 512, 4096] {
        let tree = build_tree(leaves);
        group.bench_with_input(
            BenchmarkId::from_parameter(leaves),
            &tree,
            |bencher, tree| {
                bencher.iter(|| tree.descendants().count());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_build, bench_tokens, bench_descendants);
criterion_main!(benches);
