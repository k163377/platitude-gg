pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The sidebar's own verbs: folding and peeking at a section, the second click on a row that is already lit,
/// filtering, closing, and the rows that answer for a tag.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var branchesModel: driver.branchesModel
    readonly property var remotesModel: driver.remotesModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var diffPane: driver.diffPane
    readonly property var navProbe: driver.navProbe
    readonly property var remoteDialog: driver.remoteDialog
    readonly property var refSwitchItem: driver.refSwitchItem
    readonly property var commitMenu: driver.commitMenu
    readonly property var dropCommitItem: driver.dropCommitItem
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "nav-dbl") {
            // A double-click in the left menu, entered where the row enters it. The argument is `<section>:<name>`.
            const cut = arg.indexOf(":")
            const section = arg.substring(0, cut)
            const rowName = arg.substring(cut + 1)
            const model = section === "tag" ? tagsModel : section === "remote" ? remotesModel : branchesModel
            sidebarPane.activateRow(section, rowName, rowName,
                                    model.oidOfName(rowName))
        } else if (act === "nav-fold" || act === "nav-peek"
                   || act === "nav-unfold" || act === "nav-peek-rename"
                   || act === "nav-peek-away" || act === "nav-peek-into"
                   || act === "nav-peek-out" || act === "nav-peek-shut") {
            // Hover cannot be injected, so the rail cell is named. "-away" walks the pointer off the cell, "-into" down
            // into the opened list, "-out" on out the far side (the exit no cell can see), "-shut" clicks the cell;
            // only "-into" leaves the section standing. "nav-peek" on an empty section stays shut
            // (NavRail.enterAt decides — `--preset empty` reads that side).
            if (arg === "no-tags")
                repoTab.setTagsShown(false)
            page.foldByHand(true)
            if (act === "nav-peek")
                navProbe.peekAt(arg)
            else if (act === "nav-peek-away") {
                navProbe.peekAt(arg)
                navProbe.peekAway(arg)
            } else if (act === "nav-peek-into" || act === "nav-peek-out") {
                navProbe.peekAt(arg)
                navProbe.peekInto(arg)
                if (act === "nav-peek-out")
                    navProbe.peekOut()
            } else if (act === "nav-peek-shut") {
                navProbe.peekAt(arg)
                navProbe.peekTap(arg)
            } else if (act === "nav-unfold")
                page.foldByHand(false)
            else if (act === "nav-peek-rename") {
                // Typing a name into a peeked row: the box lands on that row, in the section standing beside the rail,
                // and holds it open — the list is not put back for it (SidebarPane.startEdit).
                navProbe.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            }
            navRailTimer.start()
        } else if (act === "nav-peek-open") {
            // A row opened on the section standing beside the folded rail. **The row grows inside a popup**: what
            // this reads is that the section makes room for it and stays up, rather than taking itself — and the
            // row — down as the rows under it move.
            page.foldByHand(true)
            navProbe.peekAt("branch")
            peekOpenTimer.want = arg === "" ? 0 : Number(arg)
            peekOpenTimer.start()
        } else if (act === "tags-eye") {
            // The eye pressed at the band itself, with the list left standing beside the graph: what the switch is
            // about is the graph, and TAGS keeping its count while the graph loses a row is half of what the picture
            // says. The argument names the tag whose commit is the subject — `<tag>` presses once, `<tag>:back`
            // presses again afterwards so the round trip is read whole.
            const backing = arg.endsWith(":back")
            const tagName = backing ? arg.substring(0, arg.length - ":back".length) : arg
            acts.tagEyeOid = tagsModel.oidOfName(tagName)
            acts.tagEyeBack = backing
            acts.tagEyeStep = 0
            tagEyeTimer.start()
        } else if (act === "nav-jump") {
            // One click on a row of the left panel, and where it leads (デザイン規約 §左メニューの所作). The argument is
            // `<section>[:<row>]`, the row 0 by default. The press goes in at the row's own `leftClick`
            // (`NavList.clickRow`), so what routes it is the delegate's own answer and not a copy of it here.
            jumpTimer.start()
        } else if (act === "nav-reclick" || act === "nav-reclick-away") {
            // The rename gesture, on the section the folded rail has open. The plain verb clicks the same row twice
            // with that section standing; "-away" lets the pointer leave in between, so the two clicks land in a list
            // that went and came back — and that one reads as a first click again (デザイン規約 §左メニューの所作).
            // The argument is `<section>[:<row>]`.
            page.foldByHand(true)
            acts.reclickAway = act === "nav-reclick-away"
            acts.reclickArmed = false
            acts.reclickStep = 0
            reclickTimer.start()
        } else if (act === "nav-add-remote") {
            // The `+` at the end of the REMOTES band, which stays live while the band itself has gone unavailable
            // (デザイン規約 §左メニューの所作). Put in at the band's own signal — clicks cannot be injected (verify-ui スキル) —
            // so what answers is the page's own wiring. `--preset noremote` is the half worth reading: a section of no
            // rows is where this is worth pressing.
            //
            // "folded" is the other door onto the same dialog: the rail's own REMOTES cell, which at zero has no
            // section to open and answers the click with this instead. It goes in at `NavRail.tapAt`, the one answer
            // the cells give (`peekTap`), so the routing being tested is the cell's.
            if (arg === "folded") {
                page.foldByHand(true)
                navProbe.peekTap("remote")
            } else {
                navProbe.tapAddRemote()
            }
            navAddRemoteTimer.start()
        } else if (act === "doors-held") {
            // Every door onto the history while a write that replays is running behind the screen: the left pane's,
            // and the graph's beside it. Nothing is frozen and nothing dims (`SidebarPane.doorsHeld`). **The rebase is
            // a real one** — the argument names the branch to rebase onto, and the same one the menus are then opened
            // on — and the state comes from `RepoTab.replaying`. A demo repository
            // answers inside one beat of the sampler, so the rise is caught at the signal below and held for the
            // picture. **One run for both panes**: standing this state up twice would be two runs of the same thing.
            acts.heldBranch = arg
            repoTab.rebase(arg, "", true)
            navHeldTimer.start()
        } else if (act === "nav-pin-edge") {
            // The current branch's stand-in riding the edge its own row went out of (`HeadPinRow`): the branch list
            // is taken to its end, which puts the row off above it. **The scroll is what this is for** — the other
            // way the stand-in is on screen has no row anywhere to ride above and is answered off the model alone
            // (`seated`, which `nav-tip head` produces with a filter), so without a list that moved under it the
            // term this verb is about is never read.
            //
            // **The edge below it has no repository here that can make it.** It wants the current branch sorted
            // past the last row the section can show, and every preset checks out a name that sorts early
            // (`stack`, the deepest branch list there is, puts `main` on row 8 of 30 with 10 rows on screen) — so
            // the row can only ever leave upwards, and an argument for the other edge would be one nothing runs.
            pinEdgeTimer.start()
        } else if (act === "nav-pin-seat") {
            // The stand-in in the seat a fold opened for it, **with no hand on it**: what the line says at rest is
            // the half `nav-open head::<row>` cannot photograph, since that one has to light the stand-in to read
            // what it opens. The argument is `<畳む行>[:<手を置く行>]` — the BRANCHES row to shut, clicked at the
            // row's own click so the fold is a hand's (`NavProbe.clickRow`), and optionally a row to rest on
            // afterwards. **The second one is the seat moving under the stand-in**: a row above the seat that opens
            // pushes the seat down by what it grew, and a stand-in placed by counting whole rows stays where it was.
            const seatParts = ("" + arg).split(":")
            pinSeatTimer.row = seatParts[0] === "" ? 0 : Number(seatParts[0])
            pinSeatTimer.rest = seatParts.length > 1 ? Number(seatParts[1]) : -1
            pinSeatTimer.folded = false
            pinSeatTimer.stood = ""
            pinSeatTimer.start()
        } else if (act === "nav-close") {
            // The pane keeps sections packed against the top; what is read is where the closed header came to rest — at
            // the foot of the pane is the failure this watches for.
            navProbe.closeSection(arg)
            navSectionTimer.start()
        } else if (act === "nav-filter") {
            navProbe.typeFilter(arg)
            navFilterTimer.start()
        } else if (act === "diff-fold" || act === "diff-unfold"
                   || act === "diff-fold-by-hand"
                   || act === "diff-fold-by-rename"
                   || act === "diff-keep-folded") {
            // "-by-hand" brings the list back and so takes the diff down; "-by-rename" is the other side of that rule
            // — a box opened from a peeked row stands in the peek, so neither the fold nor the file goes anywhere.
            // "-keep-folded" had it folded before the diff arrived, so closing the diff leaves it folded.
            page.showWip()
            if (act === "diff-keep-folded")
                page.foldByHand(true)
            page.toggleDiff("unstaged", arg, "")
            if (act === "diff-fold-by-hand")
                page.foldByHand(false)
            else if (act === "diff-fold-by-rename") {
                navProbe.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            } else if (act !== "diff-fold")
                page.closeDiff()
            navRailTimer.start()
        } else {
            return false
        }
        return true
    }
    SampleTimer {
        id: pinEdgeTimer
        onTriggered: {
            // A current branch with no row at all is the other state, and this one cannot read a row that is not in
            // the list (`nav-tip head`).
            if (branchesModel.headRow < 0)
                return
            // Re-applied every tick: the list is handed its height by a layout pass of its own, and one
            // asked for its end before that has nowhere to go. What comes back is where it came to rest, which is
            // also what a repository too small for this says — a section showing every row it holds goes on
            // answering 0 and the row never leaves, so the run waits its watchdog out. That is why the preset is
            // named with the verb (verify-ui スキル).
            const rested = navProbe.scrollBranchesToEnd()
            // **The row leaving is what is waited for.** A stand-in that is up because
            // something else took the row out of the list is a different state wearing the same picture, so the
            // row's own place in the list is asked first and the stand-in only after.
            if (navProbe.rowInView("branch", branchesModel.headRow))
                return
            if (!navProbe.headPinShown || !navProbe.headPinAbove)
                return
            pinEdgeTimer.stop()
            Harness.report(
            "nav_pin_edge pin=" + navProbe.headPinShown
            + " above=" + navProbe.headPinAbove
            + " rowshown=" + navProbe.rowInView("branch", branchesModel.headRow)
            + " name=" + branchesModel.headName
            // Where the layout put it, beside the edge it was asked for: riding above, that is the top of the list
            // whatever the pane's height, so the drawn number can be claimed.
            + " y=" + Math.round(navProbe.headPinY)
            // Last, and the repository's: which row the current branch sorted to, and how
            // far the list had to go to leave it behind.
            + " row=" + branchesModel.headRow
            + " rested=" + Math.round(rested))
            driver.complete()
        }
    }
    SampleTimer {
        id: pinSeatTimer
        property int row: 0
        /// The row the hand comes to rest on once the fold has landed, -1 for none. It goes in at the row's own
        /// pointer stand-in, the same one every other rest in this list uses (`NavProbe.pointTipAt`).
        property int rest: -1
        property bool folded: false
        /// The geometry the last beat read, so a beat reading the same one knows the layout has come to rest —
        /// the seat is a row's height handed out by a layout pass of its own (`AutoActNavBoxVerbs.stood`).
        property string stood: ""
        onTriggered: {
            // **The rows have to be there first**: BRANCHES arrives on a read of its own, and a click aimed at a
            // list still empty folds nothing while answering exactly as an already-folded row would.
            if (!pinSeatTimer.folded) {
                if (!navProbe.clickRow("branch", pinSeatTimer.row))
                    return
                pinSeatTimer.folded = true
                return
            }
            // The row leaving is what is waited for, and the stand-in only after it: a stand-in up for some other
            // reason is a different state wearing the same picture (the reading `nav-pin-edge` makes).
            if (branchesModel.headRow >= 0 || !navProbe.headPinShown)
                return
            if (navProbe.headPinUnder !== pinSeatTimer.row)
                return
            // The hand, once the seat is there — re-applied every beat, since the delegate arrives on a later
            // layout than the rows the model got and a miss reads as a row that answers a rest with nothing.
            if (pinSeatTimer.rest >= 0) {
                navProbe.pointTipAt("branch", pinSeatTimer.rest)
                if (!navProbe.rowFactsOpen)
                    return
            }
            const geom = Math.round(navProbe.headPinY) + "+" + Math.round(navProbe.headPinSeatY())
            if (geom !== pinSeatTimer.stood) {
                pinSeatTimer.stood = geom
                return
            }
            pinSeatTimer.stop()
            Harness.report(
            "nav_pin_seat row=" + pinSeatTimer.row
            + " pin=" + navProbe.headPinShown
            // Whether the row the hand was sent to opened, which is the whole of what moved the seat.
            + " open=" + navProbe.rowFactsOpen
            // **The judged run is one stretch of the line**: which row it sits under, that it is in that row's own
            // seat, that no hand is on it, and the column and the words it takes there.
            //
            // `sat=` is the stand-in's own `y` against the geometry of the row holding the seat — the same number
            // worked out from opposite ends, so a stand-in placed by some other arithmetic says so here.
            + " under=" + navProbe.headPinUnder
            + " sat=" + (Math.round(navProbe.headPinY) === Math.round(navProbe.headPinSeatY()))
            + " lit=" + navProbe.headPinLit
            + " depth=" + branchesModel.headDepth
            + " says=" + branchesModel.headShownName
            // Behind them, for a reader: the two raw coordinates, which edge it would have taken had the seat been
            // off screen, and the whole name it still answers to.
            + " at=" + Math.round(navProbe.headPinY)
            + " gap=" + Math.round(navProbe.headPinSeatY())
            + " above=" + navProbe.headPinAbove
            + " name=" + branchesModel.headName)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=nav-peek-open: rest on a row of the open section and wait for it to open, which is what says the
    /// section stayed up while the rows inside it moved.
    SampleTimer {
        id: peekOpenTimer
        property int want: 0
        onTriggered: {
            // Re-applied every beat: the rows of a popup that has just opened arrive on a later layout, and a miss
            // reads exactly like a row with nothing to open (the reading `nav-tip` makes of the sections' own lists).
            navProbe.pointPeekTipAt(peekOpenTimer.want)
            const name = navProbe.peekNameAt(peekOpenTimer.want)
            if (name === "" || !navProbe.rowFactsOpen)
                return
            peekOpenTimer.stop()
            Harness.report("nav_peek_open row=" + peekOpenTimer.want + " name=" + name
                + " peek=" + navProbe.peekStanding + " open=" + navProbe.rowFactsOpen
                + " says=" + navProbe.rowFactsWords())
            driver.complete()
        }
    }
    // The splitter has to have handed the pane its new width before the width can be reported — the fold sets it, the
    // layout takes it. And the diff these verbs open is a git subprocess away: `diff=true` is the verb's own word
    // that a file is open in the middle, and a pane still reading it frames like one that has, so the run waits for
    // the read to have landed — rows, a picture or a binary notice (`DiffPane.diffSettled`) — before the rail is read
    // and the census walked.
    SampleTimer {
        id: navRailTimer
        onTriggered: {
            if (sidebarPane.width <= 0 || sidebarPane.height <= 0)
                return
            if (page.diffShown && !diffPane.diffSettled())
                return
            navRailTimer.stop()
            Harness.report(
            // The three that are read together: what the centre holds, and what is being typed into. A box opens where
            // its row is, and folded that is the section beside the rail — putting the list back would take an open
            // file down with it (デザイン規約 §左メニューを畳む), so the three are neighbours or they cannot be judged in one
            // substring.
            "nav_rail collapsed=" + page.sidebarCollapsed
            + " diff=" + page.diffShown
            + " editing=" + sidebarPane.editKey
            + " width=" + Math.round(sidebarPane.width)
            + " peek=" + sidebarPane.peekKind
            // Where the open section stands. `cell` is the top edge of the mark that opened it and `top` where the
            // panel begins — they are the same number or the list has walked away from its own cell — the failure a
            // section too tall for the pane invites. `end` against `pane` is the other half: it grows down into the
            // pane and stops at the foot of it.
            + " top=" + Math.round(navProbe.peekY)
            + " cell=" + Math.round(sidebarPane.peekTop)
            + " end=" + Math.round(navProbe.peekBottom)
            + " pane=" + Math.round(sidebarPane.height))
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=doors-held: the branch rebased onto, and the one the right-click then lands on. One away from
    /// the tree's own — `switch` is only offered away from where you already are, and that row is the whole point of
    /// the hold.
    property string heldBranch: ""
    /// The replay's own rising edge, caught where it cannot be missed. **A sampler would miss it**: the write starts
    /// and finishes inside one beat on a repository this size, and what the page then holds up is the state
    /// itself (`RepoPage.autoReplayHeld`).
    Connections {
        target: acts.repoTab
        function onReplayingChanged() {
            if (Harness.autoAct === "doors-held" && acts.repoTab.replaying)
                page.autoReplayHeld = true
        }
    }
    // The doors, tried one by one once the replay is under way. Each is read back: a box that
    // never opened, a double-click that led nowhere and a `+` that cannot be pressed all frame exactly like a pane
    // nobody touched (app-ui.md §UI 自動化の因果性). What the picture is for is the other half — that none of the rest
    // of the pane went out with them.
    /// Whether the doors have been tried; from then on the sampler is waiting for the line the blocked row says why
    /// with, which comes out on the tooltip's own delay (`Metrics.tipDelayMs`) and is the last thing to arrive.
    property bool heldTried: false
    SampleTimer {
        id: navHeldTimer
        onTriggered: {
            if (acts.heldTried) {
                if (!refSwitchItem.ToolTip.visible)
                    return
                navHeldTimer.stop()
                acts.reportHeld()
                renderedBarrier.begin()
                return
            }
            if (!page.autoReplayHeld)
                return
            acts.heldTried = true
            // ---- the graph's two doors, before the pane's ----------------
            // The road every switch on that side arrives by, asked for its own answer: a row's double-click, a chip's
            // and a row of the list a chip had to stack all end here, and `false` is a press this road turned away
            // (`RepoPage.switchToRef` — the same answer `switch-remote-twice` reads).
            acts.heldSwitch = page.switchToRef("L", acts.heldBranch) === false
            // And the menu that row raises. It comes up with its rows kept and greyed, the same as the ref menu's:
            // `drop` is read because it is the one row of this card that cannot be undone.
            page.openRowMenu(workTree.headOid)
            acts.heldRowMenu = commitMenu.opened
            acts.heldDrop = dropCommitItem.blocked
            commitMenu.close()
            // ---- and the pane's ------------------------------------------
            // The box, asked for where a hand asks for it. **The one door whose refusal a picture cannot show** —
            // nothing opening frames exactly like nothing having been asked — so it is read back instead. The
            // double-click's own refusal is the line beside this one in the same object (`SidebarRowGestures.held`),
            // and a run cannot tell a switch that was refused from one that has not landed yet.
            sidebarPane.beginRename("branch", acts.heldBranch, acts.heldBranch)
            // And the menu the rows raise, on the branch whose `switch` is exactly the move being held. Its rows
            // stay and grey (`AppMenu.heldReason`); the line one says why with is in a tooltip, forced here because a
            // pointer cannot be put on a row (verify-ui §hover の絵の撮り方).
            acts.heldMenu = page.openRefMenu("branch", acts.heldBranch, acts.heldBranch,
                                            branchesModel.oidOfName(acts.heldBranch), true)
            acts.heldBox = sidebarPane.editKey !== ""
            refSwitchItem.tipForced = true
        }
    }
    /// What each door did, read at the press — the report waits out the tooltip's delay after
    /// them.
    property bool heldBox: false
    property bool heldMenu: false
    property bool heldSwitch: false
    property bool heldRowMenu: false
    property bool heldDrop: false
    function reportHeld() {
        // **`frozen=` is in the line because the two states frame differently on purpose**: the plan's freeze takes the
        // pane whole and dims it, and this one keeps it — a run that photographed the wrong one of the two would leave
        // a picture nobody could tell apart from the other verb's.
        Harness.report("doors_held held=" + (page.doorsHeldWhy !== "")
                          + " frozen=" + page.sidebarFrozen
                          // The graph's side: the road every switch there arrives by, and the menu its rows raise.
                          + " road=" + acts.heldSwitch
                          + " rowmenu=" + acts.heldRowMenu
                          + " drop=" + acts.heldDrop
                          // The pane's: the box, the `+`, and the menu its rows raise.
                          + " box=" + acts.heldBox
                          + " plus=" + navProbe.headOf("remote").addHeld
                          + " menu=" + acts.heldMenu
                          + " switchrow=" + refSwitchItem.blocked
                          + " why=" + (refSwitchItem.blockedWhy !== ""))
    }
    // PGG_AUTO_ACT=tags-eye: the eye at the end of the TAGS band, and the graph on the other side of it. What the
    // switch moves is the walk, so the commit the named tag stands on is the one thing that answers it — a tag on a
    // commit a branch also reaches keeps both its row and its chip, and a picture of that frames exactly like a
    // picture of a switch that did nothing.
    //
    // **The graph is emptied before it is rebuilt**, so neither the row's absence nor the row count is an edge on its
    // own — a reset shows both a beat after the press, with the walk still to run. `finishCount` is what says a pass
    // landed, and the row is read only after it moves. Turning the tags back on lands twice (a tag-less pass paints
    // first — core.md §2 段ストリーミング), and the row being back is what tells the second pass from the first.
    //
    // The press waits for that same row too, and reads it in the arming step alone: a page is current as soon as the
    // *first* pass finishes, and the first pass is the tag-less one, so a run that pressed at dispatch would be
    // taking the tags out of a graph that never had them in (規約 §前提条件は入力を出す枝で読む).
    property string tagEyeOid: ""
    property int tagEyeFinish: 0
    property bool tagEyeBack: false
    /// 0 = waiting for the tags to be in the graph to take out, 1 = for the row to go, 2 = for it to come back.
    property int tagEyeStep: 0
    SampleTimer {
        id: tagEyeTimer
        onTriggered: {
            const there = graphModel.rowOf(acts.tagEyeOid) >= 0
            if (acts.tagEyeStep === 0) {
                if (!there)
                    return
                acts.tagEyeStep = 1
                acts.tagEyeFinish = graphModel.finishCount
                navProbe.tapTagEye()
                return
            }
            if (graphModel.finishCount === acts.tagEyeFinish)
                return
            if (acts.tagEyeStep === 1) {
                if (there)
                    return
                if (acts.tagEyeBack) {
                    acts.tagEyeStep = 2
                    acts.tagEyeFinish = graphModel.finishCount
                    navProbe.tapTagEye()
                    return
                }
            } else if (!there)
                return
            tagEyeTimer.stop()
            // `there` is the half a picture cannot carry on its own, and `shown=` is the switch's own answer: a run
            // whose press never reached the band photographs the state it started in, which is a real state.
            Harness.report("tags_eye shown=" + repoTab.tagsShown
                              + " there=" + there
                              + " tag=" + Harness.autoActArg
                              + " rows=" + graphModel.rowTotal
                              + " tags=" + tagsModel.total)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-jump: one click on a row of the left panel, and the commit it led to.
    //
    // **The claim is read off the graph.** Where the click sent the page is the click's own
    // bookkeeping, and a wiring that kept the selection while the history stood still would answer it; what says the
    // jump arrived is the graph's row for that commit, lit and laid out (帳簿の外の証人 — `GraphRowDelegate.selected`).
    /// What the row named when it was pressed, and whether the press is in. Read before the click: the delegate is
    /// recycled, and a row asked afterwards can be showing somebody else's name.
    property string jumpWant: ""
    property bool jumpPressed: false
    SampleTimer {
        id: jumpTimer
        /// Which section, and which of its rows — the row travels with the argument (`worktree:1`) and defaults to
        /// the first.
        readonly property string kind: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? arg : arg.substring(0, cut)
        }
        readonly property int row: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? 0 : Number(arg.substring(cut + 1))
        }
        onTriggered: {
            const list = navProbe.listOf(jumpTimer.kind)
            if (!list)
                return
            if (!acts.jumpPressed) {
                // A row the view has not laid out yet is not a row that was pressed, and one that names no commit
                // has nothing for this verb to follow (`NavList.rowOidAt`).
                const oid = list.rowOidAt(jumpTimer.row)
                if (oid === "" || !list.clickRow(jumpTimer.row))
                    return
                acts.jumpWant = oid
                acts.jumpPressed = true
                return
            }
            // The landing: the page reading that commit, the pane on the right having answered for it, and the
            // graph holding a laid-out row of its own to show for it.
            if (page.selectedOid !== acts.jumpWant || !driver.cardSettled)
                return
            const at = graphModel.rowOf(acts.jumpWant)
            // **A commit the walk never reached is an answer.** The page had settled before the click
            // (`AutoActDriver` / `PageSettled` — the walk has finished a pass), so a row that is not there is not
            // going to arrive, and a run that went on waiting for it would spend the watchdog to say what it
            // already knows — and take no picture of the state it found (`screenshot saved=false`). A row the model
            // has but the view has not laid out yet is the other case, and that one is waited for.
            const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
            if (at >= 0 && !item)
                return
            jumpTimer.stop()
            Harness.report("nav_jump section=" + jumpTimer.kind
                              + " row=" + jumpTimer.row
                              // The graph's own answer about the row the click landed on: the commit it carries and
                              // the light it draws. `same=` is what makes `lit=` about this
                              // jump.
                              + " same=" + (!!item && item.oid_hex === acts.jumpWant)
                              + " lit=" + (!!item && item.selected)
                              // The row the click landed on keeps the mark, which is the left panel's half of the
                              // same press (`NavItemDelegate` の chosenBox).
                              + " marked=" + (sidebarPane.activeKey !== "")
                              // The face the graph is on: a jump puts the history up, whatever was being read before
                              // (`RepoPage.jumpToRef`).
                              + " wip=" + page.wipShown
                              + " at=" + at
                              + " name=" + list.rowNameAt(jumpTimer.row))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-reclick / nav-reclick-away: the two clicks of the rename gesture, put in at the rows of the
    // section the folded rail has open, and what those rows made of them. Every step waits for its own answer — the
    // row has to exist before it can be clicked, the double-click window the first click opened has to have passed
    // before a second one counts as a second, and "-away" waits for the section to have actually gone (the pointer
    // leaving is answered a beat later — SectionPeekPopup.settle).
    property bool reclickAway: false
    property bool reclickArmed: false
    property int reclickStep: 0
    SampleTimer {
        id: reclickTimer
        /// Which section, and which of its rows. A section whose names fold into folders has no row to rename at the
        /// top of it, so the row travels with the argument (`branch:1`) and defaults to the first.
        readonly property string kind: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? arg : arg.substring(0, cut)
        }
        readonly property int row: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? 0 : Number(arg.substring(cut + 1))
        }
        // The pointer comes to rest on the cell. A section whose model has no rows yet opens nothing (NavRail.enterAt),
        // so the arrival is made again until one stands.
        function peeked() {
            if (sidebarPane.peekKind !== "")
                return true
            navProbe.peekAt(reclickTimer.kind)
            return false
        }
        onTriggered: {
            const section = sidebarPane.peekSection
            if (acts.reclickStep === 0) {
                if (!reclickTimer.peeked() || !section.clickRow(reclickTimer.row))
                    return
                acts.reclickStep = acts.reclickAway ? 1 : 3
            } else if (acts.reclickStep === 1) {
                navProbe.peekAway(reclickTimer.kind)
                acts.reclickStep = 2
            } else if (acts.reclickStep === 2) {
                // The pointer leaving is answered a beat later, so the section is gone when it says so and not before.
                if (sidebarPane.peekKind !== "")
                    return
                acts.reclickStep = 3
            } else if (acts.reclickStep === 3) {
                if (!reclickTimer.peeked() || section.rowGuarded(reclickTimer.row)
                        || !section.clickRow(reclickTimer.row))
                    return
                // Read where it is set: the wait is short and the box is what it turns into.
                acts.reclickArmed = section.rowArmed(reclickTimer.row)
                acts.reclickStep = 4
            } else if (acts.reclickStep === 4) {
                // Armed, the box opens once that wait runs out — on the row where it stands, which is in the section
                // beside the rail (SidebarPane.startEdit). Unarmed there is nothing further to wait for.
                if (acts.reclickArmed && sidebarPane.editKey === "")
                    return
                reclickTimer.stop()
                Harness.report(
                "nav_reclick section=" + reclickTimer.kind
                + " row=" + reclickTimer.row
                + " marked=" + sidebarPane.activeKey
                + " away=" + acts.reclickAway
                + " armed=" + acts.reclickArmed
                // The fold stays for a box, the box is there, and it has the keyboard — the three that say the
                // gesture landed where the hand was (デザイン規約 §左メニューを畳む).
                + " collapsed=" + page.sidebarCollapsed
                + " box=" + (sidebarPane.editKey !== "")
                + " focused=" + section.rowFocused(reclickTimer.row)
                + " peek=" + sidebarPane.peekKind
                + " editing=" + sidebarPane.editKey)
                driver.complete()
            }
        }
    }
    /// PGG_AUTO_ACT=nav-add-remote: the run stops with the real dialog on screen. `remotes=` is the section the `+` was
    /// pressed on — at 0 the band around it is unavailable, and that the form still came up is the half of this a
    /// picture of an empty section cannot hold either way. `collapsed=` says which of the two doors it came through,
    /// and that the folded one left the list folded (デザイン規約 §左メニューを畳む).
    SampleTimer {
        id: navAddRemoteTimer
        onTriggered: {
            if (!remoteDialog.visible)
                return
            navAddRemoteTimer.stop()
            Harness.report("nav_add_remote dialog=" + remoteDialog.visible
                              + " collapsed=" + page.sidebarCollapsed
                              + " remotes=" + remotesModel.total
                              + " name=" + remoteDialog.wantedName)
            driver.complete()
        }
    }
    // The column has to be laid out again before the header that was closed can say where it ended up.
    SampleTimer {
        id: navSectionTimer
        onTriggered: {
            if (sidebarPane.height <= 0)
                return
            navSectionTimer.stop()
            Harness.report(
            "nav_section closed=" + Harness.autoActArg
            + " header=" + Math.round(
                navProbe.headerTopOf(Harness.autoActArg))
            + " ground=" + Math.round(navProbe.groundTop)
            + " pane=" + Math.round(sidebarPane.height))
            driver.complete()
        }
    }
    /// What each section kept of what it holds. The rows a filter leaves are the ones the sections work out for
    /// themselves, so the counts are read off the models and the picture says what they drew.
    SampleTimer {
        id: navFilterTimer
        onTriggered: {
            if (sidebarPane.width <= 0)
                return
            navFilterTimer.stop()
            Harness.report(
            "nav_filter typed=" + Harness.autoActArg
            + " branches=" + branchesModel.shown() + "/" + branchesModel.total
            + " remotes=" + remotesModel.shown() + "/" + remotesModel.total
            + " tags=" + tagsModel.shown() + "/" + tagsModel.total
            + " stashes=" + stashesModel.shown() + "/" + stashesModel.total)
            driver.complete()
        }
    }
}
