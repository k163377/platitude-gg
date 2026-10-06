//! Who a commit is by, and the boxes that say so — the identity dialog
//! and its badge, the signature, the caret in a message box, and the
//! settings card the avatar list comes down inside.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // The caret is the whole of these two: a hook that never reached the
    // box leaves the resting colour, which is a real state.
    Verb {
        name: "edit-message-focus",
        when: &[],
        plain: "message_focus pane=details focused=true",
    },
    Verb {
        name: "wip-message-focus",
        when: &[],
        plain: "message_focus pane=wip focused=true",
    },
    // A press elsewhere takes that caret back out, and the save row it put
    // up with it. `stood=true` is the caret having been in at all: a hook
    // that never reached the box would read the row down as well.
    Verb {
        name: "edit-message-away",
        when: &[],
        plain: "message_away stood=true row=false caret=false page=true",
    },
    // A dialog that stayed open because the write did not take looks
    // like one nobody answered; the `*Saved=` flags say which half landed.
    Verb {
        name: "identity",
        when: &[],
        plain: "identity state=missing dialog=true nameSaved=false emailSaved=false unsaved=false",
    },
    Verb {
        name: "identity-half",
        when: &[],
        plain: "state=ready dialog=true nameSaved=true emailSaved=false unsaved=true",
    },
    // Forced tooltips: a hook that never reached its target photographs
    // the resting state. `tip=` is the attached ToolTip's own visible.
    Verb {
        name: "identity-tip",
        when: &[],
        plain: "identity_tip unsaved=true badge=true tip=true",
    },
    Verb {
        name: "signature-tip",
        when: &[],
        plain: "signature_tip code=E tip=true",
    },
    // The merge editor list's loading and settled forms, each reached
    // causally, not by a millisecond window. `typing=`: the field's own
    // press brings the list down with the caret left in the box.
    Verb {
        name: "settings-tools-loading",
        when: &[],
        plain: "settled=false loading=true open=true typing=true",
    },
    Verb {
        name: "settings-tools",
        when: &[],
        plain: "settled=true loading=false open=true typing=true",
    },
    // Enter on each side of that list, in one run (デザイン規約
    // §選ぶ欄と打つ欄). `stood=`: with the list down, Enter is the list's
    // and the screen stays; `left=`: the next press, list gone, leaves.
    Verb {
        name: "settings-tools-enter",
        when: &[],
        plain: "merge_editor_enter stood=true left=true",
    },
    // Which git the app runs, and what that binary answered. The sentence
    // under the box takes a subprocess to arrive, so a run shot as the
    // path went in frames "Asking for the version…".
    //
    // No argument: box and store empty (a fresh settings directory), and
    // the git on `PATH` answered. Which version is never claimed — the
    // container runs the minimum, a desk the newest, and `--old-git` hands
    // a run an old one (the `in-use` run included, for the offered shape
    // over a git below the minimum); every row claims its run of flags.
    //
    // `in-use` picks the git already running: that box offers nothing.
    // `other` picks the second git (`--other-git`) and gets the button.
    // Both go in as the chooser answers (`autoPickGitPath`). `offers=` is
    // the rule the press is guarded by; `held=` the frozen copy the button
    // was drawn from — `Restart to apply` over a `holdMs` of zero would
    // fall through under a hand.
    //
    // A path with nothing at its end must be refused as *missing*, not as
    // "it ran and said something else"; `offers=false` keeps the loud
    // button away.
    //
    // Every row says `named=true native=false`: the box has a placeholder,
    // and no backslash in its value or that placeholder, on Windows too —
    // the placeholder is the OS's path, and `stored-missing` seeds the
    // file with the OS's separators. That the file keeps them through the
    // way out's write is read off the file after the run
    // (`super::super::seed::stored_path_kept`).
    Verb {
        name: "settings-git-path",
        when: &[
            (
                Arg::Is(""),
                "git_path shown= stored= named=true native=false answered=true offers=false held=false restart=false",
            ),
            (
                Arg::Is("in-use"),
                "named=true native=false answered=true offers=false held=false restart=false",
            ),
            (
                Arg::Is("other"),
                "named=true native=false answered=true offers=true held=true restart=false",
            ),
        ],
        plain: "named=true native=false answered=false offers=false held=false restart=false state=missing",
    },
    // The process chapter's box, typed and read back off the store. With
    // no argument the box is emptied: the default count is the machine's
    // own (a third of its threads), so the line says `default=true`.
    Verb {
        name: "settings-processes",
        // Three, which no machine defaults to (floor four, ceiling eight),
        // so `default=false` holds on every machine.
        when: &[(Arg::Is("3"), "git_processes concurrency=3 default=false")],
        plain: "default=true",
    },
    // The refresh chapter, worked and read back off the store, with what
    // the chooser shows and which boxes stand beside it (a box that is not
    // there is an absence a picture leaves to the eye). A fresh store reads
    // at the default bounds and reads the other worktrees automatically; a
    // ceiling typed under its floor comes back as the floor.
    Verb {
        name: "settings-refresh",
        when: &[
            (
                Arg::Is("2:30:auto:3:90"),
                "settings_refresh refresh=2-30 worktrees=auto worktrees_bounds=3-90 worktrees_secs=30 shown=Automatic boxes=bounds",
            ),
            (
                Arg::Is("::fixed:20"),
                "settings_refresh refresh=5-15 worktrees=fixed worktrees_bounds=5-45 worktrees_secs=20 shown=Fixed boxes=every",
            ),
            (
                Arg::Is("::off"),
                "settings_refresh refresh=5-15 worktrees=off worktrees_bounds=5-45 worktrees_secs=30 shown=Off boxes=none",
            ),
            (
                Arg::Is("20:10"),
                "settings_refresh refresh=20-20 worktrees=auto",
            ),
        ],
        plain: "settings_refresh refresh=5-15 worktrees=auto worktrees_bounds=5-45 worktrees_secs=30 shown=Automatic boxes=bounds",
    },
    // The way out over the chapter's bound boxes left unfinished, both
    // pairs set off their defaults first (7-20, 9-60): what each box held
    // is what the store keeps — an emptied box its default, a typed one
    // its number — whichever pair the close writes first.
    Verb {
        name: "settings-refresh-leave",
        when: &[(
            Arg::Is("3::7:"),
            "settings_refresh_leave refresh=3-15 worktrees=auto worktrees_bounds=7-45 worktrees_secs=30 open=false",
        )],
        plain: "settings_refresh_leave refresh=5-15 worktrees=auto worktrees_bounds=5-45 worktrees_secs=30 open=false",
    },
    // The way out taken over a git waiting to be applied: the screen
    // still stands with its mark turned, and what holds it must be the
    // offer — an unsaved identity holds the screen the same way.
    Verb {
        name: "settings-git-leave",
        when: &[],
        plain: "settings_git_leave offers=true armed=true unsaved=0 open=true",
    },
    // The rail between the two categories, which no other verb travels
    // (the rest name a category before the screen is up); both halves of
    // the move are in the line.
    Verb {
        name: "settings-switch",
        when: &[],
        plain: "settings_switch was_app=true app=false git=true",
    },
    // The screen's own way out, which `✕` and Escape both call: it was up,
    // and it went. `unsaved=0`: the way out stops for an unsaved identity,
    // and boxes drift from git's late, repeated answers on their own — a
    // screen that called that an edit would warn about work nobody did.
    // The argument names the category (the git one holds both such
    // chapters). `save=false`: a Save lit over untouched boxes offers to
    // hand git its own answer.
    Verb {
        name: "settings-escape",
        when: &[],
        plain: "settings_escape unsaved=0 global=false repo=false save=false was_open=true now_open=false",
    },
    // The screen's words taken from the air around them: every place in
    // the air must reach a value, one press per place.
    //
    // `all=` / `reach=` / `hand=` as in verbs.md §面の掃き. `shown=`: the
    // category off screen has its fields hidden and `SweepPad` steps over
    // them, so the wrong one sweeps an empty column — hence keyed on the
    // argument, which is the category.
    //
    // `release=` is the hand that gives the keyboard back when a press
    // lands where nothing takes it. The run (no pointer) can only say it
    // stands; `tests/qml/tst_fieldrelease.qml` asks the rest with a real
    // one.
    //
    // One row per argument, each carrying the whole line: a row wins
    // outright over `plain`, so a row holding only the half that differs
    // would stop asking for `all=`.
    Verb {
        name: "settings-sweep",
        when: &[
            (
                Arg::OneOf(&["", "app"]),
                "settings_sweep all=true caret=true hand=true cat=app shown=true release=true version=true",
            ),
            // The git category stops before `version=`: the chip belongs to
            // the app one, and here it would claim how fast `git --version`
            // came back.
            (
                Arg::Is("git"),
                "settings_sweep all=true caret=true hand=true cat=git shown=true release=true",
            ),
        ],
        plain: "settings_sweep all=true caret=true hand=true",
    },
    // The same way out over an unsaved identity: the screen stays, sends
    // the reader to the chapter and arms its `✕` — it has no foot to ask
    // in. `save=true`: the typing that stops the way out is the one that
    // lights the button the reader is sent to.
    Verb {
        name: "settings-leave",
        when: &[],
        plain: "settings_leave unsaved=1 save=true asked=true open=true",
    },
    // The git category's two groups, the tallest this screen gets. The
    // pair (with `settings-repo-pick`) splits the claim, since a row is
    // one substring. With no argument this one takes the reach
    // (rules-refs/app-ui.md「`contentHeight` は `implicitHeight` を読む」).
    // With an argument it switches to another repository, and the boxes
    // must follow — else the screen offers to write the wrong thing into
    // the one now named above it.
    Verb {
        name: "settings-repo",
        when: &[(Arg::Is(""), "settings_fit reach=true")],
        plain: "repo_config rows=2 state=ready matches=true save=false",
    },
    // This one takes the read and the list. `state=ready`: empty boxes
    // cannot tell "sets nothing of its own" from "the read never landed".
    // `rows=2`: both open repositories reached the chooser. `save=false`
    // is its own claim, not a spelling of `matches=` — this Save asks for
    // no filled boxes (emptying one takes an override out), so only the
    // touch keeps it dark.
    Verb {
        name: "settings-repo-pick",
        when: &[],
        plain: "repo_config rows=2 state=ready matches=true save=false open=true",
    },
    // The same group's line-ending chapter, picked. `were=` is what the
    // repository held before — `false`, as every demo repository is built
    // (`demo::repo`). `scope=local` is the only level: the screen writes
    // `core.autocrlf` into the picked repository alone (規約 §設定の画面).
    Verb {
        name: "settings-eol",
        when: &[
            (
                Arg::Is("input"),
                "line_endings scope=local state=ready were=false held=input",
            ),
            // The row that writes nothing falls back to the machine's value,
            // so the line stops before `effective=`; the trailing space keeps
            // `held=` from matching a longer held value.
            (
                Arg::Is("inherited"),
                "line_endings scope=local state=ready were=false held= ",
            ),
        ],
        plain: "line_endings scope=local state=ready were=false",
    },
    // The card's avatar list, the one of its four verbs that wants a line:
    // the other three wait on the state itself. The list is a popup in
    // overlay.png, judged as `commit-menu` is. Three popups: the modal
    // card's dimmer, the card, and the list inside it.
    Verb {
        name: "avatar-combo",
        when: &[],
        plain: "overlay saved=true popups=3",
    },
    // The picture filed against the details' author: a refused one still
    // completes the run, with the identicon in the picture.
    Verb {
        name: "avatar-assign",
        when: &[],
        plain: " details=true ",
    },
    // Enter in that box opens the file picker (デザイン規約 §アバターを与える),
    // the platform's own window and in neither PNG (as `open-picker`).
    // `who=` is the guard the button beside the box wears: a picker opened
    // over nobody files its answer against an empty address.
    Verb {
        name: "avatar-enter",
        when: &[],
        plain: "avatar_enter who=true picker=true",
    },
    // The badge's sentence names its address, handed in late (`%1`):
    // `named=` reads the tip against the author it was opened over.
    Verb {
        name: "avatar-tip",
        when: &[],
        plain: "avatar_tip tip=true named=true",
    },
];
