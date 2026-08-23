//! The line each verb has to be caught saying, for the verbs whose
//! failure the camera cannot see (`Outcome::must_say`).
//!
//! A table rather than a branch in the run: a verb that fails invisibly
//! is one line here, and the run stays about running.
//!
//! The verbs about remotes are a table of their own
//! ([`super::remote_verbs`]), asked first.

/// What `verb` has to say for its picture to be worth anything, or `None`
/// when the picture is the whole of it. `arg` is the verb's own argument:
/// one verb serves two panes and wants a different line for each.
#[expect(clippy::too_many_lines)]
pub(super) fn must_say(verb: &str, arg: &str) -> Option<&'static str> {
    if let Some(line) = super::remote_verbs::must_say(verb, arg) {
        return Some(line);
    }
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
        // commit fills the pane; in a history that fits, a viewport that
        // followed frames like one that never moved. `op=` is the half the
        // pair cannot say: both hold of a selection already at the tip.
        "revert-commit" => Some("tip_landed follows=true onscreen=true op=revert"),
        "cherry-pick" => Some("tip_landed follows=true onscreen=true op=cherry-pick"),
        "merge-branch" => Some("tip_landed follows=true onscreen=true op=merge"),
        // The other landing the same press has. Two halves the picture
        // cannot hold either: that no red line was written over an
        // ordinary conflict, and that the command log stayed down.
        "merge-stops" => {
            Some("merge_stopped wip=true conflicts=true error=false log=false msg=true cont=false")
        }
        // The same landing for the three that step. `cont=` flips over
        // from the merge above: their `--continue` is a step onward
        // rather than the commit somebody is writing, so the row stays —
        // and a card with a row missing frames exactly like one that has
        // it.
        "cherry-pick-stops" | "revert-stops" | "rebase-stops" => {
            Some("write_stopped wip=true conflicts=true error=false log=false cont=true")
        }
        // Where a move comes to rest, and whether the carry left an entry
        // behind. The two landings frame almost alike — a branch is a
        // branch — so the counts beside it are the whole claim, and the
        // one that says nothing was stashed is half of the pair.
        //
        // **`log=false` is on all three of the move claims.** A move the
        // screen knows git would refuse is never sent, so no refusal can
        // raise the command log — a red panel under a press that has a
        // way out on screen the whole time (2026-08-22 ユーザー判断).
        // A shut panel and a panel that was never raised are the same
        // picture, so this is the half only the report can carry.
        "switch-lands" if arg.starts_with("feature/topic-a") => {
            Some("switch_landed branch=feature/topic-a stashes=2 wanted=2 conflicts=1 log=false")
        }
        "switch-lands" => {
            Some("switch_landed branch=feature/clash stashes=0 wanted=0 conflicts=0 log=false")
        }
        // Bars with no words in them frame like bars with words: both are
        // judged on the held pill and the absent chip, neither of which a
        // run that photographed the opening could say.
        "move-ask" => Some("move_ask hold=true code= branch=main"),
        "rename-remote" => Some("rename_ask hold=true code="),
        // A bar that never came down and a bar that came down empty are
        // the same picture, and where the move ended is the half the
        // picture cannot answer at all: the run that answered the
        // question has to have landed somewhere else, and the run that
        // only raised it has to have landed nowhere.
        //
        // **The shape of the question is the claim.** A stopped
        // cherry-pick is put down with `--quit` and its tree goes into a
        // stash, so nothing is destroyed and the pill is an ordinary
        // click; a stopped rebase has to be aborted (its `--quit`
        // detaches HEAD and orphans the copies it made), so that one
        // keeps the hold. The two frame alike — a bar is a bar — and only
        // the chip and `hold=` tell them apart.
        // `branch=` is empty on purpose: a rebase runs on a detached
        // HEAD, so the working tree has no branch to name while it is
        // standing.
        "switch-stopped" if arg.starts_with("main") => Some(
            "switch_stopped code=rebase --abort accept= hold=true bang=false op=rebase branch= log=false",
        ),
        "switch-stopped" => Some(
            "switch_stopped code=stash accept= hold=false bang=false op=cherry-pick branch=main log=false",
        ),
        // The same question with one clause fewer: no operation to put
        // down, the unmerged index the reader's own `--quit` left is the
        // whole of what is in the way. `op=` empty is the half that says
        // so — the bar itself frames exactly like the one above it.
        "switch-conflicted" => Some(
            "switch_stopped code=stash accept= hold=false bang=false op= branch=main log=false",
        ),
        // And the one nothing here can clear: the pill goes to the copy
        // that has the branch instead of moving onto it, which is what
        // the `!` after the word says. No chip — git has no one command
        // for opening a working copy.
        "switch-held" => {
            Some("switch_stopped code= accept=Open hold=false bang=true op= branch=main log=false")
        }
        // The mark ahead of the row, which says the press raises a
        // question rather than moving. **Read from the report, not the
        // picture**: it is 16px in a full window, and the row it stands
        // on is the same row with it and without it. The argument names
        // which half of the pair the run is.
        "switch-mark" if arg.ends_with(":asks") => {
            Some("switch_mark offered=true asks=true want=asks indent=true")
        }
        "switch-mark" => Some("switch_mark offered=true asks=false want=plain indent="),
        "switch-stopped-go" | "switch-conflicted-go" => Some(
            "switch_stopped_landed branch=feature/clash op= conflicts=0 stashes=1 wanted=1 log=false",
        ),
        // The same landing reached through a carry, where the stash the
        // rewrite took out of its own way is still standing. Nothing
        // raises git's words over it any more, so the count is the claim:
        // a screen that lost the work and one that is holding it in the
        // stash frame nearly alike (規約 §未コミット変更がある状態で
        // 履歴を書き換える).
        "drop-stops" => {
            Some("write_stopped wip=true conflicts=true error=false log=false cont=true stashes=1")
        }
        // The other end of the same merge: the button under the card is
        // what finishes it, and an empty box commits the message git
        // wrote when it stopped. Neither half is in the picture — a
        // merge commit and an ordinary one draw the same row.
        "merge-commit" => Some("merge_committed merging=false kept=true typed=false"),
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
        // The two whose subject is the mark the leading row wears once the
        // rebuild a write asks for has landed — an archive box where a
        // stash was just made, the working tree's dashed ring where one
        // was just popped back. `gone=` is the row the write took away,
        // and it is what the run waited on; without the line the shot was
        // taken at the write barrier, which is the graph as it was.
        //
        // `stash named` leaves a summary in the commit box first, and
        // `named=` is the entry's own reflog subject held against it — read
        // off the sidebar's model, so a name that never left the box says
        // `false`. **Read as a pair with the bare verb**: one alone passes
        // a build that names every entry the same way. The picture settles
        // neither — the two rows are the same row with different text in
        // it, at a width that elides most of it.
        "stash" if arg == "named" => Some("graph_settled gone=true top=stash named=true"),
        "stash" => Some("graph_settled gone=true top=stash named=false"),
        // The same press, made from the working tree's own row with the
        // pane that describes it open — so the row the press takes away is
        // the one the reader is standing on. Where they are put instead is
        // the claim, and the picture holds none of it: the highlight left
        // on a vacated index lights the entry the press just made, which
        // at this width is a row like any other, and a viewport that never
        // moved frames the same way in a history this short.
        "stash-lands" => Some("stash_landed wip=false follows=true onscreen=true"),
        // `back=` is the other half of the same press: the entry's name is
        // in the commit box, where the entry is not there to hold it any
        // more. Read off the box, and invisible in the picture at any
        // width the summary field is drawn at.
        "stash-pop-row" => Some("graph_settled gone=true top=wip named=false back=true"),
        // The row taken away before git answered for it, held open for
        // the picture. `row=-1` is the sidebar without it and `chips=true`
        // the graph without its chip — **both halves at once** is the whole
        // of the rule, and a run that only ever reached the state after
        // the refs landed would say the same `row=-1` for a different
        // reason (デザイン規約 §消す操作は先に画面から消す).
        "delete-gone" => Some("gone_row tag=v0.3-local row=-1 total=2 chips=true"),
        // The band's Stash button, on each of the four things a working
        // tree can be. A button held down frames the same whichever
        // refusal put it there, so the reading is said out loud beside
        // what the band did with it — and the four are read as a set:
        // one alone passes a band that answers the same thing to
        // everything. `tip=` is the string the button would open, for
        // the reason `fetch-tip` gives below.
        "stash-state" if arg == "ready" => Some("stash_state mode=ready enabled=true tip=true"),
        "stash-state" if arg == "clean" => Some("stash_state mode=clean enabled=false tip=true"),
        "stash-state" if arg == "conflicts" => {
            Some("stash_state mode=conflicts enabled=false tip=true")
        }
        "stash-state" => Some("stash_state mode=unborn enabled=false tip=true"),
        // The fourth: an elided row whose hover says the whole name.
        // One verb serves both panes (the argument picks one), so the
        // wanted line names neither — `tree=` echoes which view the
        // argument asked for (`-tree` keeps the tree, where row 0 is
        // the elided folder chain), `tip=` the shared instance's own
        // visible.
        "path-tip" if arg.ends_with("-tree") => Some("tree=true tip=true"),
        "path-tip" => Some("tree=false tip=true"),
        // The eye at the end of the TAGS band. Both sides of it frame as
        // a graph with the same tags listed beside it, and what separates
        // them — a row, and every tag chip — depends on the repository, so
        // the picture cannot say on its own whether the switch answered.
        // `there=` is the named tag's commit, held against `shown=`: off
        // takes the commits nothing but a tag reaches out of the walk, and
        // pressing again brings them back.
        "tags-eye" if arg.ends_with(":back") => Some("tags_eye shown=true there=true"),
        "tags-eye" => Some("tags_eye shown=false there=false"),
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
        // The card that must *not* come out. A picture cannot carry an
        // absence on its own — an empty overlay would frame the same as a
        // run whose hover request never arrived — so the menu is judged
        // with it: `menu=true` says there was something for the card to be
        // behind. Read as a pair with `row-card`, which proves that same
        // input does open the card (2026-08-17 ユーザー報告).
        "menu-hover" => Some("menu_hover menu=true card=false"),
        // The card of a row's own message, and the row it came out of.
        // `lit=` is the half a picture answers badly: the band is one
        // shade off the ground under a card that covers the rows below
        // it, and the card takes the pointer off the row the moment the
        // hand walks in to read it — so the row went dark while its own
        // card stood, and nothing on screen said which commit the message
        // was of (2026-08-22 ユーザー報告).
        "row-card" => Some("row_card open=true lit=true"),
        // Where a graph row divides into the chip's half and the commit's,
        // asked at one point along the row. The picture shows which card
        // came out, but not which point was asked for or whether that is
        // the one the reader would have called it, so the run carries the
        // question with the answer and is judged on the two agreeing —
        // and on the other card being shut, since only one of the two is
        // ever meant to be out.
        "row-part" => Some("agrees=true"),
        // A row's tooltip. `lit=` is the control — the tip that came up
        // under the same pointer on the way in, so a run that photographs
        // an empty overlay is showing the row's answer and not a pointer
        // that never landed. `wants=` is the row's own words and `tip=`
        // the shared instance carrying them: every row of the sidebar
        // spells its name in full, so both are judged.
        "nav-tip" => Some("lit=true wants=true tip=true"),
        // The left menu's rename gesture, and the absence that is the
        // whole of its bug: a click landing in the folded list's section
        // after that section went away and came back is an ordinary
        // click, not the gesture's second one. A picture cannot carry it
        // — the run that armed nothing frames as a folded rail with a
        // section beside it, which is what `nav-peek` frames as too. So
        // the pair is read out loud: `armed=` is the row's own answer to
        // the click it was given, `collapsed=`/`box=` what came of it
        // (the box puts the whole list back over the diff the fold was
        // made for — 2026-08-18 ユーザー報告).
        "nav-reclick" => Some("away=false armed=true collapsed=true box=true focused=true"),
        "nav-reclick-away" => Some("away=true armed=false collapsed=true box=false"),
        // The same claim by the other door — the menu's `beginRename` —
        // and the rule it answers to: a box opens where its row is, so a
        // peeked row keeps both the fold and whatever the fold was made
        // for. `diff=` is the half that only the second one has a file
        // to lose.
        "nav-peek-rename" => Some("collapsed=true diff=false editing=branch:"),
        "diff-fold-by-rename" => Some("collapsed=true diff=true editing=branch:"),
        // The two ways the box is walked away from with nothing typed
        // into it. Same blind spot: a pane with no box in it frames like
        // a pane that never opened one.
        // And the row the box lands on, brought back from wherever the
        // list had scrolled to. A picture cannot carry it either: a
        // section showing its last rows and one showing its first frame
        // the same, and 2,000 tags all read alike.
        "nav-rename-far" => Some("shown=true box=true"),
        "nav-rename-drop" if arg == "fold" => Some("nav_drop how=fold collapsed=true box=false"),
        "nav-rename-drop" => Some("nav_drop how=away collapsed=false box=false"),
        // The run that walks the list on past the row a box is standing
        // on: the box has to go with it, which is the whole of what a box
        // drawn outside its list owes. A sidebar with no box in it is the
        // same picture whether the box went or never opened.
        "nav-branch-box" | "nav-rename-box" if arg.ends_with(":away") => {
            Some("open=true focused=true shown=false")
        }
        // The two name boxes a sidebar row opens, held at the width the
        // argument dragged the pane to. Whether what is in the box is cut
        // is in the picture; the other two are not — a box drawn where
        // nothing can be typed frames exactly like one waiting for a
        // name, and a box drawn outside its list has one more way to be
        // missing than a box in a row has.
        "nav-branch-box" | "nav-rename-box" => Some("open=true focused=true shown=true"),
        // The same walking away, over the graph: a press that landed
        // somewhere else takes an empty box with it and leaves one with
        // something typed in it standing. Both halves are read off the
        // report because the picture holds only one of the two states,
        // and the card the run is about fades — so `shown=` says which
        // end of that fade the picture was taken at.
        "find-drop" if arg.is_empty() => Some("find_drop open=false shown=false"),
        "find-drop" => Some("find_drop open=true shown=true"),
        "name-box-drop" if arg.contains(':') => Some("name_drop box=true"),
        "name-box-drop" => Some("name_drop box=false"),
        // A window whose panel the press took down and a window that
        // never raised one frame the same, and the mark the press also
        // quiets is 12 pixels of it in a corner. `was=` is judged with
        // both — a refusal that never landed leaves a mark that was
        // never red and a panel that was never up, and clearing nothing
        // would pass.
        "commands-clear" => Some("commands_clear was=true wrong=false open=false"),
        // Recovery is the show, and the picture can only hold its quiet
        // half: a band that failed and healed ends the run looking like
        // one that never failed at all. `was=`/`hadline=` are the red
        // half — without them, a fetch that never failed raised no line,
        // and taking down nothing would pass as recovery.
        "fetch-recover" => {
            Some("fetch_recover was=true hadline=true wrong=false line=false failures=0")
        }
        // A run of failures cut short photographs the warning shape,
        // which is a real state and the one the short arguments are for
        // — so which shape the run came to rest in is said out loud.
        // `stopped=` is the tab's own suspension, the thing that puts
        // the word `Resume` on the button. The counts ride after it
        // unjudged: the fetch an opening fires lands its own failure
        // inside the run, and where it lands moves with the machine.
        "fetch-fail" if arg.is_empty() || arg == "1" || arg == "2" => {
            Some("fetch_fail stopped=false")
        }
        "fetch-fail" => Some("fetch_fail stopped=true"),
        // The resume on the end of such a run, whose whole show is over
        // before the picture is taken: what it ends on is a button back
        // at work, and that frames exactly like `fetch-fail 1`.
        "fetch-resume" => Some("fetch_resume stopped=true fetched=true"),
        // Nothing is pressed in this one, so the picture on its own is a
        // graph — and a graph that fetched and one that did not frame the
        // same way. `fails=0` is the other half: a run whose opening fetch
        // came back with something to say reached its rows some other way.
        "open-fetches" => Some("open_fetch fails=0"),
        // Intermediate communication is valid only after the real busy
        // edge was observed and latched for the asynchronous image grab.
        "force-push-hold" => Some("push_hold mode=diverged busy=true"),
        // The bare button's wait. `framed=false` is judged rather than
        // looked at: a frame that grew while git was out would be a line
        // in the picture, but a frame that did not grow is nothing at
        // all, and nothing is what a correct run looks like too.
        "fetch-busy" => Some("fetch_busy busy=true fails=0 framed=false"),
        // Both halves are absences on the picture: a dim button says
        // nothing about why it is dim, and a tooltip that stays away
        // frames exactly like one that was never asked for. The argument
        // names which side of the pair the run is.
        "fetch-tip" if arg == "off" => Some("fetch_tip enabled=false tip=false remotes=0"),
        "fetch-tip" => Some("fetch_tip enabled=true tip=true"),
        // The same popup has a real loading and a causally settled form;
        // neither is selected by a millisecond window. Both are opened by
        // the field's own press, and `typing=` is the half of that press a
        // photograph cannot answer: the list must come down without taking
        // the caret out of the box it came from.
        "settings-tools-loading" => Some("settled=false loading=true open=true typing=true"),
        "settings-tools" => Some("settled=true loading=false open=true typing=true"),
        // The same card's avatar half. Only this one of its four wants a
        // line: the other three cannot reach the camera with the wrong
        // state, because each waits on the state itself — a filed picture
        // in the list, a lit row, a row gone from the store — and a run
        // where any of that never came photographs nothing at all. What
        // no predicate covers is the second grab: the list is a popup, it
        // lives in overlay.png, and a mirror refreshed over an empty
        // overlay passes (the blind spot `commit-menu` is here for). Three
        // popups because the card is modal — its dimmer, itself, and the
        // list that came down inside it.
        "avatar-combo" => Some("overlay saved=true popups=3"),
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
        // The dotted edges under the uncommitted row, in the tokens the
        // delegate paints from (uppercase = dashed). Under `--preset
        // conflict` a merge is standing, so the row leashes HEAD and the
        // side being brought in: two lanes, both dotted. A lane is a
        // couple of pixels wide and its dashes are one each, so a row
        // that leashed only HEAD photographs as very nearly the same
        // picture.
        "wip-lanes" => Some("wip_lanes geometry=O0.0;O1.1"),
        // The stand-in for a HEAD scrolled off. The five read as one set:
        // a band that is there and a band that is gone frame the same
        // way, and which edge it took is a difference of a few hundred
        // pixels in a picture of two thousand rows. `onScreen=` is the
        // other half of every one of them — the stand-in is wanted
        // exactly while its row is not in sight, so a run where both are
        // true (or neither) has the rule backwards.
        "graph-head" => Some("shown=true above=true onScreen=false"),
        "graph-head-below" => Some("shown=true above=false onScreen=false"),
        "graph-head-lit" => Some("shown=true above=true onScreen=false lit=true"),
        // And the two that take it away again: the press has to leave the
        // selection on the row it led to, which is the whole of what it
        // is for; the scroll back only has to bring the row into sight.
        "graph-head-go" => Some("shown=false above=false onScreen=true lit=false landed=true"),
        "graph-head-back" => Some("shown=false above=false onScreen=true lit=false"),
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
        // What a tab switch carries and what it drops, in one line
        // because neither half means anything alone. `sessions=1` with
        // two tabs open is the release itself — the tab left behind is
        // holding no repository — and it is the one thing here no picture
        // can say. `folded=` / `log=` are the layout the reader left the
        // last tab in, found on the next one; `empty=` is that tab's own
        // commit editor, which the words did *not* follow into; `back=`
        // is those words still standing where they were typed, after
        // everything else on that page was thrown away and read again;
        // `wip=` is the pane holding them being the one on screen, which
        // is the half of "kept" that a report about text alone misses.
        "tab-carry" => Some("sessions=1 folded=true log=true empty=true back=true wip=true"),
        // Which tab was carried and where it came to rest. A strip whose
        // order merely changed passes with any two tabs swapped, and one
        // where the carry never took hold photographs the order it
        // started in — which is a strip that looks like every other one.
        // `moved=` is the carried tab's own path, found at the place it
        // was asked for.
        "tab-drag" => Some("tab_drag moved=true"),
        // The other half of the same gesture, and the one the settled
        // strip cannot hold: a strip whose tab was never drawn away from
        // its own row photographs exactly like one whose rows only ever
        // jumped. `lifted=` is the offset the transform is carrying, read
        // off the tab rather than off what was asked of it.
        "tab-hold" => Some("tab_hold lifted=true"),
        // And the strip travelling under a tab held past its end. The
        // picture is a scrolled strip either way — the one that travelled
        // and the one that was already there frame alike — so what is
        // judged is that the tab reached the far end of an order it could
        // not see when the hand took hold.
        "tab-edge" => Some("tab_edge landed=true"),
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
        // is lit and, once `card=true`, whose commit fills the right-hand
        // pane — the two catch up one after the other, and a shot between
        // them holds a different commit in each. What it cannot hold is
        // the walk working at all: the keyboard on the list, the settle
        // behind a held key landing the selection, the viewport carrying
        // the row stepped onto. A walk that moved nothing sits still.
        "graph-step" => Some(
            "landing=in back=false refused=0 focused=true diff=false onscreen=true selected=true card=true",
        ),
        // The other two landings, which no picture holds: a row brought in
        // flush against the bottom one step at a time, and a row centered
        // because the one it stepped off was nowhere on screen. Both frame
        // as a graph with a lit row somewhere in it.
        "graph-step-edge" => Some("landing=edge back=false refused=0 focused=true"),
        "graph-step-far" => Some("landing=center back=false refused=0 focused=true"),
        // The refusing half. `back=true` is the whole of it — nothing
        // moved — and it is worth nothing without `refused=`, since a walk
        // that was never attempted leaves the same row lit. Read it beside
        // a plain `graph-step`: a step that always refuses passes it on
        // its own.
        "graph-step-named" => Some("back=true refused=1"),
        // And the one that must *not* refuse: a half-written message is a
        // draft, and the arrows walk off it the way a click does — so
        // what it has to say is a plain step's answer, word for word.
        "graph-step-dirty" => Some(
            "landing=in back=false refused=0 focused=true diff=false onscreen=true selected=true card=true",
        ),
        // A diff opened over the graph. `focused=false` is the mechanism —
        // Qt leaves active focus on a pane it has just swapped away, and
        // the keys go on arriving there — and `diff=true` is what the
        // report was about: a step behind the diff moves the selection,
        // and moving the selection closes the diff, so the screen jumps
        // back to the graph. A picture of the diff still standing is also
        // a picture of a run where the arrow was never pressed.
        "graph-step-diff" => Some("back=true refused=1 focused=false diff=true"),
        // The file list's arrows, and the half of the keyboard rule the
        // diff's own arrows are the other half of: nothing here pressed the
        // diff, so `focused=true` says the list *kept* the keyboard when the
        // diff opened over the graph. `lit=true` is the row painted as the
        // one being read — the rectangle's own answer, not the condition
        // behind it — and `moved=true` says the diff followed the light
        // rather than only the light moving. The picture is a weak witness:
        // one file's diff frames like another's.
        "changes-step" | "wip-step" => Some("moved=true stopped=false lit=true focused=true"),
        // And the end it stops at rather than wraps past, the same shape
        // `diff-step-edge` reads: the walk asks for ten files, runs out, and
        // the rest answer false. `moved=true` beside it is what tells this
        // from a walk that was refused from the first press.
        "changes-step-edge" => Some("moved=true stopped=true lit=true focused=true"),
        // The diff's own arrows, where the picture is the weakest witness
        // in the app: a diff scrolled two rows and a diff never scrolled
        // at all are the same photograph of the same file. Everything that
        // matters is in the line. `focused=true` is the hand's arrival
        // taking the keyboard — the pane does not take it by appearing, so a
        // false here means the walk was pressing on a pane the file list
        // still owns — and `moved=true` with `atEnd=false stopped=false` is a
        // walk that had somewhere to go and went there.
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
        // Both sides have to be named on some row, and on a file that has
        // been typed over both of them are removals — the half that used
        // to be dropped, which left the legend explaining a distinction
        // no row was making (2026-08-22 ユーザー報告). A count of zero on
        // either side photographs exactly like a file with nothing to
        // tell apart.
        "conflict-sides" => Some("conflict_sides combined=true told=true both=true"),
        // The tick that asks whether the open file still reads the way it
        // did. What it is asking about did not move during the run, so
        // the read is answered with silence — the ask is the only edge
        // there is, and `asked=true` is the whole of what this side can
        // claim: the slot resolved, the pane owned the file it named, and
        // the session took the read. `loading=false` is the other half of
        // the wiring: a tick that put the pane back into a click's state
        // would blank it once a poll (`RepoSession::refresh_diff`).
        "diff-tick" => Some("diff_tick asked=true loading=false"),
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
        // Three lines staged one after another, each pressed the moment
        // the pane will take one. The count is the whole judgement: a pane
        // that stops taking presses after the first stops here too, and
        // the picture of it is a diff either way.
        "line-run" => Some("line_run staged=3 want=3"),
        // A bucket emptied from its own heading keeps that heading, both
        // ways round — which is the whole claim, so both headings are
        // read back whichever direction was pressed. The picture cannot
        // be trusted with it: a list with one heading missing frames as a
        // list, and the missing one is only missing next to the other
        // direction's picture.
        "stage-all" => Some("wip_heads from=unstaged unstaged=true staged=true"),
        "unstage-all" => Some("wip_heads from=staged unstaged=true staged=true"),
        // The conflicted heading is the one that answers the other way:
        // it stands only while git has something unmerged, so marking the
        // whole bucket resolved has to take it off the screen. `staged=`
        // rides along to say where the files went — `git add --all` would
        // have emptied the unstaged bucket with them, and the picture of
        // that is the same picture.
        "resolve-all" => Some("wip_heads from=conflicts unstaged=true staged=true conflicts=false"),
        _ => None,
    }
}
