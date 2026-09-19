//! The verbs about remotes: where a push goes, the mark that decides it,
//! and the forms that set one.
//!
//! Almost none of them can be judged from the picture — a mark, a
//! destination and a row that should have gone are each a few pixels or
//! none at all — so these rows carry more reasoning per verb than the
//! panes do.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Fetching a repository that has no tab yet. The three verbs are one
    // road stopped at three places, and none of the three stops can be
    // read off the picture: a box that never opened and one that opened
    // empty are both a window with a card in front of it, a clone that
    // landed and a strip that always had that tab frame alike, and a
    // refusal is one line of git's in a box that is up either way.
    //
    // `folder=` is the half nobody presses for: the box is opened with a
    // destination already in it, and one that opened without one would
    // sit there refusing its own accept button with nothing to say why.
    Verb {
        name: "clone-dialog",
        when: &[],
        plain: "clone dialog=true said=false cloning=false grew=false folder=true",
    },
    // The clone that landed: the box is down, git is not out any more,
    // said nothing against it, and the strip gained the tab it became.
    Verb {
        name: "clone-go",
        when: &[],
        plain: "clone dialog=false said=false cloning=false grew=true",
    },
    // The clone git would not make — the destination is the source's own
    // folder, which is taken. The box stays up and quotes the answer,
    // and no tab was made out of it.
    Verb {
        name: "clone-refused",
        when: &[],
        plain: "clone dialog=true said=true cloning=false grew=false",
    },
    // The `+` on a REMOTES band that has gone unavailable around it.
    // The picture holds the band; it cannot hold whether the mark
    // still answers, and a `+` wired to nothing frames exactly like
    // one that opened the form. Folded, `collapsed=` is the second
    // half: the rail's own cell must reach the dialog and leave
    // the fold standing over whatever it was made for.
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
    // **`:drift` is a wait.** The readings come from
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
    // The two that run a delete. **Judged on the sidebar afterwards**:
    // the read that rebuilds the list answers after the write
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
    // **Read as a set of four.** A name held only here offers the local
    // delete alone (git would report success for deleting nothing —
    // measured), one held only over there offers the remote rows alone
    // (there is nothing to name), one held on both offers all three,
    // and the drifted run is the push row's own second form. Any
    // single one proves none of that.
    Verb {
        name: "tag-menu",
        when: &[
            // The drifted run answers for both halves of one reading:
            // the push row takes its second form, and the two rows that
            // would reach the name over there stand greyed
            // (デザイン規約 §左メニューの所作 の削除の表). They sit
            // together in the line because one run has to assert them
            // together.
            (
                Arg::Ends(":drift"),
                "blocked=remote,both tag_here=true push=true code=push --force held=true lease=",
            ),
            (
                Arg::Starts("v0.9-theirs"),
                "sides=remote local_del=false remote_del=true both_del=false blocked=none",
            ),
            (
                Arg::Ends(":remote"),
                "sides=both local_del=true remote_del=true both_del=true blocked=none",
            ),
        ],
        plain: "sides=here local_del=true remote_del=false both_del=false blocked=none",
    },
    // The far side keeping a branch. **The picture cannot judge this**:
    // a bar saying nothing frames exactly like a bar saying the right
    // thing, and the whole claim is that the sentence was built out of
    // what core classified — which remote, which branch, and that it was
    // a deletion. `why=true` is the other half: the
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
    // config holds it.
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
    // (git-config(5); measured 2.55). `state=elsewhere` is the other half
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
    // The surface a first push raises, read in the turn the button was
    // pressed in.
    //
    // **The half worth judging cannot be keyed.** `dialog=` / `name=` is
    // `true` / `origin` on a repository with no remote and `false` on one
    // with two, and those two runs are told apart by their preset — which
    // this verb's argument cannot carry the way `push-target`'s does,
    // since here it is the branch to publish. What is left is the state
    // the press was taken in, which is the one thing both runs say; it is
    // a thin row, and the line above is why there is no thicker one.
    Verb {
        name: "publish",
        when: &[],
        plain: "publish state=publish",
    },
    // The first push against each of the three shapes a name on the far
    // side can wear. **The picture tells none of them apart**: one bar,
    // one pill, and three characters between `push` and `push -f` —
    // while the frame and the `!` that mark the third are a colour and
    // one glyph.
    //
    // **Read as a set of three.** `taken` is the refusal (a plain push
    // cannot land, so the pill is held and leased against what the far
    // side holds), `carried` the one that lands and moves somebody
    // else's branch on, and `outsider` the one neither answer fits — the
    // name is there and the commit under it is not in this repository.
    // Any one of them alone passes for an implementation that always
    // says the same thing.
    //
    // **`outsider` is the shape an opening's fetch answers away.** The
    // commit lands here, the comparison then reads `refused` like any
    // other diverged name, and the run photographs the wrong fixture and
    // passes on the picture (measured, 10 runs of 10). `verify::seed` is
    // what holds the fetch off; this row is what says so afterwards.
    //
    // `plain` claims only that the check answered at all — an argument
    // nobody has measured passes on any true answer.
    Verb {
        name: "publish-taken",
        when: &[
            (
                Arg::Is("outsider"),
                "publish settled far=unknown code=push hold=false alert=true lease=false theirs=0",
            ),
            (
                Arg::Is("carried"),
                "publish settled far=fast-forward code=push hold=false alert=false lease=false \
                 theirs=0",
            ),
            (
                Arg::OneOf(&["", "taken"]),
                "publish settled far=refused code=push -f hold=true alert=false lease=true \
                 theirs=1",
            ),
        ],
        plain: "publish settled far=",
    },
    // The answer, given — the line goes out in
    // the same turn as the press.
    //
    // Plain, the branch this publishes is one no remote holds under that
    // name, so `far=free` is the fixture pinned: the picture is a bar
    // that has gone, which a repository already holding the name leaves
    // behind just the same. Any other argument is asked only that a
    // classification arrived at all, since the branch it names decides
    // which one.
    Verb {
        name: "publish-go",
        when: &[(
            Arg::Is(""),
            "publish answering far=free unsure=false answerable=true",
        )],
        plain: "publish answering far=",
    },
    // The destination list with the mark in it. `marked=` is the field
    // the rows read — a list drawn with no mark and a list whose mark
    // was never plumbed frame the same way, and
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
    // its question.
    //
    // Every field is read in the turn the ✕ was pressed in, which is
    // why `shut=false` leads: it is what says the reading was taken
    // while the bar was still on screen. `neutral=` is the frame — this
    // question asks where a branch goes, so it wears the accent
    // colour — and `code=push` is the pill's word, which two of the
    // flow's own bindings hold. Both
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
    // config landed, and this says the question was answerable when it
    // was answered.
    Verb {
        name: "set-upstream-go",
        when: &[],
        plain: "there=true answerable=true",
    },
    // The `pull` row, on the two ends of the comparison it stands on and
    // on a remote row that is neither. **The picture cannot say whether
    // the row was offered at all** — a card with one row fewer frames
    // like a card that never had it — so the three forms are read here:
    // the branch the working tree is on, the upstream it is measured
    // against, and a remote nothing here tracks, which must *not* carry
    // the row (a pull there would move a branch the row does not name).
    // `sentence=false` is the other half: both offered rows run the same
    // `git pull`, so both are the chip alone.
    Verb {
        name: "pull-menu",
        when: &[
            (
                Arg::Is(""),
                "pull_menu open=true kind=branch pull=true code=pull sentence=false",
            ),
            (
                Arg::Is("origin/main"),
                "pull_menu open=true kind=remote pull=true code=pull sentence=false",
            ),
        ],
        plain: "pull_menu open=true kind=remote pull=false",
    },
    // The same row where both sides have moved: bringing those together
    // is a choice, so the row is out and the line under the pointer is
    // the only place saying where the choice is made. **None of it is in
    // the picture**: a greyed row and a live one are a shade apart, and
    // no shot carries the sentence a pointer raises.
    //
    // **Two runs, one claim**: the sidebar's row and the graph chip's are
    // handed the answer from the same place, and a run through one says
    // nothing about the other.
    Verb {
        name: "pull-blocked",
        when: &[(
            Arg::Is("chip"),
            "pull_blocked where=chip offered=true blocked=true tip=true \
             says=The branches have diverged, so pick the remote branch and rebase onto it",
        )],
        plain: "pull_blocked where=row offered=true blocked=true tip=true \
                says=The branches have diverged, so pick the remote branch and rebase onto it",
    },
];
