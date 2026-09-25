//! Everything that acts on a file or on one row of its diff — staging,
//! discarding, walking — and where the pane lands when the file under it
//! moves.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // The file list's arrows. Nothing here pressed the diff, so
    // `focused=true` says the list kept the keyboard when the diff
    // opened; `lit=true` is the row painted as the one being read, and
    // `moved=true` the diff following it.
    Verb {
        name: "changes-step",
        when: &[],
        plain: "moved=true stopped=false lit=true focused=true",
    },
    Verb {
        name: "wip-step",
        when: &[],
        plain: "moved=true stopped=false lit=true focused=true",
    },
    // The second click on the row being read closes the diff, and the
    // light goes out with it in both lists (デザイン規約 §diff のファイル一覧).
    // Read as a pair: one alone frames like a list with no diff up.
    // `open=false` makes the dark list the list's own answer.
    Verb {
        name: "wip-shut",
        when: &[],
        plain: "pane=wip open=false lit=false",
    },
    Verb {
        name: "changes-shut",
        when: &[],
        plain: "pane=changes open=false lit=false",
    },
    // Escape over an open diff with the hand inside it. `took=true` is
    // the page's handler claiming the key (unclaimed, it goes on to
    // whatever asks next); `key=empty` tells a closed pane from one
    // hidden with its reading left behind it.
    //
    // `hand=true kept=true`: the pane that had the keyboard just went,
    // and the keyboard has to stay inside the page — out at the window
    // the next Escape reaches nothing. `graph=true` is where it landed,
    // so the arrows walk the history and the file list has let go.
    Verb {
        name: "diff-escape",
        when: &[],
        plain: "diff_escape took=true shown=false key=empty folded=false hand=true kept=true graph=true",
    },
    // The row of a nested repository. git does not cross into it, so
    // there is no patch: `embedded=true` is the pane answering about the
    // path at all, `rows=0` that the answer is not a diff. Which sentence
    // it is rides in `sha8=` (the commit a stage would point at, or empty).
    Verb {
        name: "wip-embedded",
        when: &[],
        plain: "embedded=true rows=0",
    },
    // The end of the walk, as `diff-step-edge` reads it.
    Verb {
        name: "changes-step-edge",
        when: &[],
        plain: "moved=true stopped=true lit=true focused=true",
    },
    // The list's folder rows. `turn=` is the rotation the arrow is drawn
    // at: the two file lists keep the fold flag in different fields, and
    // a cell reading only one of them draws the arrow open while `shut=`
    // answers true. Read as a pair — `-unfold` strikes the same row again.
    Verb {
        name: "changes-fold",
        when: &[],
        plain: "shut=true turn=0",
    },
    Verb {
        name: "changes-unfold",
        when: &[],
        plain: "shut=false turn=90",
    },
    // The diff's own arrows. `focused=true` is the hand's arrival taking
    // the keyboard — the pane does not take it by appearing, so false
    // means the presses went to the file list.
    Verb {
        name: "diff-step",
        when: &[],
        plain: "moved=true atEnd=false stopped=false focused=true",
    },
    // The end of the walk: `stopped=true` is the refusal at the bottom,
    // and `moved=true` tells it from a pane that refused from the start.
    Verb {
        name: "diff-step-edge",
        when: &[],
        plain: "moved=true atEnd=true stopped=true focused=true",
    },
    // Everything that acts on a row of a diff. A pane the rows never
    // reached (read still out, file not dirty) acts on nothing and writes
    // nothing, so `ready=` is the pane's answer to "were the rows here
    // when I acted". `act=` names the verb, so a run whose hook never
    // reached the diff cannot pass on another verb's report.
    Verb {
        name: "diff-file",
        when: &[],
        plain: "diff_row act=diff-file ready=true",
    },
    // The three buckets a file can be previewed from. `ready=` is the
    // pane's settled form: the toggle only asks and the read is a git
    // subprocess away, so without it the shot is a black pane under the
    // header. A run given no path says `diff_arg named=false` and fails.
    // On the `pictures` preset, `pictures=` is how many sides decoded off
    // their `file:` URLs — two with both sides, one where a side is absent
    // (staged and never committed, deleted from the index, untracked).
    Verb {
        name: "preview",
        when: &[(
            Arg::Is("art/photo.png"),
            "preview_pane kind=image binary=true pictures=1",
        )],
        plain: "diff_row act=preview ready=true",
    },
    Verb {
        name: "preview-unstaged",
        when: &[
            (
                Arg::Is("art/logo.png"),
                "preview_pane kind=image binary=true pictures=2",
            ),
            (
                Arg::Is("art/mark.svg"),
                "preview_pane kind=image binary=false pictures=2",
            ),
        ],
        plain: "diff_row act=preview-unstaged ready=true",
    },
    Verb {
        name: "preview-staged",
        when: &[(
            Arg::OneOf(&["art/icon.png", "art/old.png"]),
            "preview_pane kind=image binary=true pictures=1",
        )],
        plain: "diff_row act=preview-staged ready=true",
    },
    // The same pane closed once its pictures were up: `was=image` says
    // they decoded before the close, `url=empty` that the pane let go of
    // both.
    Verb {
        name: "preview-close",
        when: &[],
        plain: "preview_close was=image kind= shown=false pictures=0 url=empty",
    },
    // Both sides have to be named on some row — on a file typed over,
    // both are removals, and dropping that half leaves the legend
    // explaining a distinction no row makes.
    Verb {
        name: "conflict-sides",
        when: &[],
        plain: "conflict_sides combined=true told=true both=true",
    },
    // Where the merge editor left the row. The shot waits for it to leave
    // the conflicts bucket (`AutoActDriver.treeBarrier`), which a row
    // taken anywhere else does too; `landed=staged` says the resolution
    // reached git (the seeded editor exits 0 and `trustExitCode` adds it).
    Verb {
        name: "open-mergetool",
        when: &[],
        plain: "tree_settled from=conflicts landed=staged",
    },
    // The tick that re-asks the open file. Nothing moved during the run,
    // so the read answers with silence and `asked=true` (slot resolved,
    // pane owned the file, session took the read) is all this side can
    // claim. `loading=false`: a tick that put the pane into a click's
    // state would blank it once a poll (`RepoSession::refresh_diff`).
    Verb {
        name: "diff-tick",
        when: &[],
        plain: "diff_tick asked=true loading=false",
    },
    Verb {
        name: "line-tools",
        when: &[],
        plain: "diff_row act=line-tools ready=true",
    },
    Verb {
        name: "hunk-tools",
        when: &[],
        plain: "diff_row act=hunk-tools ready=true",
    },
    // The text picked out of the rows. All four drag the same stretch,
    // from the head of the first line past the first removed one, so
    // `new=true removed=1` is the fixture saying the selection reached
    // both sides — without it the three below go green on an empty one.
    // `washed=` rides unjudged: it is what the picture is of.
    //
    // `door=`: the sheet the pane watches presses from must stand in
    // front of the list and the hand, which take every press on the pane
    // itself — behind them the diff gets the keyboard by wheel alone and
    // `Ctrl+C` has nothing to take. No run can press a key (verify-ui).
    Verb {
        name: "diff-select",
        when: &[],
        plain: "diff_pick door=true new=true removed=1",
    },
    // The plain copy leaves the removed line out. Read off the pad the
    // copy goes through (the clipboard does not answer a headless run),
    // as "does it hold the removed line" — a wrong four lines count to
    // four as well.
    Verb {
        name: "diff-copy",
        when: &[],
        plain: "diff_copy holdsRemoved=false",
    },
    // The card, for its words (the picture is overlay.png). `open=`: a
    // card that refused to open for having nothing in it frames like one
    // nobody asked for. `rows=2`: a selection that reached one side opens
    // a one-row card that reads like a full one.
    Verb {
        name: "diff-menu",
        when: &[],
        plain: "diff_menu open=true rows=2",
    },
    // The row named for the removed lines copies exactly them.
    Verb {
        name: "diff-copy-removed",
        when: &[],
        plain: "diff_copy_removed holdsRemoved=true",
    },
    // The blank right of a row's last character, entered through the
    // hand's own functions with pixels — the four above hand over byte
    // offsets and would pass whatever the pane made of a point.
    //
    //  - `quiet=true` — a drag that starts and ends out there selects
    //    nothing (a column walk counting past the line's end picks a
    //    place mid-line).
    //  - `clamped=true` — a drag from the row out there takes exactly the
    //    line, held against the row selected by number; without it
    //    `quiet=` passes a hand that stopped selecting.
    //  - `backwards=true` — the same drag from the blank onto the row
    //    (規約 §diff の中身をコピーする); refusing the press out there
    //    would pass `quiet=` by taking the gesture away.
    //  - `across=true` — from one row's head into the blank beside a
    //    later one, held against those rows read one at a time: the rows
    //    between are what a single-row drag cannot ask about.
    //  - `sent=true` — `quiet=` again with the code sent sideways, where
    //    an offset counted from the wrong place moves every press
    //    (`DiffCodeScroll.offset`).
    //
    // All are read off the pad the copy goes through
    // (`ClipboardHelper.lastCopied`). `rows=` rides beside them: lines
    // that all run past the pane leave no blank to press in.
    Verb {
        name: "diff-blank",
        when: &[],
        plain: "diff_blank quiet=true clamped=true backwards=true across=true sent=true",
    },
    // The diff read side by side (規約 §diff を 2 列で読む). `saved=` is
    // the choice read back off the store after the page reported its
    // layout; the tally after these two is the fixture's own shape.
    Verb {
        name: "diff-split",
        when: &[],
        plain: "diff_split split=true saved=true",
    },
    // A line named on each side puts out that side's mark only:
    // `crossed=false` catches a mark out on both.
    Verb {
        name: "split-tools",
        when: &[],
        plain: "split_tools left=true right=true crossed=false",
    },
    // A drag down each column copies that column's file: the old column's
    // copy already holds the removed line, the new column's offers it
    // across, and `agree=` says both mean the same line.
    Verb {
        name: "split-copy",
        when: &[],
        plain: "split_copy left=2 leftRemoved=0 right=2 rightRemoved=1 agree=true",
    },
    // The same text taken from the ground under the last row
    // (規約 §diff の中身をコピーする).
    //
    //  - `reach=9/9` — from every corner of that ground: pressing one
    //    point passes a hand that answers only there (as `details-sweep`).
    //  - `ground=true` — a file that fills the frame leaves no ground,
    //    and `reach=0/9` off it says nothing (`--preset dirty` の `b.txt`
    //    = 3 rows).
    //  - `ours=false` — a press on a hunk's heading stays the hunk's: it
    //    carries `Stage hunk` / `Discard hunk`.
    Verb {
        name: "diff-sweep",
        when: &[],
        plain: "diff_sweep reach=9/9 ground=true ours=false",
    },
    // The band above those rows, whose path is its one value. `all=` /
    // `hand=` as in verbs.md §面の掃き. `caret=`: `Ctrl+C` goes to whatever
    // holds the keyboard.
    Verb {
        name: "diff-band-sweep",
        when: &[],
        plain: "diff_band_sweep all=true caret=true hand=true cut=true",
    },
    // The bar down the list's side is drawn over the rows, so a hand
    // laid over them to the frame takes every press on the trough.
    //
    //  - `clear=true` — the hand ends where the bar begins, read off the
    //    two items; taking the strip out of `DiffTextSelect` turns it
    //    false.
    //  - `reach=true` — the hand still answers at its last pixel, so the
    //    strip came out of the bar's side and the code kept its width.
    //  - `out=true` — a diff that fits its frame has no bar, as for
    //    `diff-sweep` (`--preset manyhunks` の `notes.txt` = 320 rows).
    Verb {
        name: "diff-bar",
        when: &[],
        plain: "diff_bar out=true clear=true reach=true",
    },
    Verb {
        name: "stage-hunk",
        when: &[],
        plain: "diff_row act=stage-hunk ready=true",
    },
    Verb {
        name: "stage-line",
        when: &[],
        plain: "diff_row act=stage-line ready=true",
    },
    // `at=` against `want=`: the place is kept only if the rebuilt list
    // came back to it. `room=` rides along because a diff that fits the
    // pane has no place to lose (`--preset manyhunks`).
    Verb {
        name: "keep-place",
        when: &[],
        plain: "diff_place at=400 want=400",
    },
    Verb {
        name: "discard-hunk",
        when: &[],
        plain: "diff_row act=discard-hunk ready=true",
    },
    Verb {
        name: "discard-hunk-go",
        when: &[],
        plain: "diff_row act=discard-hunk-go ready=true",
    },
    // The colours landing behind the rows, and the two things they leave
    // whole. `at=` is `scrollTo`'s number read back after the swap — 400
    // or the reader lost their place. `held=` is the diff's reach before
    // and after the colours: the rows are markup either way, so the two
    // must agree (`DiffReach`).
    Verb {
        name: "colour-place",
        when: &[],
        plain: "colour_place coloured=true at=400 held=true",
    },
    // Judged on what only happens when there is somewhere to go: the bar
    // came out and the hand still carries the rows. A fixture whose lines
    // fit fails here (`--preset widelines` is the one that does not).
    Verb {
        name: "code-send",
        when: &[],
        plain: "code_send bar=true hand=true",
    },
    // Where that width comes from. `code-grow` is the pick corrected: a
    // line no record named, drawn further than those that were, reaches
    // the width once its row is laid out (`grew=`, `past=`), and keeps it
    // under the same row after a trip to the head and back — a reused
    // delegate would file it under the wrong row (`kept=`).
    //
    // `ends=`: sent to the far end with that row on screen, the frame's
    // right edge is the line's end. The other fields read a width against
    // a number worked out from it, so a reach running past every line
    // agrees with itself and shows only here.
    Verb {
        name: "code-grow",
        when: &[],
        plain: "code_grow grew=true past=true kept=true ends=true",
    },
    // The same file with its widest line written away: the place is kept
    // (`kept=`), the clamp width comes down (`shrank=`), and so does the
    // reach, where a latch would keep its largest (`narrower=`).
    Verb {
        name: "code-shrink",
        when: &[],
        plain: "code_shrink kept=true shrank=true narrower=true",
    },
    // Another file: the place goes (`dropped=`) and so does the width
    // (`narrower=`). `room=`: a file with nowhere sideways to go stands at
    // its left edge whatever the reach did.
    Verb {
        name: "code-swap",
        when: &[],
        plain: "code_swap dropped=true room=true narrower=true",
    },
    // A line staged from the diff, then the file moved from the list. The
    // picture is the last of three frames, so all three are read: the
    // rows shrank, came back, and the pane followed the file to the
    // staged side (`--preset manyhunks` has the one file).
    Verb {
        name: "line-back",
        when: &[],
        plain: "line_back back=true shrank=true followed=staged:notes.txt",
    },
    // Where the pane lands when the file under it is moved whole:
    // `shown=true` fails when the pane closes on the reader.
    Verb {
        name: "diff-follow",
        when: &[],
        plain: "diff_follow shown=true",
    },
    // Three lines staged one after another, each the moment the pane will
    // take it: the count catches a pane that stops after the first.
    Verb {
        name: "line-run",
        when: &[],
        plain: "line_run staged=3 want=3",
    },
    // A bucket emptied from its own heading keeps that heading, both ways
    // round, so both headings are read back whichever way was pressed.
    Verb {
        name: "stage-all",
        when: &[],
        plain: "wip_heads from=unstaged unstaged=true staged=true",
    },
    Verb {
        name: "unstage-all",
        when: &[],
        plain: "wip_heads from=staged unstaged=true staged=true",
    },
    // The conflicted heading stands only while git has something
    // unmerged, so resolving the whole bucket takes it away. `staged=`
    // says where the files went — `git add --all` would have emptied the
    // unstaged bucket with them.
    Verb {
        name: "resolve-all",
        when: &[],
        plain: "wip_heads from=conflicts unstaged=true staged=true conflicts=false",
    },
];
