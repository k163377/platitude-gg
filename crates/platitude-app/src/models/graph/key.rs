//! A row's identity across passes: what a page holds of the row it is
//! reading and of the row under the hand, and asks the rows for again once
//! they have moved (`GraphModel.keyAt` / `rowOfKey`).
//!
//! The rows are of two kinds, and each is found its own way: a commit by
//! its id, a row of uncommitted work by whose it is. The key carries the
//! kind, so the one question "where is it now" has one answer for all of
//! them (`GraphModel::row_by_key`) — the page never picks among lookups.

use qtbridge::qtbridge_type_lib::QVariantMap;

use platitude_core::Oid;

/// Which row, told apart as the rows are: a commit by its id; a row of
/// uncommitted work by whose it is — this window's own, or another
/// worktree's by its path as git printed it (`CarriedRow::path`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RowKey {
    Commit(Oid),
    Here,
    Other(String),
}

impl RowKey {
    /// The word QML branches on.
    fn kind(&self) -> &'static str {
        match self {
            Self::Commit(_) => "commit",
            Self::Here => "here",
            Self::Other(_) => "worktree",
        }
    }
}

/// QML reads `kind` (`commit` / `here` / `worktree`) and the field that
/// kind names: `oid` on a commit, `path` on another worktree's row. The
/// field a kind does not name is empty.
impl crate::encode::Record for RowKey {
    fn to_map(&self) -> QVariantMap {
        let (oid, path) = match self {
            Self::Commit(oid) => (oid.to_hex(), String::new()),
            Self::Here => (String::new(), String::new()),
            Self::Other(path) => (String::new(), path.clone()),
        };
        crate::encode::Fields::new()
            .put("kind", &self.kind().to_string())
            .put("oid", &oid)
            .put("path", &path)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        use crate::encode::field;
        let kind: String = field(map, "kind")?;
        match kind.as_str() {
            "commit" => crate::encode::oid_arg(&field::<String>(map, "oid")?).map(Self::Commit),
            "here" => Some(Self::Here),
            "worktree" => {
                let path: String = field(map, "path")?;
                (!path.is_empty()).then_some(Self::Other(path))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Record;

    /// Every kind comes back out of Qt's map as it went in; a commit named
    /// by no id, a worktree named by no path and an unknown word do not.
    #[test]
    fn a_key_round_trips_and_a_malformed_one_is_none() {
        let oid = Oid::from_hex_str(&"ab".repeat(20)).unwrap();
        for key in [
            RowKey::Commit(oid),
            RowKey::Here,
            RowKey::Other("/tmp/seat".into()),
        ] {
            assert_eq!(RowKey::from_map(&key.to_map()), Some(key));
        }
        let broken = |kind: &str, oid: &str, path: &str| {
            crate::encode::Fields::new()
                .put("kind", &kind.to_string())
                .put("oid", &oid.to_string())
                .put("path", &path.to_string())
                .done()
        };
        assert_eq!(RowKey::from_map(&broken("commit", "", "")), None);
        assert_eq!(RowKey::from_map(&broken("worktree", "", "")), None);
        assert_eq!(RowKey::from_map(&broken("stash", "", "")), None);
    }
}
