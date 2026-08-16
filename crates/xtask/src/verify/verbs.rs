//! The line each verb has to be caught saying, for the verbs whose
//! failure the camera cannot see (`Outcome::must_say`).
//!
//! A table rather than a branch in the run: a verb that fails invisibly
//! is one line here, and the run stays about running.

/// What `verb` has to say for its picture to be worth anything, or `None`
/// when the picture is the whole of it. `arg` is the verb's own argument:
/// one verb serves two panes and wants a different line for each.
pub(super) fn must_say(verb: &str, arg: &str) -> Option<&'static str> {
    match verb {
        "solo" => Some("solo blocked=true"),
        "details-fit" => Some("details_fit fits=true"),
        // A pull that took more room than the pane had leaves what sits
        // under the box — the author card, the commit button — drawn over
        // the window's own footer, and that frames like a pane that fits:
        // the same blind spot details-fit answers for. Only that half is
        // judged here: whether the grip was offered at all depends on the
        // message, and the run where it stays away is half of the pair.
        "details-grow" | "details-grow-squeeze" | "wip-grow" | "wip-grow-squeeze" => {
            Some("description_grow keeps=true")
        }
        // Half of these the picture holds — which row is lit, and whose
        // commit fills the right-hand pane. The other half it cannot: in a
        // history short enough to fit on screen, a viewport that followed
        // the new commit and one that never moved frame the same way.
        "revert-commit" | "cherry-pick" | "merge-branch" => {
            Some("tip_landed follows=true onscreen=true")
        }
        // The caret is the whole of these two, and a hook that never
        // reached the box leaves a picture of the resting colour --
        // which is a real state, and the other half of each pair.
        "edit-message-focus" => Some("message_focus pane=details focused=true"),
        "wip-message-focus" => Some("message_focus pane=wip focused=true"),
        // A dialog that stayed open because the write did not take looks
        // exactly like one nobody has answered yet, and "which half
        // landed" is not something a picture holds at all.
        "identity" => Some(
            "identity state=missing dialog=true nameSaved=false emailSaved=false unsaved=false",
        ),
        "identity-half" => {
            Some("state=ready dialog=true nameSaved=true emailSaved=false unsaved=true")
        }
        // The three forced tooltips: a hook that never reached its
        // target photographs the resting state, which is a real state.
        // `tip=` is the attached ToolTip's own visible — the output
        // side, as everywhere.
        "identity-tip" => Some("identity_tip unsaved=true badge=true tip=true"),
        "signature-tip" => Some("signature_tip code=E tip=true"),
        "stash-tip" => Some("stash_tip blocked=true tip=true"),
        // The fourth: an elided row whose hover says the whole name.
        // One verb serves both panes (the argument picks one), so the
        // wanted line names neither — `tree=` echoes which view the
        // argument asked for (`-tree` keeps the tree, where row 0 is
        // the elided folder chain), `tip=` the shared instance's own
        // visible.
        "path-tip" if arg.ends_with("-tree") => Some("tree=true tip=true"),
        "path-tip" => Some("tree=false tip=true"),
        // The two verbs whose whole picture is the card in overlay.png,
        // and the one line that says the card is in it: `popups=` is the
        // window overlay's own count of what it was holding when the
        // mirror was refreshed for the shot, so a blank overlay.png can
        // only mean nothing was open. Any verb whose subject is a menu, a
        // dialog or a tooltip can be judged the same way — these two are
        // where it bit (2026-08-16: two of four concurrent `commit-menu`
        // runs photographed a blank overlay and passed). `reset-menu`
        // counts two because the submenu is a popup of its own.
        "commit-menu" => Some("overlay saved=true popups=1"),
        "reset-menu" => Some("overlay saved=true popups=2"),
        // An emptied panel with a red mark over it and an emptied panel
        // with a quiet one frame the same from the waist down: the panel
        // is the picture, and the mark is 12 pixels of it in a corner.
        // `was=` is judged with it — a refusal that never landed leaves
        // a mark that was never red, and clearing nothing would pass.
        "commands-clear" => Some("commands_clear was=true wrong=false"),
        // Recovery is the show, and the picture can only hold its quiet
        // half: a band that failed and healed ends the run looking like
        // one that never failed at all. `was=`/`hadline=` are the red
        // half — without them, a fetch that never failed raised no line,
        // and taking down nothing would pass as recovery.
        "fetch-recover" => {
            Some("fetch_recover was=true hadline=true wrong=false line=false failures=0")
        }
        // Intermediate communication is valid only after the real busy
        // edge was observed and latched for the asynchronous image grab.
        "force-push-hold" => Some("push_hold mode=diverged busy=true"),
        // The same popup has a real loading and a causally settled form;
        // neither is selected by a millisecond window.
        "settings-tools-loading" => Some("settled=false loading=true open=true"),
        "settings-tools" => Some("settled=true loading=false open=true"),
        // `edge=` rides along in that report but is not judged: whether an
        // edge would land off the screen is a question about a real
        // monitor, and the offscreen platform has none to answer with.
        "window-fill" => Some("window_fill fills=true"),
        // A window held at its floor and one let past it frame alike — the
        // picture is of the panes either way, and the one that went past
        // simply has a pane outside the frame, where a screenshot cannot
        // follow. What the floor came to is a number or it is nothing.
        "window-floor" => Some("window_floor fits=true"),
        // The graph column pulled past its floor: the clamp has to land
        // on the floor exactly, and the floor is the message tick
        // brought up against lane 0's co-author badge without touching
        // it (GraphPane.graphColWMin). The number is the token
        // arithmetic spelled out — it moves only when those tokens do,
        // and a clamp that stopped anywhere else photographs just as
        // neatly, since the gap in question is one pixel of the frame.
        "graph-min" => Some("graph_min w=21 min=21"),
        // A drag carried past one of a divider's bounds. The badge is 12
        // pixels in the middle of a pane, and a run where the hook never
        // reached the divider photographs a window that looks entirely
        // well — so the refusal is said out loud. `line=` rides with it
        // because the two are a pair: the boundary still moves the other
        // way, and one that withdrew its line would be answering a
        // different question (that is `graph-divider`'s squeezed half).
        // One wanted line for every case: `line=` is already whichever
        // divider has the hand, so the argument does not change it.
        "divider-refuse" => Some("divider_refuse refuses=true line=true"),
        // The end of a history and the end of what was loaded are the
        // same picture but for one line, and a footer that failed to draw
        // takes that line with it — so the cut says itself. `shown=` is
        // the footer's own visible, beside the model's answer: the two
        // are what the verb is for, and only neighbours are caught in one
        // substring.
        "graph-tail" => Some("graph_tail truncated=true shown=true"),
        // Which column the middle click landed in is the whole question,
        // and a photograph answers neither half of it: lanes carried
        // sideways and lanes left where they were frame alike at this
        // size, and so do a gesture that panned and one that never
        // started. The two halves want opposite lines, which is what the
        // argument is for.
        "middle-scroll" if arg == "message" => Some("middle_scroll lanes=false x=0"),
        "middle-scroll" => Some("middle_scroll lanes=true"),
        // Which tab went is the whole question, and every demo working
        // tree is called `repo`, so the strip photographs the same either
        // way. `gone=` is the pressed tab's own path, asked of the strip
        // after it caught up with the model — a run that closed the
        // neighbour, or one that reported before a tab had ever opened,
        // is caught by nothing else. How many are left is not judged: the
        // verb takes as many repositories as it is given.
        "middle-close" => Some("middle_close gone=true"),
        // The three states themselves. A run where one of them never stood
        // photographs a band that was never crowded, and that picture
        // cannot be told from a band that gave the crowd room — so the
        // crowd has to be said out loud.
        //
        // Neither the fit nor which of the group's three shapes landed is
        // judged: this verb takes a width, and the widths that show the
        // last shape are below the floor a hand can drag the window to
        // (`fits=false` is what was asked for there). The floor itself is
        // `window-floor`'s question.
        "badges" => Some("op=true conflicts=true identity=true"),
        // The card, opened. `rows=` is the half the picture cannot carry
        // on its own: a card with one row and a card with three frame the
        // same way once it is cropped to the band, and which rows arrived
        // is the whole question the group raises when it gives way.
        "badges-hover" => Some("card=true rows=op,conflicts,identity"),
        // The fourth badge. A run whose shim never reached PATH reads the
        // git this machine has, wears no badge, and photographs an
        // ordinary window — which is exactly what an ordinary window looks
        // like. `badge=` is the band's own reading, so the whole path from
        // `git --version` to the row is what passes or fails here.
        "old-git" => Some("old-git badge=true"),
        // And the card it opens. `rows=` is not judged: the other three
        // rows come and go with the machine (a container with no identity
        // configured stands one of them), and only this row is the verb's.
        "old-git-card" => Some("old-git badge=true card=true"),
        // The band folded with nothing red standing in it. Two halves, and
        // the picture holds neither on its own: a mark that never came up
        // frames as a band with room to spare, and the colour of three
        // dots is not something a cropped screenshot settles an argument
        // about. `tint=` is named rather than spelled in hex — what is
        // being judged is which rule painted it (規約 §状態: 最も重い状態が
        // 決める), and a red mark over a lone warning is the way that rule
        // fails silently.
        "old-git-fold" => Some("mark=true tint=warning"),
        // Walking the graph with the arrows. The picture holds which row
        // is lit and whose commit fills the right-hand pane, but not the
        // three things that make the walk work: that the keyboard was on
        // the list at all, that the settle behind a held key landed the
        // selection, and that the viewport carried the row it stepped
        // onto. A walk that moved nothing frames as a graph sitting still,
        // which is what a graph does most of the time.
        "graph-step" => Some(
            "landing=in held=false back=false refused=0 focused=true diff=false onscreen=true selected=true",
        ),
        // The other two landings, which no picture holds: a row brought in
        // flush against the bottom one step at a time, and a row centered
        // because the one it stepped off was nowhere on screen. Both frame
        // as a graph with a lit row somewhere in it.
        "graph-step-edge" => Some("landing=edge held=false back=false refused=0 focused=true"),
        "graph-step-far" => Some("landing=center held=false back=false refused=0 focused=true"),
        // The refusing halves. `back=true` is the whole of them — nothing
        // moved — and it is worth nothing without `refused=`, since a walk
        // that was never attempted leaves the same row lit. Read them
        // beside a plain `graph-step`: a step that always refuses passes
        // these two on its own.
        "graph-step-named" => Some("held=false back=true refused=1"),
        "graph-step-dirty" => Some("held=true back=true refused=1"),
        // A diff opened over the graph. `focused=false` is the mechanism —
        // Qt leaves active focus on a pane it has just swapped away, and
        // the keys go on arriving there — and `diff=true` is what the
        // report was about: a step behind the diff moves the selection,
        // and moving the selection closes the diff, so the screen jumps
        // back to the graph. A picture of the diff still standing is also
        // a picture of a run where the arrow was never pressed.
        "graph-step-diff" => Some("back=true refused=1 focused=false diff=true"),
        // The diff's own arrows, where the picture is the weakest witness
        // in the app: a diff scrolled two rows and a diff never scrolled
        // at all are the same photograph of the same file. Everything that
        // matters is in the line. `focused=true` is the arrival taking the
        // keyboard — nothing pressed this pane, so a false here means the
        // arrows would have been dead in a real window — and `moved=true`
        // with `atEnd=false stopped=false` is a walk that had somewhere to
        // go and went there.
        "diff-step" => Some("moved=true atEnd=false stopped=false focused=true"),
        // And the end it stops at rather than wraps past. `stopped=true`
        // is the refusal itself: the walk asks for twenty rows, gets as
        // far as the bottom, and the rest answer false. Without `moved=`
        // beside it a pane that refused every step from the start — never
        // on screen, never focused — would read the same.
        "diff-step-edge" => Some("moved=true atEnd=true stopped=true focused=true"),
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
        "diff-file" => Some("diff_row act=diff-file ready=true"),
        "line-tools" => Some("diff_row act=line-tools ready=true"),
        "hunk-tools" => Some("diff_row act=hunk-tools ready=true"),
        "stage-hunk" => Some("diff_row act=stage-hunk ready=true"),
        "stage-line" => Some("diff_row act=stage-line ready=true"),
        // Not the row this one: `ready=` says the diff arrived, and a diff
        // that arrived is where this verb's failures start rather than
        // ends. The place is only kept if the rebuilt list came back to
        // it, so `at=` is read against `want=` — and `room=` is there
        // because the fixture is half of it: a file whose diff fits the
        // pane has no place to lose, scrolls nowhere, and photographs
        // exactly like one that lost nothing (`--preset manyhunks`).
        "keep-place" => Some("diff_place at=400 want=400"),
        "discard-hunk" => Some("diff_row act=discard-hunk ready=true"),
        "discard-hunk-go" => Some("diff_row act=discard-hunk-go ready=true"),
        // The colours that land behind the rows, and the place they must
        // not cost. Neither half is a picture: a diff whose colours never
        // came frames as a language the set has no rules for, and a view
        // thrown back to the top frames as one nobody had scrolled. `at=`
        // is `scrollTo`'s own number read back after the swap — 400 or
        // the reader lost their place.
        "colour-place" => Some("colour_place coloured=true at=400"),
        // The picture cannot tell a diff sent to its end from one that had
        // nowhere to go: both frame as a pane of text with its left edge
        // showing. So the two things that only happen when there is
        // somewhere to go are what it is judged on — the bar came out, and
        // the hand is still carrying the rows. A fixture whose lines fit
        // the pane fails here rather than passing on a blank
        // (`--preset widelines` is the one that does not fit).
        "code-send" => Some("code_send bar=true hand=true"),
        // A line staged from the diff, the file then moved from the list,
        // and the diff following both. The picture is the last frame of
        // three and cannot show the two before it, so all three answers
        // are read: the rows shrank, the rows came back, and the pane
        // followed the file over to the staged side when the unstaged one
        // ran out (`--preset manyhunks` has the one file, so that is where
        // it has to land).
        "line-back" => Some("line_back back=true shrank=true followed=staged:notes.txt"),
        // Where the pane lands when the file under it is moved whole. The
        // picture shows a diff either way and cannot say which file it is
        // of, so the landing is read: `shown=true` is the half that fails
        // when the pane closes on the reader instead of following.
        "diff-follow" => Some("diff_follow shown=true"),
        // A bucket emptied from its own heading keeps that heading, both
        // ways round — which is the whole claim, so both headings are
        // read back whichever direction was pressed. The picture cannot
        // be trusted with it: a list with one heading missing frames as a
        // list, and the missing one is only missing next to the other
        // direction's picture.
        "stage-all" => Some("wip_heads from=unstaged unstaged=true staged=true"),
        "unstage-all" => Some("wip_heads from=staged unstaged=true staged=true"),
        _ => None,
    }
}
