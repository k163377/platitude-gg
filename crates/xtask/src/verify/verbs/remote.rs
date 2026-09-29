//! The verbs about remotes: where a push goes, the mark that decides it,
//! and the forms that set one.
//!
//! Almost none of them can be judged from the picture: a mark, a
//! destination and a row that should have gone are each a few pixels or
//! none at all.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Cloning: three verbs, one road stopped at three places. `folder=`:
    // the box opens with a destination already in it, or it would refuse
    // its own accept button with nothing to say why.
    Verb {
        name: "clone-dialog",
        when: &[],
        plain: "clone dialog=true said=false cloning=false grew=false folder=true",
    },
    // The clone that landed: box down, git done and silent, and the strip
    // gained its tab.
    Verb {
        name: "clone-go",
        when: &[],
        plain: "clone dialog=false said=false cloning=false grew=true",
    },
    // Refused (the destination is the source's own folder): the box stays
    // up quoting git, and no tab.
    Verb {
        name: "clone-refused",
        when: &[],
        plain: "clone dialog=true said=true cloning=false grew=false",
    },
    // The `+` on a REMOTES band gone unavailable around it: a `+` wired to
    // nothing frames like one that opened the form. `folded` reaches the
    // dialog from the rail's cell and must leave the fold standing.
    Verb {
        name: "nav-add-remote",
        when: &[(
            Arg::Is("folded"),
            "nav_add_remote dialog=true collapsed=true",
        )],
        plain: "nav_add_remote dialog=true",
    },
    // The tag menu's push row, whose chip, hold and colour come off what a
    // remote was last heard to carry; a forced push with an empty lease is
    // a plain force (デザイン規約 §相手の履歴を置き換える). `:drift` waits:
    // the readings come from `ls-remote --tags` after the fetch, so
    // stopping at the fetch photographs the plain row. The pair: plain
    // alone passes an implementation that never reads the remotes, drift
    // alone one that always forces.
    Verb {
        name: "push-tag",
        when: &[(
            Arg::Ends(":drift"),
            "push=true code=push --force held=true lease=",
        )],
        plain: "push=true code=push held=false lease= ",
    },
    // The two that run a delete, judged on the sidebar afterwards: the
    // rebuild answers after the write, so the barrier alone photographs
    // the deleted row. `was=` / `sides=` are the name's reading either
    // side of the press, not a count — the remote half of a name held on
    // both sides takes no row away.
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
    // The three deletes a tag's name can want; `sides=` is the reading
    // they come off, so a wrong row and a wrong reading fail apart. Which
    // rows a side offers is core's
    // (`offers::each_delete_row_needs_the_side_it_names`) and the card's
    // use of it `tests/qml/tst_tagcard.qml`'s; these claim the two lookups
    // the menu makes (`RefRowMenu.tagFacts`): `:remote` that `tagSides`
    // arrived (unread answers `here`, one row instead of three), `:drift`
    // that `remoteTagDrift` did. No run for a name on one side only: core
    // and the card test already decide those.
    Verb {
        name: "tag-menu",
        when: &[
            // Every remote read first. Fork and mirror carry the name and
            // origin does not: no remote can be picked unasked, so the row
            // stands out and names who has it (`tag_reach`, the line after
            // `tag_menu`).
            (
                Arg::Is("v3.0-pair:read"),
                "tag_reach reach= offered=true blocked=true push=to origin del=v3.0-pair \
                 why=Several remotes have it: fork, mirror",
            ),
            // Only fork carries it: fork is reached, and the row says so.
            (
                Arg::Is("v0.9-theirs:read"),
                "tag_reach reach=fork offered=true blocked=false push=to origin \
                 del=v0.9-theirs from fork why=",
            ),
            // The greyed `push --delete` row's reason, forced out: the
            // sentence every copy of a name standing apart says, naming
            // the remote the row would reach (`Words.differsFrom`).
            (
                Arg::Ends(":drift:tip"),
                "delete_blocked code=push --delete tip=true holder= reason=Differs from origin",
            ),
            // One reading, both halves in one line: the push row's second
            // form, and the two rows reaching the remote name greyed
            // (デザイン規約 §左メニューの所作 の削除の表).
            (
                Arg::Ends(":drift"),
                "blocked=remote,both tag_here=true push=true code=push --force held=true lease=",
            ),
            (
                Arg::Ends(":remote"),
                "sides=both local_del=true remote_del=true both_del=true blocked=none",
            ),
        ],
        plain: "sides=here local_del=true remote_del=false both_del=false blocked=none",
    },
    // The graph row's menu on a tag's chip that draws one remote's reading
    // (`v3.2-split` on HEAD's row is mirror's, the copy here standing a row
    // down): that remote is named on its own, so the card pushes to it and
    // deletes from it (`CommitMenuState.tagFacts` → `tagAimAt`).
    Verb {
        name: "tag-chip-menu",
        when: &[(
            Arg::Is("v3.2-split@0"),
            "tag_reach reach=mirror offered=true blocked=false push=to mirror \
             del=v3.2-split from mirror why=",
        )],
        plain: "tag_reach reach=",
    },
    // The far side keeping a branch: `said=` is the sentence built from
    // what core classified (remote, branch, a deletion), `why=true` the
    // far side's own words (デザイン規約 §可否・警告の出し場所).
    // `log=false`: the panel did not come up over the same news;
    // `wrong=false`: the corner's mark is not calling it an error.
    // `tone=danger` is a press turned down and over
    // (デザイン規約 §答えの要らない報せ).
    Verb {
        name: "remote-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=origin would not delete main",
    },
    // The same refusal over a tag: a `push_tag` / `delete_remote_tag`
    // returning a plain failure would put the log up over news the branch
    // reports quietly. `said=` naming the tag is the claim.
    Verb {
        name: "tag-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=origin would not delete v1.0",
    },
    // A push git will not send. The session fetches on it, so it is a
    // report like the others; `why=true` is git's advice, `hint:` off.
    Verb {
        name: "push-outdated",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=main was not sent to origin",
    },
    // Which remote a push goes to: `local=true` is the repository's own
    // config holding the mark. The run ends once `checkout.defaultRemote`
    // names the remote too (the two keys arrive in one snapshot), which is
    // `origin=` on the line.
    Verb {
        name: "push-default",
        when: &[],
        plain: "push_default local=true",
    },
    // Where the toolbar says the push is going (the button's tooltip).
    // `--preset forkmark`: the branch tracks `origin` and its own mark
    // sends pushes to `fork`, which git weighs first (git-config(5)) — a
    // label from `remote.pushDefault` and the upstream says `origin/main`
    // here. `state=elsewhere`: the fork holds nothing yet, so nothing here
    // tracks the branch over there. `--preset forkdiverged`: it does, and
    // the fork's own counts decide — `ready` against origin, `diverged`
    // against the fork. With neither mark set the upstream is the
    // destination.
    Verb {
        name: "push-target",
        when: &[
            (
                Arg::Is("forkmark"),
                "push_target label=fork/main state=elsewhere branch_mark=fork",
            ),
            (
                Arg::WithPreset("forkdiverged"),
                "push_target label=fork/main state=diverged branch_mark=fork",
            ),
        ],
        plain: "push_target label=origin/main state=ready branch_mark=",
    },
    // A hand on that button: `tip=true` the words up (`tipDelayMs` waited
    // out), `mode=` which sentence (`ready` what a push sends, `diverged`
    // what the overwrite drops), and the counts it says, per fixture —
    // `forkdiverged`'s are the fork's, where origin's read `ahead=1
    // behind=0`.
    Verb {
        name: "push-hover",
        when: &[
            (
                Arg::WithPreset("diverged"),
                "push_hover tip=true mode=diverged ahead=1 behind=1",
            ),
            (
                Arg::WithPreset("forkdiverged"),
                "push_hover tip=true mode=diverged ahead=1 behind=1",
            ),
        ],
        plain: "push_hover tip=true mode=ready ahead=1 behind=0",
    },
    // A remote's own menu: `rows=` because the marked remote's menu loses
    // a row, `open=true` since an unopened menu photographs as the sidebar.
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
    // The surface a first push raises, read in the turn of the press. Only
    // the state both presets share is claimed: `dialog=` / `name=` split
    // by preset (`noremote` opens the dialog on `origin`, `unpublished`
    // does not), a pair `Arg::WithPreset` could key.
    Verb {
        name: "publish",
        when: &[],
        plain: "publish state=publish",
    },
    // The first push against the three shapes a name on the far side can
    // wear, read as a set (any one alone passes an implementation that
    // always says the same): `taken` refused (pill held, leased against
    // the far side), `carried` lands and moves somebody else's branch on,
    // `outsider` the name is there but its commit is not in this
    // repository. An opening fetch would bring that commit in and turn
    // `outsider` into `refused`; `verify::seed` holds it off and this row
    // says so. `plain` claims only that the check answered.
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
    // The refusal with a hand on its pill, whose tip alone says what the
    // overwrite drops; `far=` / `theirs=` pin the fixture behind the count.
    Verb {
        name: "publish-tip",
        when: &[(
            Arg::OneOf(&["", "taken"]),
            "publish_tip tip=true far=refused theirs=1",
        )],
        plain: "publish_tip tip=true far=",
    },
    // The answer given; the line goes out in the turn of the press. With
    // no argument the branch is one no remote holds, so `far=free` pins
    // the fixture (the gone bar looks the same either way); other branches
    // claim only that a classification arrived.
    Verb {
        name: "publish-go",
        when: &[(
            Arg::Is(""),
            "publish answering far=free unsure=false answerable=true",
        )],
        plain: "publish answering far=",
    },
    // The destination list with the mark in it: `marked=` is the field the
    // rows read. The plain `publish-remotes` run is the unmarked half.
    Verb {
        name: "publish-remotes-marked",
        when: &[],
        plain: "publish_remotes open=true marked=true",
    },
    // What the bar wears on its way out after the ✕: the shot comes once
    // it has gone, so an empty band in the wrong colour frames like one
    // that kept its question. Read in the turn of the press — `shut=false`
    // says the bar was still up. `neutral=` (the accent: it asks where a
    // branch goes) and `code=push` come off the flow's bindings; both fall
    // off a bar re-dressed at the press, and so do the words.
    Verb {
        name: "publish-dismiss",
        when: &[],
        plain: "ask_dismissed shut=false words=true detail=true code=push neutral=true",
    },
    // The question of what a branch is measured against: `there=` is
    // whether the typed name is one this repository holds. `:missing`
    // types a name no demo repository carries, answerable all the same —
    // the next push makes it (デザイン規約 §ブランチが測られる相手を決める).
    // The pair: plain alone passes an implementation that reads nothing,
    // `:missing` alone one that never looks at the refs.
    Verb {
        name: "set-upstream",
        when: &[(Arg::Ends(":missing"), "there=false answerable=true")],
        plain: "there=true answerable=true",
    },
    // The name box's list: the box opens one only when it has rows
    // (`AppCombo.hasList`), so `open=true` is the list plumbed. What is in
    // it is the picture's (overlay.png), with `rows=` to count by.
    Verb {
        name: "set-upstream-list",
        when: &[],
        plain: "open=true",
    },
    // Answered, and judged before the press: the write barrier says the
    // config landed, and this says the question was answerable when it
    // was answered.
    Verb {
        name: "set-upstream-go",
        when: &[],
        plain: "there=true answerable=true",
    },
    // The first push's question answered from its name box instead of the
    // pill (デザイン規約 §立っている質問は 1 か所で聞く), on `publish-go`'s
    // line; the write barrier says the key reached the pill's run. Not for
    // a held question (`taken`): a keystroke is not a hold, so that run
    // waits out the watchdog by rule.
    Verb {
        name: "publish-enter",
        when: &[],
        plain: "publish answering far=free unsure=false answerable=true",
    },
    // The same from the name box: the run raises the box's own `accepted`
    // and nothing else, so the write barrier is the claim — a build where
    // Enter reaches nothing waits out the watchdog.
    Verb {
        name: "set-upstream-enter",
        when: &[],
        plain: "there=true answerable=true",
    },
    // The toolbar with an upstream the far side has not got yet
    // (`tracked=false`): the press reopens the destination question, and
    // what is read is that it opened on the name the branch was just
    // pointed at, not the guess.
    Verb {
        name: "publish-upstream",
        when: &[(
            Arg::Is(""),
            "publish upstream=origin/brand-new tracked=false state=publish \
             remote=origin branch=brand-new",
        )],
        plain: "tracked=false state=publish",
    },
    // The `pull` row on the current branch, on its upstream, and on a
    // remote nothing here tracks, which must not carry it (a pull there
    // would move a branch the row does not name). `sentence=false`: both
    // offered rows run the same `git pull`, so both are the chip alone.
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
    // The same row once both sides moved: bringing them together is a
    // choice, so the row is out and its tooltip says where the choice is
    // made. The sidebar's row and the graph chip's get the answer from the
    // same place, but a run through one says nothing about the other.
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
