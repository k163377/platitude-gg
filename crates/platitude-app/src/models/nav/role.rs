use super::*;

/// One row of the shaped list.
///
/// Sixteen bytes where a row of the source can be pointed at, against the
/// two hundred a `NavItem` costs; a folder row is the one thing no source
/// holds, so it is the one thing carried whole (behind a box, so the
/// pointed-at rows are not all widened to hold one).
///
/// `from` is where in the whole name the segment this row shows begins —
/// worked out by whichever tree placed the row, because the two do not
/// agree on it: the refs tree indents one folder per `/`, while the
/// working tree's compacts a chain of single-child folders into one row
/// and leaves the file showing only its last segment.
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
            Self::Said(text) => QVariant::from(*text),
            Self::Spelled(text) => QVariant::from(text),
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

/// The roles a delegate reads a row by.
///
/// The numbers are `NavItem`'s fields in order, which is what
/// `#[derive(QModelItem)]` hands the view as names. The test at the foot
/// of this file holds the two together: **a role answered under a name
/// the delegate does not ask for draws nothing at all**, and says nothing
/// about it — no warning, no error, an empty row.
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
}

impl Role {
    /// Every role the view is handed, in the order their numbers run —
    /// the order `NavItem` declares its fields in, one for one (the test
    /// at the foot of this file holds them together).
    ///
    /// A branch's upstream is not a role: no delegate asks for it, and
    /// the menus that do ask by name (`upstream_of` / `upstream_drifted`)
    /// read the snapshot's own row through its index.
    pub(super) const ALL: [Self; 15] = [
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names the delegate asks by and the numbers `data` is called
    /// with come from two places; a role that answers under the wrong one
    /// draws an empty row and reports nothing, so they are pinned here.
    #[test]
    fn every_role_the_view_is_handed_is_answered_by_the_same_name() {
        let handed = <NavItem as QModelItem>::role_names();
        assert_eq!(handed.len(), 15, "the view is handed one role per field");
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
}
