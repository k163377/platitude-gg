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
    // The tag menu's push row, whose whole shape — chip, hold, colour —
    // comes off what a remote was last heard to carry. The two forms
    // differ by five glyphs in an otherwise identical card, and the one
    // thing no picture holds is the commit the lease is pinned to: a
    // forced push wearing an empty lease is a plain force, which is the
    // failure this row exists to prevent (デザイン規約 §相手の履歴を置き換える).
    //
    // **`:drift` is a wait, not a claim.** The readings come from
    // `ls-remote --tags` well after the fetch they ride out with, so a
    // run that stopped at the fetch would photograph the plain row and
    // call it the forced one. Both halves are needed: the plain side
    // alone passes for an implementation that never reads the remotes,
    // and the drifted side alone for one that always forces.
    Verb {
        name: "push-tag",
        when: &[(
            Arg::Ends(":drift"),
            "push=true code=push --force held=true lease=",
        )],
        plain: "push=true code=push held=false lease= ",
    },
    // The two that run a delete. **Judged on the sidebar afterwards**, not on
    // the write: the read that rebuilds the list answers after the write
    // does, so a run stopped at the write barrier photographs the row it
    // just deleted and passes. `was=` / `sides=` is that name's own
    // reading either side of the press — a count would not do, since the
    // remote half of a name held on both sides takes no row away.
    Verb {
        name: "delete-remote-tag",
        when: &[(
            Arg::Starts("v0.9-theirs"),
            "tag_gone tag=v0.9-theirs was=remote sides= row=-1",
        )],
        plain: "was=both sides=here",
    },
    Verb {
        name: "delete-tag-both",
        when: &[],
        plain: "was=both sides= row=-1",
    },
    // The three deletes a tag's name can want, told apart by which of
    // them is drawn at all — a card missing one frames exactly like a
    // card that never offered it. `sides=` is the reading they come off,
    // so a wrong row and a wrong reading are not the same failure.
    //
    // **Read as a set of four.** A name held only here must not offer
    // the remote rows (git would report success for deleting nothing —
    // 実測), one held only over there must not offer the local delete
    // (there is nothing to name), one held on both offers all three, and
    // the drifted run is the push row's own second form. Any single one
    // proves none of that.
    Verb {
        name: "tag-menu",
        when: &[
            (
                Arg::Ends(":drift"),
                "push=true code=push --force held=true lease=",
            ),
            (
                Arg::Starts("v0.9-theirs"),
                "sides=remote local_del=false remote_del=true both_del=false",
            ),
            (
                Arg::Ends(":remote"),
                "sides=both local_del=true remote_del=true both_del=true",
            ),
        ],
        plain: "sides=here local_del=true remote_del=false both_del=false",
    },
    // The far side keeping a branch. **The picture cannot judge this**:
    // a bar saying nothing frames exactly like a bar saying the right
    // thing, and the whole claim is that the sentence was built out of
    // what core classified — which remote, which branch, and that it was
    // a deletion rather than a send. `why=true` is the other half: the
    // far side's own words came across as well, and a report without
    // them is a report of nothing (デザイン規約 §可否・警告の出し場所).
    //
    // It is also the run that proves no error was raised — `log=false`
    // (the panel did not come up over the same news) and `wrong=false`
    // (the mark in the corner is not calling it one). A window that
    // opened the log and closed it again frames exactly like one that
    // never opened it, so neither is a claim the picture can make.
    // `tone=danger` is what a press that was turned down and is over wears
    // (デザイン規約 §答えの要らない報せ) — and the hairline that says so is two
    // pixels of colour, which no picture is judged on.
    Verb {
        name: "remote-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=origin would not delete main",
    },
    // The same refusal over a tag, which used to be the one that did not
    // come down in the bar at all: `push_tag` and `delete_remote_tag`
    // returned a plain failure whatever the far side had said, so the log
    // went up over news the branch beside it reported quietly. `said=`
    // naming the tag is the whole of the claim — a run that classified
    // nothing frames identically, with the panel up instead.
    Verb {
        name: "tag-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=origin would not delete v1.0",
    },
    // A push git itself will not send. **The next move is already made**
    // — the session fetches on this one — so it is a report like the
    // others, and `why=true` is git's own advice arriving with its
    // `hint:` framing off.
    Verb {
        name: "push-outdated",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=main was not sent to origin",
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
    // What the bar wears on the way back up, after the ✕. **No picture
    // of this run can hold it**: the bar takes 200ms to go and the shot
    // is taken once it has, so a bar that spent the whole of it as an
    // empty band in the wrong colour frames exactly like one that kept
    // its question (2026-08-27 ユーザー報告).
    //
    // Every field is read in the turn the ✕ was pressed in, which is
    // why `shut=false` leads: it is what says the reading was taken
    // while the bar was still on screen. `neutral=` is the frame — this
    // question asks where a branch goes rather than for consent, so it
    // wears the accent colour and not `warning` — and `code=push` is
    // the pill's word, which two of the flow's own bindings hold. Both
    // fall off a bar re-dressed at the press, and so do the words.
    Verb {
        name: "publish-dismiss",
        when: &[],
        plain: "ask_dismissed shut=false words=true detail=true code=push neutral=true",
    },
    // The question about what a branch is measured against. **The
    // picture holds two boxes and a pill and cannot say what any of them
    // are worth**: whether the name answered with is one this repository
    // actually holds is the whole of what decides the press, and a pill
    // greyed for that reason frames like one greyed for any other.
    //
    // **`:missing` is the refused half.** The reserved name is one no
    // demo repository carries, so the run photographs the frame and the
    // line a name that was never fetched puts up — and both halves are
    // needed: the plain side alone passes for an implementation that
    // never checks, and the refused side alone for one that always
    // refuses.
    Verb {
        name: "set-upstream",
        when: &[(Arg::Ends(":missing"), "there=false answerable=false")],
        plain: "there=true answerable=true",
    },
    // Answered, and judged before the press: the write barrier says the
    // config landed, and this says the question was answerable rather
    // than answered past a dead pill.
    Verb {
        name: "set-upstream-go",
        when: &[],
        plain: "there=true answerable=true",
    },
];
