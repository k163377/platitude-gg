//! Everything that acts on a file or on one row of its diff — staging,
//! discarding, walking — and where the pane lands when the file under it
//! moves.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // The file list's arrows, and the half of the keyboard rule
    // the diff's own arrows are the other half of: nothing here
    // pressed the diff, so `focused=true` says the list *kept* the
    // keyboard when the diff opened over the graph. `lit=true` is
    // the row painted as the one being read — the rectangle's own
    // answer — and `moved=true` says the diff followed the light.
    // The picture is a weak witness: one file's diff frames like
    // another's.
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
    // The second click stopped at, in each list, where the two used to
    // part and now do not: the light means the file being read in both,
    // so both go out with the diff (デザイン規約 §diff のファイル一覧).
    // `lit=false` is the whole of it, and it has to be a pair — one of
    // these alone is a list with no diff up, which either answer frames
    // identically. `open=false` is what makes the dark list the list's
    // own answer.
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
    // Escape over an open diff, with the hand already inside it. The
    // page's handler is the last thing an Escape nobody else claimed
    // reaches, and `took=true` is it claiming the key — without it the
    // key would go on to whatever asks next and the run would read as a
    // pane that closed itself. `shown=false` is the pane gone and
    // `key=empty` the page holding no file any more, which is what tells
    // this from a pane hidden with its reading left standing behind it.
    //
    // `hand=true kept=true` is the other half, and the one no picture
    // could hold: the pane that had the keyboard is the pane that just
    // went, and `kept=` says the keyboard stayed inside the page. Out
    // at the window, this page is a descendant and nothing it handles
    // is on the key's way any more, so the *next* Escape would reach
    // nothing at all.
    //
    // `graph=true` says where inside the page it landed: the history the
    // reader is left looking at, so the arrows walk it. One item holds
    // active focus at a time, so this is the file list letting go as
    // well — and a run where the key closed the pane but left the arrows
    // with nobody photographs exactly the same graph.
    Verb {
        name: "diff-escape",
        when: &[],
        plain: "diff_escape took=true shown=false key=empty folded=false hand=true kept=true graph=true",
    },
    // The row of a repository of its own. There is no patch behind it and
    // never will be — git does not cross into another repository — so
    // `embedded=true` is the pane having an answer about the path at all,
    // and `rows=0` says it is not one made of a diff. Which of the two
    // sentences it is rides in `sha8=`, which the argument decides: the
    // commit a stage would point at, or empty where that repository has
    // none yet.
    Verb {
        name: "wip-embedded",
        when: &[],
        plain: "embedded=true rows=0",
    },
    // And the end it stops at, the same shape `diff-step-edge` reads:
    // the walk asks for ten files, runs out, and the rest answer false.
    // `moved=true` beside it is what tells this from a walk that was
    // refused from the first press.
    Verb {
        name: "changes-step-edge",
        when: &[],
        plain: "moved=true stopped=true lit=true focused=true",
    },
    // The same list's folder rows, where the arrow is the whole of the
    // picture: `>` shut, `v` open. `turn=` is the rotation the icon is
    // actually drawn at — the two file lists keep the model's flag in
    // different fields, and a cell that read only one of them left this
    // list's arrow lying open in both states while `shut=` went on
    // answering true. Read as a pair: an arrow on its own says nothing
    // about which way it turned, so `-unfold` strikes the same row a
    // second time and has to come back to 90.
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
    // The diff's own arrows, where the picture is the weakest witness
    // in the app: a diff scrolled two rows and a diff never scrolled
    // at all are the same photograph of the same file. Everything that
    // matters is in the line. `focused=true` is the hand's arrival
    // taking the keyboard — the pane does not take it by appearing, so a
    // false here means the walk was pressing on a pane the file list
    // still owns — and `moved=true` with `atEnd=false stopped=false` is a
    // walk that had somewhere to go and went there.
    Verb {
        name: "diff-step",
        when: &[],
        plain: "moved=true atEnd=false stopped=false focused=true",
    },
    // And the end it stops at. `stopped=true` is the refusal itself:
    // the walk asks for twenty rows, gets as far as the bottom, and
    // the rest answer false. Without `moved=` beside it a pane that
    // refused every step from the start — never on screen, never
    // focused — would read the same.
    Verb {
        name: "diff-step-edge",
        when: &[],
        plain: "moved=true atEnd=true stopped=true focused=true",
    },
    // Everything that acts on a row of a diff. The failure these
    // share is the one no camera catches: a pane the rows never
    // reached — because the read was still out, or because the file
    // was not dirty in the first place — is a pane with nothing in
    // it, and so is a file with nothing to show. The verb then names
    // a row that is not there, picks no lines, or holds a button
    // nobody drew, and none of it writes, so the run came back green
    // with an empty picture. `ready=` is the pane's own answer to
    // "were the rows here when I acted", and it is worth spelling
    // per verb: the wanted line
    // names the act, so a run whose hook never reached the diff at
    // all cannot borrow another verb's report to pass on.
    Verb {
        name: "diff-file",
        when: &[],
        plain: "diff_row act=diff-file ready=true",
    },
    // The three buckets a file can be previewed from. `ready=` is the
    // pane's settled form arriving — the toggle only asks, the read is
    // a git subprocess away, and a pane the answer never reached
    // photographs as a black pane under a DIFF header, which
    // `screenshot saved=true` is perfectly happy with (measured,
    // with the toggle as the completion, every run shot the header
    // alone). A run fired with no path never opens anything and says
    // `diff_arg named=false` instead, so it fails here too.
    // On the `pictures` preset's own files the line is the pictures
    // themselves: `pictures=` is how many sides decoded off the `file:`
    // URLs they were handed, and a URL that names nothing photographs
    // as the same caption over the same empty frame as one never
    // handed over. Two for a file with both sides, one where a side is
    // not there (staged and never committed, deleted from the index,
    // untracked).
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
    // The same pane, closed once its pictures were there: `was=image`
    // is the pictures having been decoded before the close, `url=empty`
    // the pane having let go of both — the picture after this is of a
    // page with no diff on it, which is also what a run that never
    // opened one photographs.
    Verb {
        name: "preview-close",
        when: &[],
        plain: "preview_close was=image kind= shown=false pictures=0 url=empty",
    },
    // Both sides have to be named on some row, and on a file that has
    // been typed over both of them are removals — the half that used
    // to be dropped, which left the legend explaining a distinction
    // no row was making. A count of zero on
    // either side photographs exactly like a file with nothing to
    // tell apart.
    Verb {
        name: "conflict-sides",
        when: &[],
        plain: "conflict_sides combined=true told=true both=true",
    },
    // Where the merge editor left the row it was handed. The shot waits
    // for that row to leave the conflicts bucket
    // (`AutoActDriver.treeBarrier`), so a tool that never started ends
    // at the watchdog — but a tool that took the row anywhere else
    // empties that bucket just the same, and a list one conflict
    // shorter frames the same either way. `landed=staged` is the half
    // that says the resolution reached git: the seeded editor exits 0
    // and `trustExitCode` has it added (verbs.md, which has been
    // naming this line as what judges the verb).
    Verb {
        name: "open-mergetool",
        when: &[],
        plain: "tree_settled from=conflicts landed=staged",
    },
    // The tick that asks whether the open file still reads the way it
    // did. What it is asking about did not move during the run, so
    // the read is answered with silence — the ask is the only edge
    // there is, and `asked=true` is the whole of what this side can
    // claim: the slot resolved, the pane owned the file it named, and
    // the session took the read. `loading=false` is the other half of
    // the wiring: a tick that put the pane back into a click's state
    // would blank it once a poll (`RepoSession::refresh_diff`).
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
    // The text picked out of the rows. All four drag the same stretch —
    // the head of the first line down past the first removed one — so
    // `diff_pick new=true removed=1` is the fixture answering that the
    // selection reached both sides. Without it the three below prove
    // nothing: a drag that never crossed a removed line copies no removed
    // line either, and every claim comes back green on an empty
    // selection.
    //
    // `washed=` is what the picture is of, and it is the one number a
    // camera can be asked for here — the wash on a removed line is the
    // failure this feature has, and it is a coloured rectangle a few
    // pixels tall inside a pane of them.
    //
    // `door=` stands in front of both because a selection nobody holds
    // the keyboard for is not one `Ctrl+C` can take away, and no run can
    // press a key to find out (verify-ui). It reads where the pane
    // watches presses from: on the pane itself that sheet heard nothing
    // — the list and the hand over it take every press there — and the
    // diff's keyboard arrived by wheel alone.
    Verb {
        name: "diff-select",
        when: &[],
        plain: "diff_pick door=true new=true removed=1",
    },
    // The plain copy, and the whole of what it claims: the old line
    // stayed out of it. Read back off the pad the copy goes through,
    // because the clipboard will not answer a headless run — and read
    // as "does the text hold the removed line", since a copy that took
    // the wrong four lines counts to four as well.
    Verb {
        name: "diff-copy",
        when: &[],
        plain: "diff_copy holdsRemoved=false",
    },
    // The card itself, for the words on it (the reader's whole way to the
    // old side, so the spelling is the deliverable — the picture is
    // overlay.png). `open=` is the menu's own answer: a card that
    // refused to open because it had nothing in it photographs exactly
    // like one nobody asked for. `rows=2` is beside it because a
    // selection that reached only one side opens a one-row card that
    // reads like a full one.
    Verb {
        name: "diff-menu",
        when: &[],
        plain: "diff_menu open=true rows=2",
    },
    // And the same claim from the other side: the row named for the
    // removed lines puts exactly them on the clipboard.
    Verb {
        name: "diff-copy-removed",
        when: &[],
        plain: "diff_copy_removed holdsRemoved=true",
    },
    // The blank right of a row's last character. Entered through the
    // hand's own two functions with **pixels** — which is what the four
    // above cannot reach, since they hand over byte offsets and would
    // pass whatever the pane made of a point.
    //
    //  - `quiet=true` — a drag that starts and ends out there selects
    //    nothing. This is the report: a walk of columns went on counting
    //    characters past the end of the line and picked a place in the
    //    middle of it (`e`+U+0301 four hundred times ends at 3,200px and
    //    was answering about byte 675 at 3,600).
    //  - `clamped=true` — and a drag that starts on the row and ends out
    //    there takes the line and exactly the line, held against the same
    //    row selected by number. Without it `quiet=` goes green on a hand
    //    that stopped selecting at all.
    //  - `backwards=true` — and the same drag made the other way, from
    //    the blank back onto the row. The blank is a place to start from
    //    as much as a place to stop at (規約 §diff の中身をコピーする),
    //    and a fix that refused the press out there would pass `quiet=`
    //    by taking the gesture away.
    //  - `across=true` — a drag from one row's head down into the blank
    //    beside a later one, held against those rows read one at a time.
    //    The rows between the two ends are what a single-row drag cannot
    //    ask about: the copy joins them and leaves the removed lines out.
    //  - `sent=true` — `quiet=` again after the code has been sent
    //    sideways, where an offset counted from the wrong place puts
    //    every press somewhere else (`DiffCodeScroll.offset`).
    //
    // All four are read off the pad the copy goes through
    // (`ClipboardHelper.lastCopied`): the claim is what a reader ends
    // up holding.
    //
    // `rows=` is the run's own honesty beside them: a fixture whose lines
    // all run past the pane leaves no blank to press in, and five trues
    // off none of them say nothing (`diff_sweep ground=`, the same rule).
    Verb {
        name: "diff-blank",
        when: &[],
        plain: "diff_blank quiet=true clamped=true backwards=true across=true sent=true",
    },
    // The diff read side by side (規約 §diff を 2 列で読む). The picture
    // is two columns; what it cannot say is whether the choice reached
    // the store (`saved=`, read back off it after the page reported its
    // layout) — the tally after these two is the fixture's own shape.
    Verb {
        name: "diff-split",
        when: &[],
        plain: "diff_split split=true saved=true",
    },
    // A line named on each side puts that side's mark out and not the
    // other's: `crossed=false` is the half a picture of one mark cannot
    // answer, since a mark out on both sides frames like one out on one.
    Verb {
        name: "split-tools",
        when: &[],
        plain: "split_tools left=true right=true crossed=false",
    },
    // A drag down each column copies that column's file: two lines each,
    // the old column's copy already holding the removed line (nothing
    // left for the menu's second word), the new column's offering it
    // across — and the two agreeing on which line that is.
    Verb {
        name: "split-copy",
        when: &[],
        plain: "split_copy left=2 leftRemoved=0 right=2 rightRemoved=1 agree=true",
    },
    // The same text taken from the ground under the last row — every
    // place in the code column that nobody else takes is a start
    // (規約 §diff の中身をコピーする). Three claims, and each answers
    // something the other two cannot:
    //
    //  - `reach=9/9` — from every corner of that ground. A verb that
    //    pressed one point in it goes green over a hand that only
    //    answers there, which is the fault the right pane's own values
    //    shipped with (`details-sweep`).
    //  - `ground=true` — the fixture is half the claim. A file that
    //    fills the frame leaves no ground at all, and `reach=0/9` off
    //    one says nothing about the hand (`--preset dirty` の `b.txt`
    //    = 3 rows).
    //  - `ours=false` — and a press on a hunk's heading is still the
    //    hunk's. That face carries `Stage hunk` / `Discard hunk`, and
    //    taking it is the one way this widening could cost anybody
    //    anything.
    Verb {
        name: "diff-sweep",
        when: &[],
        plain: "diff_sweep reach=9/9 ground=true ours=false",
    },
    // The band above those rows, whose path is the one value in it. The
    // claim is `all=`, for the reason every `SweepPad` run makes it:
    // how much air a band has depends on the words in it and on the
    // machine that drew them. `caret=` is the half a selection does not
    // say — `Ctrl+C` goes to whatever holds the keyboard — and `hand=`
    // is the half a sweep cannot say for itself, since a run enters the
    // pad's functions and would go green with the `MouseArea` taken
    // back out.
    Verb {
        name: "diff-band-sweep",
        when: &[],
        plain: "diff_band_sweep all=true caret=true hand=true cut=true",
    },
    // The other end of that same hand: the bar down the side of the
    // list is drawn over the rows, so a hand laid over them covers it,
    // and one that ran to the frame took every press on the trough —
    // the bar could not be grabbed at all.
    // The picture cannot say any of it (a bar that answers nothing is
    // drawn exactly like one that does), so all three are read here:
    //
    //  - `clear=true` — where the hand ends against where the
    //    bar begins, read off the two items themselves.
    //    Taking the strip back out of `DiffTextSelect` turns
    //    this false.
    //  - `reach=true` — and the hand still answers at its own
    //    last pixel, so the strip came out of the bar's side
    //    and the code kept its width.
    //  - `out=true` — the fixture is half the claim, as it is for
    //    `diff-sweep`: a diff that fits its frame has no bar to be kept
    //    clear of (`--preset manyhunks` の `notes.txt` = 320 rows).
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
    // This one asks for more: `ready=` says the diff arrived, and a
    // diff that arrived is where this verb's failures start. The
    // place is only kept if the rebuilt list came back to it, so
    // `at=` is read against `want=` — and `room=` is there because
    // the fixture is half of it: a file whose diff fits the pane has
    // no place to lose, scrolls nowhere, and photographs exactly
    // like one that lost nothing (`--preset manyhunks`).
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
    // The colours that land behind the rows, and the two things they
    // leave whole. Neither is a picture: a diff whose colours never
    // came frames as a language the set has no rules for, and a view
    // thrown back to the top frames as one nobody had scrolled. `at=`
    // is `scrollTo`'s own number read back after the swap — 400 or
    // the reader lost their place. `held=` is how far the diff reaches,
    // read on the last look before the colours and again after them:
    // the rows are markup either way, so measuring one of them in the
    // turn the tags land must come to the same number as measuring it
    // before they did (`DiffReach`).
    Verb {
        name: "colour-place",
        when: &[],
        plain: "colour_place coloured=true at=400 held=true",
    },
    // The picture cannot tell a diff sent to its end from one that had
    // nowhere to go: both frame as a pane of text with its left edge
    // showing. So the two things that only happen when there is
    // somewhere to go are what it is judged on — the bar came out, and
    // the hand is still carrying the rows. A fixture whose lines fit
    // the pane fails here (`--preset widelines` is the one
    // that does not fit).
    Verb {
        name: "code-send",
        when: &[],
        plain: "code_send bar=true hand=true",
    },
    // Where that width comes from, in the one shape the picture is
    // blind to: every one of these frames as a pane of text standing
    // somewhere along its lines.
    //
    // `code-grow` is the pick being corrected — a line no record named,
    // drawn further than the ones that were, reaching the width when
    // its row is finally laid out (`grew=`, `past=`), and still the
    // same width under the same row after a trip back to the head and
    // down again, which is where a delegate reused for another row
    // would have filed one under the wrong one (`kept=`).
    //
    // `ends=` is the one the other three cannot make between them, and
    // the only one with a picture behind it: sent to its far end with
    // that row on screen, what stands at the frame's right edge is the
    // end of the line. Everything else here reads a width against
    // another number worked out from it, so a reach running past the
    // end of every line agrees with itself all the way down — and
    // shows only there, as room left over past the last character.
    Verb {
        name: "code-grow",
        when: &[],
        plain: "code_grow grew=true past=true kept=true ends=true",
    },
    // `code-shrink` is the same file read again with its widest line
    // written away: the place along the row is kept (`kept=`) while the
    // width it is clamped against comes down (`shrank=`), and both
    // halves of the reach come down with it, where a latch would keep
    // the largest number it ever saw (`narrower=`).
    Verb {
        name: "code-shrink",
        when: &[],
        plain: "code_shrink kept=true shrank=true narrower=true",
    },
    // `code-swap` is another file: the place goes (`dropped=`) and so
    // does the width (`narrower=`). `room=` is what makes the first of
    // those mean anything — a file with nowhere sideways to go would
    // stand at its left edge however the reach behaved.
    Verb {
        name: "code-swap",
        when: &[],
        plain: "code_swap dropped=true room=true narrower=true",
    },
    // A line staged from the diff, the file then moved from the list,
    // and the diff following both. The picture is the last frame of
    // three and cannot show the two before it, so all three answers
    // are read: the rows shrank, the rows came back, and the pane
    // followed the file over to the staged side when the unstaged one
    // ran out (`--preset manyhunks` has the one file, so that is where
    // it has to land).
    Verb {
        name: "line-back",
        when: &[],
        plain: "line_back back=true shrank=true followed=staged:notes.txt",
    },
    // Where the pane lands when the file under it is moved whole. The
    // picture shows a diff either way and cannot say which file it is
    // of, so the landing is read: `shown=true` is the half that fails
    // when the pane closes on the reader.
    Verb {
        name: "diff-follow",
        when: &[],
        plain: "diff_follow shown=true",
    },
    // Three lines staged one after another, each pressed the moment
    // the pane will take one. The count is the whole judgement: a pane
    // that stops taking presses after the first stops here too, and
    // the picture of it is a diff either way.
    Verb {
        name: "line-run",
        when: &[],
        plain: "line_run staged=3 want=3",
    },
    // A bucket emptied from its own heading keeps that heading, both
    // ways round — which is the whole claim, so both headings are
    // read back whichever direction was pressed. The picture cannot
    // be trusted with it: a list with one heading missing frames as a
    // list, and the missing one is only missing next to the other
    // direction's picture.
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
    // The conflicted heading is the one that answers the other way:
    // it stands only while git has something unmerged, so marking the
    // whole bucket resolved has to take it off the screen. `staged=`
    // rides along to say where the files went — `git add --all` would
    // have emptied the unstaged bucket with them, and the picture of
    // that is the same picture.
    Verb {
        name: "resolve-all",
        when: &[],
        plain: "wip_heads from=conflicts unstaged=true staged=true conflicts=false",
    },
];
