//! The verbs about remotes: where a push goes, the mark that decides it,
//! and the forms that set one.
//!
//! Almost none of them can be judged from the picture — a mark, a
//! destination and a row that should have gone are each a few pixels or
//! none at all — so these rows carry more reasoning per verb than the
//! panes do.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // The `+` on a REMOTES band that has gone unavailable around it.
    // The picture holds the band; it cannot hold whether the mark
    // still answers, and a `+` wired to nothing frames exactly like
    // one that opened the form. Folded, `collapsed=` is the second
    // half: the rail's own cell must reach the dialog without
    // putting the list back over whatever the fold was made for.
    Verb {
        name: "nav-add-remote",
        when: &[(
            Arg::Is("folded"),
            "nav_add_remote dialog=true collapsed=true",
        )],
        plain: "nav_add_remote dialog=true",
    },
    // Which remote a push goes to. The mark is one badge on one row:
    // a picture of the band cannot tell "marked" from "the badge was
    // never wired", and `local=true` is what says the repository's own
    // config holds it rather than the machine's.
    Verb {
        name: "push-default",
        when: &[],
        plain: "push_default local=true",
    },
    // Where the toolbar says the push is going. **The picture cannot
    // judge this at all** — the destination lives in the button's
    // tooltip, and a label naming the wrong remote is spelled exactly
    // like one naming the right remote.
    //
    // `--preset forkmark` is the arrangement that separates the two
    // readings: the branch tracks `origin` and its own mark sends
    // pushes to `fork`. A label worked out from `remote.pushDefault`
    // and the upstream says `origin/main` here and the push still
    // goes to the fork; git weighs the branch's mark first
    // (git-config(5); 実測 2.55). `state=elsewhere` is the other half
    // — the ahead/behind counts are about origin, so they say nothing
    // about where this is going.
    // With neither mark set the upstream is the destination, and the
    // counts beside it are about the remote it is really going to.
    Verb {
        name: "push-target",
        when: &[(
            Arg::Is("forkmark"),
            "push_target label=fork/main state=elsewhere branch_mark=fork",
        )],
        plain: "push_target label=origin/main state=ready branch_mark=",
    },
    // The row a remote's own menu is offering. `open=true` because a
    // menu that never opened photographs as the sidebar it stands on,
    // and the count because the row that is gone on the marked remote
    // is the whole of what this reads.
    Verb {
        name: "remote-menu",
        when: &[(Arg::Ends(":marked"), "remote_menu open=true rows=1")],
        plain: "remote_menu open=true rows=2",
    },
    // The form, and whether its box came up in the state the
    // repository is actually in.
    Verb {
        name: "remote-url",
        when: &[(Arg::Ends(":marked"), "remote_url dialog=true box=true")],
        plain: "remote_url dialog=true box=false",
    },
    // The destination list with the mark in it. `marked=` is the field
    // the rows read, not a row of its own — a list drawn with no mark
    // and a list whose mark was never plumbed frame the same way, and
    // the plain `publish-remotes` run is the half with nothing marked.
    Verb {
        name: "publish-remotes-marked",
        when: &[],
        plain: "publish_remotes open=true marked=true",
    },
];
