//! Pure rules QML asks by value: stateless slots over platitude-core and
//! this crate's wire formats. Every slot reads only its arguments, so a
//! binding that passes its own properties re-evaluates exactly when they
//! change — the qproperty rule (.claude/rules/app-ui.md) is about slots
//! that read state QML cannot see, and there is none here to read.

use qtbridge::qobject;

use super::qml_register;

/// The singleton every QML file can ask a rule of without wiring a model
/// through: which id is the synthetic WIP row's, which chips a gone set
/// leaves standing, what a remote ref or a typed identity splits into,
/// what a push can do.
#[derive(Default)]
pub struct GitFacts;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl GitFacts {
    /// Whether a hex id is the synthetic uncommitted-changes row's
    /// all-zero sentinel (`Oid::zero_like`). Empty is "no id at all".
    #[qslot]
    fn wip_oid(&self, hex: String) -> bool {
        platitude_core::oid::Oid::hex_is_zero(&hex)
    }

    /// The chip records still standing once the page's gone set has
    /// spoken (`encode::labels_shown`): `packed` as the row carries it,
    /// less the chips `gone` names.
    #[qslot]
    fn labels_shown(&self, packed: String, gone: String) -> String {
        crate::encode::labels_shown(&packed, &gone)
    }

    /// The ref kind a chip record's letter names, in the word the menus
    /// branch on — `""` for the HEAD marker, which names nothing to act
    /// on.
    #[qslot]
    fn record_kind(&self, record: String) -> String {
        crate::encode::label_kind_word(&record).to_string()
    }

    /// The name on a chip record (`encode::label_name_of`).
    #[qslot]
    fn record_name(&self, record: String) -> String {
        crate::encode::label_name_of(&record).to_string()
    }

    /// How many records a packed list holds (`encode::RECORD_SEP`
    /// between them); `""` holds none.
    #[qslot]
    fn record_count(&self, packed: String) -> i32 {
        if packed.is_empty() {
            return 0;
        }
        packed.split(crate::encode::RECORD_SEP).count() as i32
    }

    /// The remote half of a remote-tracking name (`origin/main`), read
    /// against the configured names (`\u{1f}`-packed,
    /// `RepoTab.remoteNames`): the longest configured name wins — a
    /// remote's own name may contain `/` — and where none owns the ref
    /// the first slash answers, so the gesture still acts and git gets
    /// to refuse loudly (`refs::split_remote_ref_or_first_slash` — the
    /// list may still be loading, or the remote may be gone from
    /// configuration while its refs remain). `""` only where there is no
    /// slash at all: that name is not a remote branch.
    #[qslot]
    fn remote_of_ref(&self, full: String, remote_names: String) -> String {
        platitude_core::refs::split_remote_ref_or_first_slash(&full, packed_names(&remote_names))
            .map(|(remote, _)| remote.to_string())
            .unwrap_or_default()
    }

    /// The branch half of the same cut. A name no slash divides is all
    /// branch — the shape a rename box is typed in.
    #[qslot]
    fn branch_of_ref(&self, full: String, remote_names: String) -> String {
        match platitude_core::refs::split_remote_ref_or_first_slash(
            &full,
            packed_names(&remote_names),
        ) {
            Some((_, branch)) => branch.to_string(),
            None => full,
        }
    }

    /// What a push of the current branch can do
    /// (`platitude_core::remote::push_standing`), in the word
    /// `PublishFlow.pushState` branches on. The tab-lifecycle half of
    /// "closed" stays the caller's: this answers for the repository, not
    /// for whether a page is open on it.
    #[qslot]
    #[expect(clippy::too_many_arguments)]
    fn push_standing(
        &self,
        detached: bool,
        branch: String,
        upstream: String,
        upstream_tracked: bool,
        ahead: i32,
        behind: i32,
        push_default: String,
        remote_names: String,
    ) -> String {
        platitude_core::remote::push_standing(
            detached,
            &branch,
            &upstream,
            upstream_tracked,
            ahead,
            behind,
            &push_default,
            packed_names(&remote_names),
        )
        .as_str()
        .to_string()
    }

    /// The address half of a typed identity
    /// (`platitude_core::trailers::split_identity`); `""` where nothing
    /// in the text reads as one.
    #[qslot]
    fn identity_email_of(&self, text: String) -> String {
        platitude_core::trailers::split_identity(&text).1
    }

    /// The name half of a typed identity; `""` where nothing stands
    /// before an address.
    #[qslot]
    fn identity_name_of(&self, text: String) -> String {
        platitude_core::trailers::split_identity(&text).0
    }

    /// The last segment of a path, whichever separator wrote it — the
    /// name a working copy's folder is shown by (`urlpath::path_leaf`).
    #[qslot]
    fn path_leaf(&self, path: String) -> String {
        crate::urlpath::path_leaf(&path).to_string()
    }
}

/// The names in a `\u{1f}`-packed list, none for `""`.
fn packed_names(packed: &str) -> impl Iterator<Item = &str> {
    packed.split('\u{1f}').filter(|name| !name.is_empty())
}

qml_register!(GitFacts, "GitFacts", singleton = true);
