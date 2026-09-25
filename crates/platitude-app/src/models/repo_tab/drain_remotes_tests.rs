//! What the remotes out of the feed leave the properties QML reads
//! saying — where pushes go and which remote wears the origin mark
//! (`TabMsg::Remotes` in `drain::absorb`).

use super::*;

fn remotes(names: &[&str], push_default: &str, checkout_default: &str) -> TabMsg {
    TabMsg::Remotes {
        names: names.iter().map(|name| name.to_string()).collect(),
        urls: names.iter().map(|_| String::new()).collect(),
        push_default: push_default.into(),
        push_default_local: true,
        checkout_default: checkout_default.into(),
    }
}

/// A remote is origin only where both keys name it. A rename in a
/// terminal carries the push's key to the new name and leaves the
/// checkout one on the old, and that remote still has the mark to
/// finish — while its badge goes on following the push's key.
#[test]
fn only_a_remote_both_keys_name_is_marked_as_origin() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![remotes(&["fork", "origin"], "fork", "fork")]);
    assert_eq!(tab.marked_origin, "fork");

    tab.absorb(vec![remotes(&["home", "origin"], "home", "fork")]);
    assert_eq!(tab.push_default, "home", "the badge follows the push's key");
    assert_eq!(tab.marked_origin, "", "the mark is half set");

    tab.absorb(vec![remotes(&["origin"], "", "")]);
    assert_eq!(tab.marked_origin, "");
}

/// A push key naming a remote this repository does not have is left out,
/// and the checkout key naming the same missing remote does not bring it
/// back as origin.
#[test]
fn keys_naming_a_missing_remote_mark_nothing() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![remotes(&["origin"], "gone", "gone")]);
    assert_eq!(tab.push_default, "");
    assert_eq!(tab.marked_origin, "");
}
