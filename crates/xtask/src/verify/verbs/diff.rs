//! Everything that acts on a file or on one row of its diff — staging,
//! discarding, walking — and where the pane lands when the file under it
//! moves.

use super::Verb;

pub(super) const TABLE: &[Verb] = &[
    // The file list's arrows, and the half of the keyboard rule the
    // diff's own arrows are the other half of: nothing here pressed the
    // diff, so `focused=true` says the list *kept* the keyboard when the
    // diff opened over the graph. `lit=true` is the row painted as the
    // one being read — the rectangle's own answer, not the condition
    // behind it — and `moved=true` says the diff followed the light
    // rather than only the light moving. The picture is a weak witness:
    // one file's diff frames like another's.
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
    // And the end it stops at rather than wraps past, the same shape
    // `diff-step-edge` reads: the walk asks for ten files, runs out, and
    // the rest answer false. `moved=true` beside it is what tells this
    // from a walk that was refused from the first press.
    Verb {
        name: "changes-step-edge",
        when: &[],
        plain: "moved=true stopped=true lit=true focused=true",
    },
    // The same list's folder rows, where the arrow is the whole of the
    // picture: `>` shut, `v` open. `turn=` is the rotation the icon is
    // actually drawn at rather than the model's flag — the two file
    // lists keep that flag in different fields, and a cell that read
    // only one of them left this list's arrow lying open in both states
    // while `shut=` went on answering true. Read as a pair: an arrow on
    // its own says nothing about which way it turned, so `-unfold`
    // strikes the same row a second time and has to come back to 90.
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
    // And the end it stops at rather than wraps past. `stopped=true`
    // is the refusal itself: the walk asks for twenty rows, gets as
    // far as the bottom, and the rest answer false. Without `moved=`
    // beside it a pane that refused every step from the start — never
    // on screen, never focused — would read the same.
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
    // `screenshot saved=true` is perfectly happy with (2026-08-23 実測:
    // with the toggle as the completion, every run shot the header
    // alone). A run fired with no path never opens anything and says
    // `diff_arg named=false` instead, so it fails here too.
    Verb {
        name: "preview",
        when: &[],
        plain: "diff_row act=preview ready=true",
    },
    Verb {
        name: "preview-unstaged",
        when: &[],
        plain: "diff_row act=preview-unstaged ready=true",
    },
    Verb {
        name: "preview-staged",
        when: &[],
        plain: "diff_row act=preview-staged ready=true",
    },
    // Both sides have to be named on some row, and on a file that has
    // been typed over both of them are removals — the half that used
    // to be dropped, which left the legend explaining a distinction
    // no row was making (2026-08-22 ユーザー報告). A count of zero on
    // either side photographs exactly like a file with nothing to
    // tell apart.
    Verb {
        name: "conflict-sides",
        when: &[],
        plain: "conflict_sides combined=true told=true both=true",
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
    // Not the row this one: `ready=` says the diff arrived, and a diff
    // that arrived is where this verb's failures start rather than
    // ends. The place is only kept if the rebuilt list came back to
    // it, so `at=` is read against `want=` — and `room=` is there
    // because the fixture is half of it: a file whose diff fits the
    // pane has no place to lose, scrolls nowhere, and photographs
    // exactly like one that lost nothing (`--preset manyhunks`).
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
    // The colours that land behind the rows, and the place they must
    // not cost. Neither half is a picture: a diff whose colours never
    // came frames as a language the set has no rules for, and a view
    // thrown back to the top frames as one nobody had scrolled. `at=`
    // is `scrollTo`'s own number read back after the swap — 400 or
    // the reader lost their place.
    Verb {
        name: "colour-place",
        when: &[],
        plain: "colour_place coloured=true at=400",
    },
    // The picture cannot tell a diff sent to its end from one that had
    // nowhere to go: both frame as a pane of text with its left edge
    // showing. So the two things that only happen when there is
    // somewhere to go are what it is judged on — the bar came out, and
    // the hand is still carrying the rows. A fixture whose lines fit
    // the pane fails here rather than passing on a blank
    // (`--preset widelines` is the one that does not fit).
    Verb {
        name: "code-send",
        when: &[],
        plain: "code_send bar=true hand=true",
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
    // when the pane closes on the reader instead of following.
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
