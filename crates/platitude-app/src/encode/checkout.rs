//! Where a move puts HEAD (`branch::CheckoutTarget`), as the page holds the
//! move it is about to send (`GitFacts.switchMove` / `moveOnto`) and hands it to
//! `RepoTab.checkout` as it was read.

use platitude_core::branch::CheckoutTarget;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field, optional_record};

/// What a press on a ref adds up to (`offers::switch_action`), as the page
/// branches on it: the action's word (`holder` / `switch` / `materialize`
/// / `move`, `""` for none) and, where the move is a plain checkout, the
/// checkout it sends (`SwitchAction::checkout`) — left out for the others.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwitchMove {
    pub action: String,
    pub checkout: Option<CheckoutTarget>,
}

impl Record for SwitchMove {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("action", &self.action)
            .put_record("checkout", self.checkout.as_ref())
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            action: field(map, "action")?,
            checkout: optional_record(map, "checkout")?,
        })
    }
}
/// `kind` — `branch` lands on a local branch, `remote` makes `local`
/// tracking the remote-tracking ref, `force` moves `local` to `start`
/// first — and `local`, the branch every move lands on,
/// beside the variant's own parts (`remoteRef`, `start`).
impl Record for CheckoutTarget {
    fn to_map(&self) -> QVariantMap {
        match self {
            Self::Branch { name } => Fields::new().put("kind", "branch").put("local", name),
            Self::Track { remote_ref, local } => Fields::new()
                .put("kind", "remote")
                .put("remoteRef", remote_ref)
                .put("local", local),
            Self::ForceCreate { local, start } => Fields::new()
                .put("kind", "force")
                .put("local", local)
                .put("start", start),
        }
        .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        let kind: String = field(map, "kind")?;
        let local: String = field(map, "local")?;
        Some(match kind.as_str() {
            "branch" => Self::Branch { name: local },
            "remote" => Self::Track {
                remote_ref: field(map, "remoteRef")?,
                local,
            },
            "force" => Self::ForceCreate {
                local,
                start: field(map, "start")?,
            },
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Optional;
    use qtbridge::QVariantConvertible;

    #[test]
    fn every_move_round_trips() {
        let moves = [
            CheckoutTarget::Branch {
                name: "main".into(),
            },
            CheckoutTarget::Track {
                remote_ref: "origin/feat".into(),
                local: "feat".into(),
            },
            CheckoutTarget::ForceCreate {
                local: "feat".into(),
                start: "origin/feat".into(),
            },
        ];
        for asked in moves {
            let held = Optional::some(asked.clone());
            let back = <Optional<CheckoutTarget> as QVariantConvertible>::try_from_qvariant(
                &held.to_qvariant(),
            );
            assert_eq!(back.and_then(|b| b.as_ref().cloned()), Some(asked));
        }
    }

    /// The checkout rides the action where there is one, and a move that
    /// sends none reads back with none.
    #[test]
    fn a_switch_move_round_trips_with_and_without_its_checkout() {
        for asked in [
            SwitchMove {
                action: "materialize".into(),
                checkout: Some(CheckoutTarget::Track {
                    remote_ref: "origin/feat".into(),
                    local: "feat".into(),
                }),
            },
            SwitchMove {
                action: "holder".into(),
                checkout: None,
            },
        ] {
            assert_eq!(SwitchMove::from_map(&asked.to_map()), Some(asked));
        }
    }
}
