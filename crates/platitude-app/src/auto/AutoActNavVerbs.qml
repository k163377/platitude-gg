pragma ComponentBehavior: Bound

import QtQuick
// For the attached `ToolTip` alone (rules-refs/app-ui.md「attached 型は宣言元モジュールを import しないと」).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The sidebar's own verbs: folding and peeking at a section, the second click on a row that is already lit,
/// filtering, closing, and the rows that answer for a tag.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

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

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
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
            // into the opened list, "-out" on out the far side, "-shut" clicks the cell; only "-into" leaves the
            // section standing. "nav-peek" on an empty section stays shut (NavRail.enterAt).
            //
            // The section goes a beat after the pointer leaves (`HoverCardHost`, `Metrics.hoverKeepMs`), so the three
            // whose subject is its going wait for it — finished on the rail's answer, the run frames it standing.
            acts.peekGoneWanted = act === "nav-peek-away" || act === "nav-peek-out" || act === "nav-peek-shut"
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
                // The box lands on the peeked row and holds the section open — the list is not put back for it
                // (SidebarRowGestures.startEdit).
                navProbe.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            }
            navRailTimer.start()
        } else if (act === "nav-peek-open") {
            // A row opened inside the peeked section: what is read is that the popup makes room and stays up while
            // the rows under it move.
            page.foldByHand(true)
            navProbe.peekAt("branch")
            peekOpenTimer.want = arg === "" ? 0 : Number(arg)
            peekOpenTimer.start()
        } else if (act === "tags-eye") {
            // The eye pressed at the TAGS band. The argument names the tag whose commit is the subject; `<tag>:back`
            // presses again so the round trip is read whole.
            const backing = arg.endsWith(":back")
            const tagName = backing ? arg.substring(0, arg.length - ":back".length) : arg
            acts.tagEyeOid = tagsModel.oidOfName(tagName)
            acts.tagEyeBack = backing
            acts.tagEyeStep = 0
            tagEyeTimer.start()
        } else if (act === "nav-jump") {
            // One click on a row of the left panel (デザイン規約 §左メニューの所作), put in at the row's own `leftClick`
            // (`NavList.clickRow`) so the routing tested is the delegate's.
            jumpTimer.start()
        } else if (act === "nav-reclick" || act === "nav-reclick-away") {
            // The rename gesture (two clicks on one row) in the peeked section. "-away" lets the pointer leave in
            // between, and a list that went and came back reads the second as a first click (デザイン規約 §左メニューの所作).
            page.foldByHand(true)
            acts.reclickAway = act === "nav-reclick-away"
            acts.reclickArmed = false
            acts.reclickStep = 0
            reclickTimer.start()
        } else if (act === "nav-add-remote") {
            // The `+` of the REMOTES band, live even while the band is unavailable (デザイン規約 §左メニューの所作), put
            // in at the band's own signal. "folded" is the other door: the rail's REMOTES cell, which at zero answers
            // its click with the dialog, put in at `NavRail.tapAt` (`peekTap`) so the routing tested is the cell's.
            if (arg === "folded") {
                page.foldByHand(true)
                navProbe.peekTap("remote")
            } else {
                navProbe.tapAddRemote()
            }
            navAddRemoteTimer.start()
        } else if (act === "doors-held") {
            // Every door onto the history (left pane and graph) while a replaying write runs; nothing freezes or dims
            // (`SidebarPane.doorsHeld`). The rebase is real — the argument is the branch rebased onto and the menus'
            // subject — and its rise is latched below.
            acts.heldBranch = arg
            repoTab.rebase(arg, "", true)
            navHeldTimer.start()
        } else if (act === "nav-pin-edge") {
            // The current branch's stand-in riding the edge its row scrolled out of (`HeadPinRow`): the list is taken
            // to its end so the row leaves above. The other way the stand-in shows (`nav-open head:<filter>`) has no
            // row to ride, so only the scroll reads this. No preset can make the lower edge: each checks out a
            // name that sorts early.
            pinEdgeTimer.start()
        } else if (act === "nav-pin-seat") {
            // The stand-in in the seat a fold opened for it, with no hand on it (`nav-open head::<row>` has to light
            // it). The argument is `<畳む行>[:<手を置く行>]`: the BRANCHES row shut at its own click (`NavProbe.clickRow`),
            // and optionally a row to rest on, whose opening pushes the seat down — a stand-in placed by counting
            // whole rows would stay put.
            const seatParts = ("" + arg).split(":")
            pinSeatTimer.row = seatParts[0] === "" ? 0 : Number(seatParts[0])
            pinSeatTimer.rest = seatParts.length > 1 ? Number(seatParts[1]) : -1
            pinSeatTimer.folded = false
            pinSeatTimer.stood = ""
            pinSeatTimer.start()
        } else if (act === "nav-close") {
            // Sections stay packed against the top; a closed header resting at the pane's foot is the failure.
            navProbe.closeSection(arg)
            navSectionTimer.start()
        } else if (act === "nav-filter") {
            navProbe.typeFilter(arg)
            navFilterTimer.start()
        } else if (act === "diff-fold" || act === "diff-unfold"
                   || act === "diff-fold-by-hand"
                   || act === "diff-fold-by-rename"
                   || act === "diff-keep-folded") {
            // "-by-hand" brings the list back and so takes the diff down; "-by-rename" opens a box in the peek, so
            // neither the fold nor the file goes; "-keep-folded" folds first, so closing the diff leaves it folded.
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
            // None of these is about a section going: "-by-rename" opens one and holds it open for the box.
            acts.peekGoneWanted = false
            navRailTimer.start()
        } else {
            return false
        }
        return true
    }
    SampleTimer {
        id: pinEdgeTimer
        onTriggered: {
            // No row at all is the other state (`nav-open head:<filter>`).
            if (branchesModel.headRow < 0)
                return
            // Re-applied every tick: a list asked for its end before its layout pass has nowhere to go. A section
            // showing every row keeps answering 0 and the run waits out its watchdog — hence the preset in verbs.md.
            const rested = navProbe.scrollBranchesToEnd()
            // The row leaving first, then the stand-in: one up because the row left the list some other way is a
            // different state wearing the same picture.
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
            // Riding above, that is the list's top whatever the pane's height, so the number can be claimed.
            + " y=" + Math.round(navProbe.headPinY)
            // Last: the repository's own numbers, outside the judgement.
            + " row=" + branchesModel.headRow
            + " rested=" + Math.round(rested))
            driver.complete()
        }
    }
    SampleTimer {
        id: pinSeatTimer
        property int row: 0
        /// The row the hand rests on once the fold has landed, -1 for none (`NavProbe.pointTipAt`).
        property int rest: -1
        property bool folded: false
        /// The geometry the last beat read: two beats alike say the layout pass handing out the seat has settled.
        property string stood: ""
        onTriggered: {
            // BRANCHES arrives on a read of its own, and a click on a still-empty list folds nothing.
            if (!pinSeatTimer.folded) {
                if (!navProbe.clickRow("branch", pinSeatTimer.row))
                    return
                pinSeatTimer.folded = true
                return
            }
            // The row leaving first, then the stand-in (as in `nav-pin-edge`).
            if (branchesModel.headRow >= 0 || !navProbe.headPinShown)
                return
            if (navProbe.headPinUnder !== pinSeatTimer.row)
                return
            // Re-applied every beat: the delegate arrives a layout after the model's rows, and a miss reads as a row
            // with nothing to open.
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
            // The rested row opened — what moved the seat.
            + " open=" + navProbe.rowFactsOpen
            // The judged fields are one stretch (`under=` .. `says=`). `sat=` compares the stand-in's own `y` with the
            // seat row's geometry — one number worked out from both ends.
            + " under=" + navProbe.headPinUnder
            + " sat=" + (Math.round(navProbe.headPinY) === Math.round(navProbe.headPinSeatY()))
            + " lit=" + navProbe.headPinLit
            + " depth=" + branchesModel.headDepth
            + " says=" + branchesModel.headShownName
            // Unjudged, for a reader.
            + " at=" + Math.round(navProbe.headPinY)
            + " gap=" + Math.round(navProbe.headPinSeatY())
            + " above=" + navProbe.headPinAbove
            + " name=" + branchesModel.headName)
            driver.complete()
        }
    }
    SampleTimer {
        id: peekOpenTimer
        property int want: 0
        onTriggered: {
            // Re-applied every beat, as the rest in `pinSeatTimer` is.
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
    /// Whether this act's subject is the peeked section going away. `run()` writes it on every road to the beat, so a
    /// later act never waits on an earlier one's answer.
    property bool peekGoneWanted: false
    // Three waits: the splitter handing the pane its width; the diff's read having landed (`DiffPane.diffSettled`),
    // since a pane still reading frames like one that has; and, for `peekGoneWanted`, the section having gone.
    SampleTimer {
        id: navRailTimer
        onTriggered: {
            if (sidebarPane.width <= 0 || sidebarPane.height <= 0)
                return
            if (page.diffShown && !diffPane.diffSettled())
                return
            // The popup itself, not `kind`: that clears as the exit begins (`SectionPeekPopup.onAboutToHide`).
            if (acts.peekGoneWanted && sidebarPane.peekSection.visible)
                return
            navRailTimer.stop()
            Harness.report(
            // The first three are neighbours so one `must_say` substring judges them (デザイン規約 §左メニューを畳む).
            "nav_rail collapsed=" + page.sidebarCollapsed
            + " diff=" + page.diffShown
            + " editing=" + sidebarPane.editKey
            + " width=" + Math.round(sidebarPane.width)
            + " peek=" + sidebarPane.peekKind
            // `top` equals `cell` (the opening mark's top edge) or the list walked off its cell; `end` <= `pane` says
            // it grew down and stopped at the pane's foot.
            + " top=" + Math.round(navProbe.peekY)
            + " cell=" + Math.round(sidebarPane.peekTop)
            + " end=" + Math.round(navProbe.peekBottom)
            + " pane=" + Math.round(sidebarPane.height))
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=doors-held: the branch rebased onto and right-clicked. Not the current one — `switch` is only
    /// offered elsewhere, and that row is the point of the hold.
    property string heldBranch: ""
    /// The replay's rising edge, latched (`RepoPage.autoReplayHeld`): on a demo repository the write starts and ends
    /// inside one sampler beat.
    Connections {
        target: acts.repoTab
        function onReplayingChanged() {
            if (Harness.autoAct === "doors-held" && acts.repoTab.replaying)
                page.autoReplayHeld = true
        }
    }
    // The doors, tried once the replay is under way. Each is read back, since a refused door frames like an untouched
    // pane; the picture is for the rest of the pane staying live.
    /// Whether the doors have been tried; after that the sampler waits for the blocked row's tooltip, the last thing
    /// to arrive (`Metrics.tipDelayMs`).
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
            // Every switch on the graph side ends at `RepoPage.switchToRef`; `false` is a press it turned away.
            acts.heldSwitch = page.switchToRef("branch", acts.heldBranch) === false
            // The row menu keeps its rows, greyed; `drop` is read as the one row that cannot be undone.
            page.openRowMenu(workTree.headOid)
            acts.heldRowMenu = commitMenu.opened
            acts.heldDrop = dropCommitItem.blocked
            commitMenu.close()
            // ---- and the pane's ------------------------------------------
            // The box, asked for where a hand asks. The double-click's refusal shares its gate
            // (`SidebarRowGestures.held`) and goes unreported: a run cannot tell a refused switch from one not landed.
            sidebarPane.beginRename("branch", acts.heldBranch, acts.heldBranch)
            // The ref menu's rows stay and grey (`AppMenu.heldReason`); the reason is a tooltip, forced since a pointer
            // cannot be put on a row (verify-ui §hover の絵の撮り方).
            acts.heldMenu = page.openRefMenu("branch", acts.heldBranch, acts.heldBranch,
                                            branchesModel.oidOfName(acts.heldBranch), true)
            acts.heldBox = sidebarPane.editKey !== ""
            refSwitchItem.tipForced = true
        }
    }
    /// What each door did, read at the press.
    property bool heldBox: false
    property bool heldMenu: false
    property bool heldSwitch: false
    property bool heldRowMenu: false
    property bool heldDrop: false
    function reportHeld() {
        // `frozen=`: a run that caught the plan's whole-pane freeze instead would pass for that verb's picture.
        Harness.report("doors_held held=" + (page.doorsHeldWhy !== "")
                          + " frozen=" + page.sidebarFrozen
                          // The graph's side.
                          + " road=" + acts.heldSwitch
                          + " rowmenu=" + acts.heldRowMenu
                          + " drop=" + acts.heldDrop
                          // The pane's.
                          + " box=" + acts.heldBox
                          + " plus=" + navProbe.headOf("remote").addHeld
                          + " menu=" + acts.heldMenu
                          + " switchrow=" + refSwitchItem.blocked
                          + " why=" + (refSwitchItem.blockedWhy !== ""))
    }
    // PGG_AUTO_ACT=tags-eye: the eye moves the walk, so the answer is the named tag's commit's row (a tag whose
    // commit a branch also reaches keeps its row, and frames like a switch that did nothing).
    //
    // The graph is emptied before it is rebuilt, so the row is read only after `finishCount` moves. Tags back on
    // lands twice, tag-less first (rules-refs/core.md「2 段ストリーミング」), and the row being back tells the second
    // pass. The press waits for the row too, in step 0 alone: the page is current after the tag-less pass
    // (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」).
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
            // `shown=` is the switch's own answer: a press that never reached the band photographs a real state.
            Harness.report("tags_eye shown=" + repoTab.tagsShown
                              + " there=" + there
                              + " tag=" + Harness.autoActArg
                              + " rows=" + graphModel.rowTotal
                              + " tags=" + tagsModel.total)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-jump: the claim is read off the graph's row for the commit, lit and laid out
    // (`GraphRowDelegate.selected`) — the page's selection is the click's own bookkeeping.
    /// What the row named when pressed, and whether the press is in. Read before the click: a recycled delegate
    /// asked afterwards can show another row's name.
    property string jumpWant: ""
    property bool jumpPressed: false
    SampleTimer {
        id: jumpTimer
        /// The argument `<section>[:<row>]`, the row 0 by default.
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
                // A row not laid out yet, or naming no commit, is not a press (`NavList.rowOidAt`).
                const oid = list.rowOidAt(jumpTimer.row)
                if (oid === "" || !list.clickRow(jumpTimer.row))
                    return
                acts.jumpWant = oid
                acts.jumpPressed = true
                return
            }
            if (page.selectedOid !== acts.jumpWant || !driver.cardSettled)
                return
            const at = graphModel.rowOf(acts.jumpWant)
            // A commit the walk never reached is an answer: the page had settled before the click (`PageSettled`), so
            // the row will not arrive. A row the model has but the view has not laid out yet is waited for.
            const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
            if (at >= 0 && !item)
                return
            jumpTimer.stop()
            Harness.report("nav_jump section=" + jumpTimer.kind
                              + " row=" + jumpTimer.row
                              // `same=` makes `lit=` about this jump.
                              + " same=" + (!!item && item.oid_hex === acts.jumpWant)
                              + " lit=" + (!!item && item.selected)
                              // The left panel's half: the clicked row keeps the mark (`NavItemDelegate` の chosenBox).
                              + " marked=" + (sidebarPane.activeKey !== "")
                              // A jump puts the history up whatever was being read (`RepoPage.jumpToRef`).
                              + " wip=" + page.wipShown
                              + " at=" + at
                              + " name=" + list.rowNameAt(jumpTimer.row))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-reclick / nav-reclick-away: every step waits for its own answer — the row existing, the first
    // click's double-click window passing (`rowGuarded`), and for "-away" the section having gone.
    property bool reclickAway: false
    property bool reclickArmed: false
    property int reclickStep: 0
    SampleTimer {
        id: reclickTimer
        /// The argument `<section>[:<row>]`, the row 0 by default — a section folding names into folders has no
        /// row to rename at its top.
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
        // A section with no rows yet opens nothing (NavRail.enterAt), so the arrival is repeated until one stands.
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
                // The pointer leaving is answered a beat later (SectionPeekPopup.settle).
                if (sidebarPane.peekKind !== "")
                    return
                acts.reclickStep = 3
            } else if (acts.reclickStep === 3) {
                if (!reclickTimer.peeked() || section.rowGuarded(reclickTimer.row)
                        || !section.clickRow(reclickTimer.row))
                    return
                // Read here: the wait is short and turns into the box.
                acts.reclickArmed = section.rowArmed(reclickTimer.row)
                acts.reclickStep = 4
            } else if (acts.reclickStep === 4) {
                // Armed, the box opens in the peeked section once that wait runs out (SidebarRowGestures.startEdit).
                if (acts.reclickArmed && sidebarPane.editKey === "")
                    return
                reclickTimer.stop()
                Harness.report(
                "nav_reclick section=" + reclickTimer.kind
                + " row=" + reclickTimer.row
                + " marked=" + sidebarPane.activeKey
                + " away=" + acts.reclickAway
                + " armed=" + acts.reclickArmed
                // The fold stays for a box, the box is there, and it has the keyboard (デザイン規約 §左メニューを畳む).
                + " collapsed=" + page.sidebarCollapsed
                + " box=" + (sidebarPane.editKey !== "")
                + " focused=" + section.rowFocused(reclickTimer.row)
                + " peek=" + sidebarPane.peekKind
                + " editing=" + sidebarPane.editKey)
                driver.complete()
            }
        }
    }
    /// PGG_AUTO_ACT=nav-add-remote: the run stops with the real dialog on screen. `collapsed=` says which door it came
    /// through, and that the folded one left the list folded (デザイン規約 §左メニューを畳む).
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
    /// What each section's model kept under the filter; the picture says what was drawn.
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
