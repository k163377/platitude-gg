use super::*;

/// One row of the shaped list: an index into the source (sixteen bytes,
/// against a `NavItem`'s two hundred), or a folder row — the one thing no
/// source holds — carried whole behind a box so the indexed rows stay
/// small.
///
/// `from` is where in the whole name the shown segment begins, written by
/// whichever tree placed the row: the refs tree (one folder per `/`) and
/// the working tree's (single-child chains compacted) disagree on it.
pub(super) enum Arranged {
    At { at: u32, depth: i32, from: u32 },
    Made(Box<NavItem>),
}

impl platitude_core::mem::Footprint for Arranged {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::At { .. } => 0,
            Self::Made(item) => item.heap_bytes(),
        }
    }
}

/// One row as the view reads it.
#[derive(Clone, Copy)]
pub(super) enum Row<'a> {
    Made(&'a NavItem),
    /// A row of the source, shown at this depth and from this point in
    /// its name.
    Shown {
        of: Entry<'a>,
        depth: i32,
        from: usize,
    },
}

/// What a row answers for one role, before Qt is handed it — a step
/// between the row and `QVariant` so Rust callers (automation, the
/// arranging) read the same answer the delegate is given.
pub(super) enum Value<'a> {
    Said(&'a str),
    Spelled(String),
    Flag(bool),
    Number(i32),
}

impl Value<'_> {
    pub(super) fn variant(&self) -> QVariant {
        match self {
            Self::Said(text) => QVariant::from(&QString::from(*text)),
            Self::Spelled(text) => QVariant::from(&QString::from(text)),
            Self::Flag(flag) => QVariant::from(flag),
            Self::Number(number) => QVariant::from(number),
        }
    }

    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::Said(text) => text,
            Self::Spelled(text) => text,
            Self::Flag(_) | Self::Number(_) => "",
        }
    }

    pub(super) fn flag(&self) -> bool {
        matches!(self, Self::Flag(true))
    }

    pub(super) fn number(&self) -> i32 {
        match self {
            Self::Number(number) => *number,
            Self::Said(_) | Self::Spelled(_) | Self::Flag(_) => 0,
        }
    }
}

/// The roles a delegate reads a row by — this model's own table, put on
/// the wire by `role_names()` and dispatched by `data()` (`qmodel.rs`), so
/// it runs past `NavItem`'s fifteen derived fields.
///
/// The numbers are the declared order, `NavItem`'s fields first. The test
/// at the foot of this file holds the two together: **a role answered
/// under a name the delegate does not ask for draws an empty row**, with
/// no warning.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Name,
    Full,
    OidHex,
    Change,
    Bucket,
    Group,
    OrigPath,
    OrigName,
    IsHead,
    HasRemote,
    OnlyRemote,
    HasPr,
    EolMark,
    Depth,
    Folder,
    /// How far a local branch stands from its upstream. Past the end of
    /// `NavItem`: no folder row is measured, so these carry no field.
    Ahead,
    Behind,
}

impl Role {
    /// Every role the view is handed, in number order.
    ///
    /// No upstream role: no delegate asks for it, and the menus that do
    /// read it by name (`upstream_of` / `upstream_drifted`).
    pub(super) const ALL: [Self; 17] = [
        Self::Name,
        Self::Full,
        Self::OidHex,
        Self::Change,
        Self::Bucket,
        Self::Group,
        Self::OrigPath,
        Self::OrigName,
        Self::IsHead,
        Self::HasRemote,
        Self::OnlyRemote,
        Self::HasPr,
        Self::EolMark,
        Self::Depth,
        Self::Folder,
        Self::Ahead,
        Self::Behind,
    ];

    pub(super) fn of(role: i32) -> Option<Self> {
        usize::try_from(role)
            .ok()
            .and_then(|at| Self::ALL.get(at))
            .copied()
    }

    /// The name the delegate asks for this role by.
    pub(super) fn spelling(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Full => "full",
            Self::OidHex => "oid_hex",
            Self::Change => "change",
            Self::Bucket => "bucket",
            Self::Group => "group",
            Self::OrigPath => "orig_path",
            Self::OrigName => "orig_name",
            Self::IsHead => "is_head",
            Self::HasRemote => "has_remote",
            Self::OnlyRemote => "only_remote",
            Self::HasPr => "has_pr",
            Self::EolMark => "eol_mark",
            Self::Depth => "depth",
            Self::Folder => "folder",
            Self::Ahead => "ahead",
            Self::Behind => "behind",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The derive's names and `Role`'s come from two places; they are held
    /// together as far as `NavItem`'s fields run.
    #[test]
    fn every_field_of_a_made_row_is_the_role_of_the_same_number() {
        let handed = <NavItem as QModelItem>::role_names();
        assert!(
            handed.len() <= Role::ALL.len(),
            "a field with no role of its own can never be asked for",
        );
        for (number, name) in handed {
            let role = Role::of(number);
            assert!(
                role.is_some(),
                "role {number} ({name}) has no answer at all"
            );
            assert_eq!(
                role.map(Role::spelling),
                Some(name.as_str()),
                "role {number} is answered under another name",
            );
        }
    }

    /// Two roles spelled alike put one name on the wire twice; the
    /// delegate silently reads whichever `role_names()` kept.
    #[test]
    fn no_two_roles_answer_under_one_name() {
        let mut spellings: Vec<&str> = Role::ALL.iter().map(|role| role.spelling()).collect();
        let declared = spellings.len();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), declared, "two roles share one name");
    }
}
