//! A branch on a remote as QML reads it: the two halves of a
//! remote-tracking name, cut once by core (`GitFacts.remoteBranchOf`).

use platitude_core::remote::RemoteBranch;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// Under core's own names: `remote` / `branch`.
impl Record for RemoteBranch {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("remote", &self.remote)
            .put("branch", &self.branch)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            remote: field(map, "remote")?,
            branch: field(map, "branch")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::One;
    use qtbridge::QVariantConvertible;

    #[test]
    fn a_remote_branch_round_trips_with_a_slash_in_either_half() {
        let far = One::new(RemoteBranch {
            remote: "team/fork".into(),
            branch: "feature/日本語".into(),
        });
        let back =
            <One<RemoteBranch> as QVariantConvertible>::try_from_qvariant(&far.to_qvariant());
        assert_eq!(back.map(|b| (*b).clone()), Some((*far).clone()));
    }
}
