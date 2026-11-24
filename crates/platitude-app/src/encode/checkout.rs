//! Where a move puts HEAD (`branch::CheckoutTarget`), as the page holds the
//! move it is about to send (`GitFacts.checkoutTo`) and hands it to
//! `RepoTab.checkout` as it was read.

use platitude_core::branch::CheckoutTarget;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// The move from its words: `branch` lands on `target`, `remote` makes
/// `local` tracking the remote-tracking ref `target`, `force` moves the
/// local branch `target` to `start` first. `None` for any other word.
pub fn checkout_to(
    kind: &str,
    target: String,
    local: String,
    start: String,
) -> Option<CheckoutTarget> {
    Some(match kind {
        "branch" => CheckoutTarget::Branch { name: target },
        "remote" => CheckoutTarget::Track {
            remote_ref: target,
            local,
        },
        "force" => CheckoutTarget::ForceCreate {
            local: target,
            start,
        },
        _ => return None,
    })
}

/// `kind` (the word above) and `local`, the branch every move lands on,
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
    fn every_move_round_trips_from_its_words() {
        let moves = [
            checkout_to("branch", "main".into(), "main".into(), String::new()),
            checkout_to("remote", "origin/feat".into(), "feat".into(), String::new()),
            checkout_to("force", "feat".into(), "feat".into(), "origin/feat".into()),
        ];
        for asked in moves {
            let held = Optional::new(asked.clone());
            let back = <Optional<CheckoutTarget> as QVariantConvertible>::try_from_qvariant(
                &held.to_qvariant(),
            );
            assert_eq!(back.and_then(|b| b.as_ref().cloned()), asked);
        }
        assert_eq!(
            checkout_to("force", "feat".into(), "x".into(), "origin/feat".into()),
            Some(CheckoutTarget::ForceCreate {
                local: "feat".into(),
                start: "origin/feat".into()
            })
        );
        assert_eq!(
            checkout_to("tag", "v1".into(), "v1".into(), String::new()),
            None
        );
    }
}
