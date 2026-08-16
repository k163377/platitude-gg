//! Fixtures the three test files share.

use crate::model::{CommitMeta, StrPool};
use crate::oid::Oid;

use super::{GraphBuilder, GraphRow, SegmentKind};

pub(super) fn oid(n: u8) -> Oid {
    let hex = format!("{n:02x}").repeat(20);
    // Test-only helper; the input is always valid hex.
    #[allow(clippy::unwrap_used)]
    Oid::from_hex_str(&hex).unwrap()
}

pub(super) fn commit(pool: &mut StrPool, id: u8, parents: &[u8]) -> CommitMeta {
    CommitMeta {
        oid: oid(id),
        parents: parents.iter().map(|p| oid(*p)).collect(),
        author: pool.intern("Test"),
        author_email: pool.intern("test@example.com"),
        co_authors: Box::new([]),
        body: "".into(),
        time: 1_700_000_000 + i64::from(id),
        subject: format!("commit {id:02x}").into_boxed_str(),
    }
}

/// Renders rows into a deterministic ASCII grid for snapshots:
/// `*` node, `|` through, `/` into-node, `\` out-of-node, `x` both.
pub(super) fn render(rows: &[GraphRow]) -> String {
    let width = rows.iter().map(|r| r.width).max().unwrap_or(0) as usize;
    let mut out = String::new();
    for r in rows {
        let mut cells = vec![' '; width];
        for s in &r.segments {
            let c = &mut cells[s.lane as usize];
            let mark = match s.kind {
                SegmentKind::Through => '|',
                SegmentKind::IntoNode => '/',
                SegmentKind::OutOfNode => '\\',
            };
            *c = match (*c, mark) {
                (' ', m) => m,
                ('/', '\\') | ('\\', '/') => 'x',
                ('|', m) | (m, '|') if m != ' ' => m,
                _ => 'x',
            };
        }
        cells[r.node_lane as usize] = '*';
        let line: String = cells.into_iter().collect();
        out.push_str(&format!(
            "{:>3} {} lane={} color={}\n",
            r.row,
            line.trim_end_matches(' '),
            r.node_lane,
            r.node_color
        ));
    }
    out
}

pub(super) fn build(commits: &[CommitMeta]) -> (Vec<GraphRow>, GraphBuilder) {
    let mut b = GraphBuilder::new();
    let rows = commits.iter().map(|c| b.push(c)).collect();
    (rows, b)
}
