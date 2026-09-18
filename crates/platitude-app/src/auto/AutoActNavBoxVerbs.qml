pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The boxes the sidebar opens on a row — renaming a branch, a tag, a stash or a remote — and the tooltip a row
/// puts out. All of them are one box over one row, and all of them close the same three ways.
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
    readonly property var branchesModel: driver.branchesModel
    readonly property var remotesModel: driver.remotesModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var graphPane: driver.graphPane
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var navProbe: driver.navProbe
    readonly property var refMenu: driver.refMenu
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "nav-rename-far") {
            // A box on a row the list had scrolled away from. The list has to bring it back (デザイン規約 §左メニューの所作)
            // — a name changing itself off screen is a name nobody agreed to. Entered on the folded rail's section
            // because that is the one list a run can scroll and read back through a single handle, and the row is
            // named the way the menu names it (`beginRename`).
            page.foldByHand(true)
            farTimer.start()
        } else if (act === "nav-rename-drop") {
            // The box, and the two ways it is walked away from without a word being typed: "fold" takes the list down
            // to the rail, "away" is the press that landed anywhere else (Main's `FocusRelease` enters here).
            sidebarPane.beginRename("branch", workTree.branch,
                                    workTree.branch)
            if (arg === "fold")
                page.foldByHand(true)
            else
                page.releasePressedAway(null)
            Harness.report("nav_drop how=" + arg
                              + " collapsed=" + page.sidebarCollapsed
                              + " box=" + (sidebarPane.editKey !== "")
                              + " editing=" + sidebarPane.editKey)
        } else if (act === "nav-tip" || act === "nav-open") {
            // `<section>:<row>`, or `head` for the current branch's sticky stand-in. The pointer goes in at the row's
            // own `pointedTipRow`, the same one the file lists carry. `nav-open` is the same walk read on the lines a
            // branch row opens under itself (`NavRowFacts`) instead of on the shared tooltip.
            navTipTimer.opens = act === "nav-open"
            navTipTimer.then = ""
            navTipTimer.begin(arg)
        } else if (act === "nav-open-foot") {
            // The same rest, taken on the last row of a section with more rows than it has height for: the list goes
            // to its end first, so the row the hand comes to rest on has the bottom edge under it and nowhere to
            // open into. **The list is what has to move** (`NavList.revealOpenRow`).
            // With `away` it goes on: the hand leaves, and the list gives back what showing the lines took.
            navTipTimer.opens = true
            navTipTimer.then = arg
            navTipTimer.foot = true
            navTipTimer.begin("branch")
        } else if (act === "nav-open-held") {
            // The other order: the name box first, and **then** a hand on a row. Nothing may open — what opens moves
            // the rows under it, and the box is the only thing on screen saying what mode the reader is in.
            sidebarPane.beginRename("branch", navProbe.tipNameAt("branch", 2),
                                    navProbe.tipNameAt("branch", 2))
            openHeldTimer.start()
        } else if (act === "nav-open-then") {
            // The open row, and then the next thing the hand does — `away` leaves it, `edit` opens the name box on
            // that row, `menu` opens a menu from somewhere else, `filter` types a filter that takes the row out of
            // the list, and `rightclick` / `sweep` / `tap` are the three the hand makes on the open lines themselves.
            // **The orders are what is being read**, not the opening on its own: each is a state it has to answer to.
            navTipTimer.opens = true
            navTipTimer.then = arg
            navTipTimer.begin("branch:2")
        } else if (act === "nav-rename" || act === "rename-branch"
                   || act === "rename-tag" || act === "rename-stash") {
            // Which row: the current branch, the first tag, the first stash. "nav-rename" leaves the box standing for
            // the shot.
            const kind = act === "rename-tag" ? "tag" : act === "rename-stash" ? "stash" : "branch"
            const id = kind === "branch" ? workTree.branch
                     : kind === "tag" ? tagsModel.nameAt(0) : stashesModel.fullAt(0)
            const shown = kind === "stash" ? stashesModel.nameAt(0) : id
            sidebarPane.beginRename(kind, id, shown)
            if (act !== "nav-rename")
                sidebarPane.submitEdit(arg)
        } else if (act === "rename-taken") {
            // A name git will not take, submitted for real: the box stays open holding it, and git's own words go
            // under it as well as into the bar (デザイン規約 §答えの要らない報せ). The argument is the name to type.
            sidebarPane.beginRename("branch", workTree.branch, arg)
            sidebarPane.submitEdit(arg)
            renameTakenTimer.start()
        } else if (act === "rename-tag-box") {
            // The box opened with the argument already typed in it, the way `rename-remote-box` is — so a name the box
            // itself turns down can be photographed being turned down (デザイン規約 §答えの要らない報せ: 押す前に断る側).
            //
            // **The picture cannot judge this**: the answer is the frame's colour and a line that lives in a tooltip,
            // and a box that took the name frames the same as one that would not. `was=` is the name it is being
            // weighed against, so a run that opened the box on the wrong row says so.
            tagNameBoxTimer.begin(arg)
        } else if (act === "rename-remote" || act === "rename-remote-box"
                   || act === "rename-remote-go") {
            // Named outright (`origin/billing:billing-v2`) because the remote's rows are behind a fold. "-box" leaves
            // the box standing, the plain act stops at the question, "-go" holds the pill to the end.
            const parts = arg.split(":")
            const ref = parts[0]
            const was = GitFacts.branchOfRef(ref, repoTab.remoteNames)
            // "-box" opens with the argument already in it, so a name the remote already carries can be photographed
            // being refused — and the remote's fold has to come open for the row to be there at all (a remote root
            // starts closed).
            if (act === "rename-remote-box")
                remotesModel.toggleFolder(GitFacts.remoteOfRef(ref, repoTab.remoteNames))
            sidebarPane.beginRename("remote", ref,
                                    act === "rename-remote-box" ? parts[1] : was)
            if (act === "rename-remote-box") {
                renderedBarrier.begin()
                // A known verb answers true even on its early way out —
                // falsy would send the dispatch on asking every other
                // family about it.
                return true
            }
            sidebarPane.submitEdit(parts[1])
            // The hold's end is the press the write barrier is armed on (`holdToEnd`), said by the pane when the
            // pill confirms.
            if (act === "rename-remote-go")
                driver.holdToEnd(graphPane)
            else
                renameAskTimer.start()
        } else if (act === "nav-branch-box" || act === "nav-rename-box" || act === "nav-tag-box") {
            // The two boxes the left menu opens on a row, left standing — the copy of the chip
            // column's box on the side with no lanes to grow into, and the rename box that shares the field with it.
            // The argument is `<section>:<ref>[:<幅>][:away]`: the width is what a hand would drag the pane's own bar
            // to, since what the box is drawn at is the row's share of it and the indent under a folder comes out of
            // that share; `away` walks the list on past the row afterwards.
            navNameBoxTimer.begin(act === "nav-rename-box" ? "rename"
                                : act === "nav-tag-box" ? "tag" : "branch", arg)
        } else if (act === "name-branch") {
            graphPane.view.namingSubmitted(graphModel.oidAt(0), arg, "branch")
        } else if (act === "rename-box-out") {
            // The argument is the way out; the row is the first one, which every preset with a chip on it can answer.
            boxOutTimer.route = arg
            boxOutTimer.start()
        } else {
            return false
        }
        return true
    }
    // The box has to still be there when the picture is taken, and what proves it is the box's own state:
    // a box that closed and a box that stayed open with a warning frame are two pixels apart. Waited on the
    // refusal arriving, since the words come back with the answer.
    SampleTimer {
        id: renameTakenTimer
        onTriggered: {
            // The bar all the way down as well as the box: the words are written on the answer and the height follows
            // over 200ms, so a picture taken on the refusal alone catches a bar still on its way down
            // (`NoticeBar.settled` — the first run of this verb framed exactly that).
            if (repoTab.busyCount !== 0 || !sidebarPane.editRefused || !page.noticeCard.settled)
                return
            renameTakenTimer.stop()
            Harness.report("rename_taken open=" + (sidebarPane.editKey !== "")
                              + " refused=" + sidebarPane.editRefused
                              + " bar=" + page.noticeCard.open
                              + " tone=" + page.noticeCard.tone
                              + " why=" + sidebarPane.editRefusedWhy)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=rename-tag-box: the box, and whether the reason for turning the name down reached the reader.
    //
    // **Every field on the line is the output side.** `beginRename` is an ask, and `editKey` is that ask written down:
    // the box it names is built by a `Loader` in a row the view has yet to lay out, so a run that read the answer off
    // the key would report a refusal on a field nobody can see or type into. `box=` is the box as drawn and holding
    // the keyboard (`NavList.rowBoxShown` / `rowFocused`), which is the state the refusal is about.
    //
    // **`tip=` and `text=` are the halves nothing else here can answer.** `refused=` is the box's own mark, and a box
    // that refuses in silence carries it just as well; the sentence lives in the shared tooltip, which a binding on
    // the box raises (`NavNameBox`) and Qt can drop without a word — it reads the re-entry as a binding loop. So
    // `tip=` is taken **through the box** (`NavList.rowTipShown`), which is the read that weighs whose tip it is
    // (`tests/qml/tst_tipowner.qml`), and `text=` is the instance saying what is written on it. What the
    // model's reason is worth is that the reader sees it, and `text=` is that same sentence
    // where the reader gets it.
    SampleTimer {
        id: tagNameBoxTimer
        property string typed: ""
        function begin(arg) {
            tagNameBoxTimer.typed = "" + arg
            sidebarPane.beginRename("tag", tagsModel.nameAt(0), tagNameBoxTimer.typed)
            tagNameBoxTimer.start()
        }
        onTriggered: {
            // Asked again every beat: the row is a call, so a binding taken off it would hold the
            // answer the empty model gave (app-ui.md §測って決める値は押し出す).
            const was = tagsModel.nameAt(0)
            const row = tagsModel.rowOfName(was)
            const list = navProbe.listOf("tag")
            const open = row >= 0 && list.rowBoxShown(row) && list.rowFocused(row)
            if (!open)
                return
            // The name the box takes raises nothing, so only the refused arm has a tip to wait for.
            const tipped = list.rowTipShown(row)
            if (sidebarPane.editRefused && !tipped)
                return
            tagNameBoxTimer.stop()
            Harness.report("tag_name_box was=" + was
                              + " typed=" + tagNameBoxTimer.typed
                              + " box=" + open
                              + " refused=" + sidebarPane.editRefused
                              + " tip=" + tipped
                              + " text=" + page.ToolTip.toolTip.text)
            renderedBarrier.begin()
        }
    }
    // PGG_AUTO_ACT=nav-rename-far: the section is opened, scrolled until its first row is out of sight, and only then
    // asked for a box on that row. Each step waits for the one before to have landed — a list still building has no
    // height to scroll by, and a run that named the row before the scroll took would be watching the list stay put.
    property int farStep: 0
    SampleTimer {
        id: farTimer
        onTriggered: {
            const section = sidebarPane.peekSection
            if (acts.farStep === 0) {
                if (sidebarPane.peekKind === "") {
                    navProbe.peekAt("tag")
                    return
                }
                section.scrollToEnd()
                acts.farStep = 1
            } else if (acts.farStep === 1) {
                // The scroll is what puts the row out of sight; until it has, there is nothing to bring back.
                if (section.rowInView(0))
                    return
                sidebarPane.beginRename("tag", tagsModel.nameAt(0),
                                        tagsModel.nameAt(0))
                acts.farStep = 2
            } else if (acts.farStep === 2) {
                if (sidebarPane.editKey === "")
                    return
                farTimer.stop()
                Harness.report(
                "nav_far row=" + tagsModel.nameAt(0)
                + " shown=" + section.rowInView(0)
                + " box=" + (sidebarPane.editKey !== "")
                + " collapsed=" + page.sidebarCollapsed
                + " editing=" + sidebarPane.editKey)
                driver.complete()
            }
        }
    }
    // Where the move came to rest, and what the carry left in the stash list. The branch itself is the edge — the
    // status pass after the move is what writes it — so a run that never landed waits out the
    // watchdog.
    // The rename's own question, waited on for the same settle: a bar photographed before its words arrive is a red
    // line with nothing on it. The plain verb ends here — the write is "-go"'s half.
    SampleTimer {
        id: renameAskTimer
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            renameAskTimer.stop()
            // `code=` being empty is part of the claim: a push and a delete make no one command, so the pill answers
            // in the ordinary voice (規約 §git 用語のコード表記 の 1:1 規則 — the same reading `move_ask` makes).
            Harness.report("rename_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode)
            driver.complete()
        }
    }
    // The other order: a name box standing, and then a hand on a row. Nothing may open, and the row still has to be
    // the one under the hand — a run that never reached it would answer the same way.
    SampleTimer {
        id: openHeldTimer
        onTriggered: {
            // The hand goes in the same place a pointer's does. **The answer is read in the same beat**: the stand-in
            // opens outright where a hand would sit out a rest, so a row that was going to open is open already.
            navProbe.pointTipAt("branch", 2)
            const name = navProbe.tipNameAt("branch", 2)
            if (name === "" || sidebarPane.editKey === "")
                return
            openHeldTimer.stop()
            Harness.report("nav_open_held row=2 name=" + name
                + " box=true open=" + navProbe.rowFactsOpen
                + " lit=" + navProbe.rowWashLit("branch", 2))
            driver.complete()
        }
    }
    // The sidebar's row tooltips, and the rows that answer with none. Every run lights a control row first — the
    // WORKTREES row always says where it leads — so a run that photographs an empty overlay has said in the same line
    // that the pointer and the shared instance were both working. Without that, "nothing came out" and "nothing was
    // pointed at" are one picture. The rows that open under themselves instead walk the same path and are read on
    // what they opened (`opens`).
    SampleTimer {
        id: navTipTimer
        property string kind: "branch"
        property int row: 0
        property bool head: false
        /// Whether the run is about the lines a branch row opens under itself rather than the shared tooltip
        /// (PGG_AUTO_ACT=nav-open), and what the hand does once the row is open (PGG_AUTO_ACT=nav-open-then).
        property bool opens: false
        property string then: ""
        /// Whether that next thing has been done — once, not on every beat, because the beat after it is what the
        /// answer is read from — and the name the hand had rested on when it was.
        property bool acted: false
        property string rested: ""
        /// What the stand-in is put on screen by: a filter its own branch does not answer to. **This is the half with
        /// no row anywhere** (`HeadPinRow.seated`) — the two ways into it are a filter and a folded folder, and the
        /// repositories this argument is run against give every section height for all its rows
        /// (`NavList.Layout.maximumHeight`), so there is nothing here to scroll the row off with. The other half,
        /// where the row is in the list and the list moved out from under it, is `nav-pin-edge`'s — and the one
        /// preset whose BRANCHES does overflow is what that and `nav-open-foot` are run on.
        property string hide: ""
        /// Whether the row rested on is the last of its section, with the list taken to its end first
        /// (PGG_AUTO_ACT=nav-open-foot). Done once: the scroll is the state the rest is taken in, not something
        /// re-applied under a hand that has already arrived.
        property bool foot: false
        property bool footDone: false
        /// Where the list was standing when the hand arrived, remembered out here: what closing has to give back is
        /// held against the run's own note of it rather than against the list's (`NavList.openRestY`).
        property real restY: 0
        /// The geometry the last beat read, so a beat that reads the same one knows the layout has come to rest.
        property string stood: ""
        property bool lit: false
        function begin(arg) {
            const parts = ("" + arg).split(":")
            navTipTimer.head = parts[0] === "head"
            navTipTimer.kind = navTipTimer.head || parts[0] === "" ? "branch" : parts[0]
            // A filter, typed once the control has answered: the way the stand-in's own row is taken out of the list,
            // and the way a leaf under a folded folder is brought into it (a filtered row leaves the tree and stands
            // under its full name — `SidebarFilterRow`).
            navTipTimer.hide = navTipTimer.head ? (parts.length > 1 ? parts[1] : "")
                             : (parts.length > 2 ? parts[2] : "")
            navTipTimer.row = !navTipTimer.head && parts.length > 1 ? Number(parts[1]) : 0
            navTipTimer.lit = false
            navTipTimer.acted = false
            navTipTimer.footDone = false
            navTipTimer.stood = ""
            navTipTimer.start()
        }
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!navTipTimer.lit) {
                // Re-applied every beat: the delegate arrives on a later layout than the rows the model got, and a
                // miss reads exactly like a row that wants no tooltip (`NavList.clickRow`).
                navProbe.pointTipAt("worktree", 0)
                if (!tip.visible)
                    return
                navTipTimer.lit = true
                navProbe.pointTipAt("worktree", -1)
                // The stand-in stands while its own branch has no row of its own — and the filter that takes that row
                // away would take the control row with it, so it goes in only once the control has answered.
                if (navTipTimer.hide !== "")
                    navProbe.typeFilter(navTipTimer.hide)
                if (navTipTimer.head)
                    sidebarPane.headPinPointed = true
                navTipTimer.restY = navProbe.listContentY(navTipTimer.kind)
                return
            }
            // The list to its end, once, before the hand comes to rest: what the rest is taken on is the last row of
            // a section that had to scroll to reach it. A beat is given back so the list is standing still by the
            // time the pointer is on it.
            if (navTipTimer.foot && !navTipTimer.footDone) {
                // **The rows have to be there first.** What proved the tooltip works is a row of the working
                // copies' listing, which is a read of its own and can land before the refs one — so the branches
                // may still be empty here. Taking the last row then takes -1, which is also "the pointer is on no
                // row", and the run rests on nothing and waits out the ceiling (measured: 1 run in 7).
                const list = navProbe.listOf("branch")
                if (!list || list.count <= 0)
                    return
                navProbe.scrollBranchesToEnd()
                navTipTimer.row = list.count - 1
                navTipTimer.footDone = true
                navTipTimer.restY = navProbe.listContentY("branch")
                return
            }
            const target = navTipTimer.head ? branchesModel.headRow : navTipTimer.row
            // **Stopped once the hand has moved on**: re-applying the rest every beat would put the pointer back on
            // the row the run has just walked it off (`nav-open-then away`).
            if (!navTipTimer.head && !navTipTimer.acted)
                navProbe.pointTipAt(navTipTimer.kind, target)
            // Nothing is attached to the stand-in, so what is read there is that the pointer is on it and the
            // instance went back down.
            // Once the hand has moved on, the row it was on may not be in the list any more — a filter takes it out —
            // so what the run says it rested on is the name it read then, not what is there now.
            const name = navTipTimer.acted ? navTipTimer.rested
                       : navTipTimer.head ? branchesModel.headName
                       : navProbe.tipNameAt(navTipTimer.kind, target)
            if (name === "" || (!navTipTimer.acted && navTipTimer.head && !navProbe.headPinLit))
                return
            navTipTimer.rested = name
            const words = navTipTimer.head ? navProbe.headPinWords
                        : navProbe.tipWordsAt(navTipTimer.kind, target)
            // A branch row opens its facts under itself instead of raising the tooltip, so that is what the run waits
            // for and reads (`SidebarPane.rowFactsWords`). Everywhere else: a row with something to say is not
            // photographed until the shared instance is up, and one with nothing to say not until the control's own
            // tip has left the screen.
            //
            // **Nothing is waited out after the hand moves on**: what opens is inside the row, so it goes the moment
            // the pointer leaves and there is no beat between the two — the tick after the order is the answer.
            if (!navTipTimer.acted) {
                if (navTipTimer.opens) {
                    if (!navProbe.rowFactsOpen)
                        return
                    // The row is open: the second half of the run is whatever the hand does next.
                    if (navTipTimer.then !== "") {
                        navTipTimer.acted = true
                        navProbe.afterOpen(navTipTimer.then, navTipTimer.kind, target)
                        return
                    }
                } else if ((words !== "") !== tip.visible) {
                    return
                }
            }
            // **Nothing is read until the layout has stopped moving.** What a row opens is measured on a layout, and
            // the section's own height answers to that measurement in turn — so on the pass the row opened on
            // neither has arrived, and a run reading the geometry there says the lines are out of view of a list
            // that is about to show them (measured: `room=21` against a section still 25 tall). Two beats saying the
            // same thing is the list standing still; what it is standing at is then the answer.
            if (navTipTimer.opens) {
                const geom = sidebarPane.rowFactsGeom()
                if (geom !== navTipTimer.stood) {
                    navTipTimer.stood = geom
                    return
                }
            }
            navTipTimer.stop()
            // The row at the foot of a section that had to scroll to reach it, and — with `away` — the list after
            // the hand left it again. **The list is the subject here**, not the hand, so it has a line of its own:
            // `shown=` is the row weighed against what the list is showing, and `back=` is where the list is standing
            // weighed against where this run saw it standing before the hand arrived.
            if (navTipTimer.foot) {
                const at = navProbe.listContentY(navTipTimer.kind)
                Harness.report("nav_open_foot what=" + navTipTimer.then
                    + " row=" + target + " name=" + name
                    + " open=" + navProbe.rowFactsOpen
                    + " shown=" + navProbe.rowFactsShown()
                    + " back=" + (Math.round(at) === Math.round(navTipTimer.restY))
                    + " rest=" + Math.round(navTipTimer.restY) + " at=" + Math.round(at)
                    + " seat=" + sidebarPane.rowFactsGeom())
                driver.complete()
                return
            }
            if (navTipTimer.then !== "") {
                Harness.report("nav_open_then what=" + navTipTimer.then
                    + " row=" + target + " name=" + name
                    + " open=" + navProbe.rowFactsOpen
                    + " lit=" + navProbe.rowWashLit(navTipTimer.kind, target)
                    + " box=" + (sidebarPane.editKey !== "")
                    + " menu=" + sidebarPane.menuOpen
                    + " rows=" + navProbe.listOf(navTipTimer.kind).count
                    // What the hand on those lines came away with, and where the click landed: a drag takes words and
                    // the row hears nothing, a press that never moved is the row's (`NavRowFacts.handClicked`).
                    + " caret=" + navProbe.factsCaret()
                    + " copied=" + (navProbe.factsTook() !== "")
                    + " clicked=" + (sidebarPane.activeKey !== "")
                    // The two above said as words, which is the repository's business rather than the rule's.
                    + " took=" + navProbe.factsTook()
                    + " active=" + sidebarPane.activeKey)
                driver.complete()
                return
            }
            Harness.report("nav_tip section=" + (navTipTimer.head ? "head" : navTipTimer.kind)
                + " row=" + target + " name=" + name
                + " lit=" + navTipTimer.lit + " wants=" + (words !== "")
                + " tip=" + tip.visible + " open=" + navProbe.rowFactsOpen
                // Whether what opened is inside what the list shows — the whole of the question at the foot, and
                // true everywhere else because there was room under the row to begin with.
                + (navTipTimer.opens ? " shown=" + navProbe.rowFactsShown() : "")
                + " text=" + (tip.visible ? tip.text : "")
                // Last, and after a text that may carry anything: the judged four above have to stay one substring.
                + (navTipTimer.opens ? " seat=" + sidebarPane.rowFactsGeom() : "")
                + (navTipTimer.opens ? " says=" + navProbe.rowFactsWords() : "")
                + (navTipTimer.head ? " pin=" + navProbe.headPinLit : ""))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-branch-box / nav-rename-box: the pane is set to the width the argument names, the ref it names
    // is given a box, and the box is left standing for the shot. One timer for both, because what the shot is about —
    // how wide the box comes out on a row of that depth in a pane of that width — is one question asked of the two
    // things the box is opened for.
    SampleTimer {
        id: navNameBoxTimer
        property string mode: "branch"
        property string kind: "branch"
        property string ref: ""
        /// The width the splitter settles at: the ask held to the pane's own floor, since a run asking for less than
        /// the floor is asking what the floor looks like and the answer to that is the floor.
        property real want: 0
        property int step: 0
        readonly property NavSectionModel section:
            navNameBoxTimer.kind === "tag" ? tagsModel
            : navNameBoxTimer.kind === "remote" ? remotesModel : branchesModel
        /// The run walks the list on past the row once the box is standing, to read the one thing a box drawn outside
        /// the list has to answer for: that it goes when its row does.
        property bool away: false
        function begin(mode, arg) {
            const parts = ("" + arg).split(":")
            navNameBoxTimer.mode = mode
            navNameBoxTimer.away = parts[parts.length - 1] === "away"
            navNameBoxTimer.kind = parts[0]
            navNameBoxTimer.ref = parts[1]
            // Left at whatever the session opened with when no width is named.
            const asked = parts.length > 2 && parts[2] !== "away" ? Number(parts[2]) : NaN
            navNameBoxTimer.want = isNaN(asked) ? sidebarPane.width
                                                  : Math.max(asked, sidebarPane.minOpenWidth)
            if (!isNaN(asked))
                page.setSidebarWidth(asked)
            // A remote's root folder starts closed (`models::nav::tree`), so the rows under it are in no list for a
            // menu to be raised on.
            if (navNameBoxTimer.kind === "remote")
                remotesModel.toggleFolder(
                    GitFacts.remoteOfRef(navNameBoxTimer.ref, repoTab.remoteNames))
            navNameBoxTimer.step = 0
            navNameBoxTimer.start()
        }
        onTriggered: {
            // What is on show: a row behind a closed folder answers -1, and so does one whose section has not been
            // read yet.
            const row = navNameBoxTimer.section.rowOfName(navNameBoxTimer.ref)
            if (navNameBoxTimer.step === 0) {
                // The width first. The box is laid out in what the row has left over, so one opened before the pane
                // has been given its width would settle into a width nobody asked about.
                if (row < 0 || Math.round(sidebarPane.width) !== Math.round(navNameBoxTimer.want))
                    return
                if (navNameBoxTimer.mode === "rename") {
                    // What a second click puts in the box, by the same rule the row itself follows (`NavList`): a
                    // remote branch is typed without the remote it lives on, everything else answers to what it shows.
                    const shown = navNameBoxTimer.kind === "remote"
                        ? GitFacts.branchOfRef(navNameBoxTimer.ref, repoTab.remoteNames)
                        : navNameBoxTimer.ref
                    sidebarPane.beginRename(navNameBoxTimer.kind, navNameBoxTimer.ref, shown)
                    navNameBoxTimer.step = 1
                    return
                }
                const oid = navNameBoxTimer.section.oidOfName(navNameBoxTimer.ref)
                // Through the menu, which is the box's only door on this side — and the door that decides between
                // this box and the graph's (`RepoPage.refMenuInSidebar`). The row shows its last segment and answers
                // to the whole name; the menu wants both, and only the second is what git was given.
                page.openRefMenu(navNameBoxTimer.kind, navNameBoxTimer.ref,
                                 navNameBoxTimer.ref, oid, true)
                if (navNameBoxTimer.mode === "tag")
                    page.startTagAt(oid)
                else
                    page.startBranchAt(oid)
                // Dismissed the way choosing a row dismisses it: the box it leaves behind is the subject, and a menu
                // still standing is drawn over the rows beside it.
                refMenu.close()
                navNameBoxTimer.step = 1
                return
            }
            const key = navNameBoxTimer.kind + ":" + navNameBoxTimer.ref
            const list = navProbe.listOf(navNameBoxTimer.kind)
            const drawn = list.rowBoxWidth(row)
            if (navNameBoxTimer.step === 1) {
                // On the row, and built: a row the view has not laid out yet answers 0 for its box, the same as a row
                // with no box on it. Whether the box then took the keyboard is reported — a box
                // drawn where nothing can be typed is a real state, and one this picture would not tell from the
                // other.
                if (sidebarPane.editKey !== key || drawn <= 0)
                    return
                if (navNameBoxTimer.away) {
                    // Far enough that the row is out of the list, near enough that the view still holds its delegate —
                    // otherwise what went is the row and not the box.
                    list.scrollRows(4)
                    navNameBoxTimer.step = 2
                    return
                }
            } else if (list.rowBoxShown(row)) {
                // Walked past: what is waited for now is the box going. **Read off `shown`** — a delegate the view
                // did let go of answers 0 for everything, and waiting on that number again would wait for ever.
                return
            }
            navNameBoxTimer.stop()
            const whole = list.rowBoxWhole(row)
            Harness.report("nav_name_box mode=" + navNameBoxTimer.mode
                              + " ref=" + navNameBoxTimer.ref
                              + " row=" + row
                              + " pane=" + Math.round(sidebarPane.width)
                              // What the row had for it, what it came out at, and what it would take to read whole:
                              // the seat is where the box used to stop, so the three together say whether this box
                              // needed the room outside the pane and whether it got it.
                              + " seat=" + Math.round(list.rowBoxSeat(row))
                              + " box=" + Math.round(drawn)
                              + " whole=" + Math.ceil(whole)
                              // What the picture is being taken for: the placeholder is all there is to say what the
                              // box is for, and a cut one asks nothing (デザイン規約 §グラフ行のダブルクリック).
                              + " cut=" + (Math.ceil(whole) > Math.round(drawn))
                              + " open=" + (sidebarPane.editKey === key)
                              + " focused=" + list.rowFocused(row)
                              + " shown=" + list.rowBoxShown(row)
                              + " at=" + list.rowBoxAt(row))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=rename-box-out: the ways out of the name box, one route per run. **The claim is what the box and the
    // gesture are left holding**: a wait still running is the whole of how a box
    // comes back by itself, so `armed=false` is what says it will not (observed — the box on a row clicked
    // again closed and reopened a window later).
    property int boxOutStep: 0
    SampleTimer {
        id: boxOutTimer
        /// Which way out this run takes: `same-row` / `other-row` / `escape` / `away` / `typed-away`.
        property string route: ""
        onTriggered: {
            const item = graphPane.view.itemAtIndex(0)
            if (!item)
                return
            if (acts.boxOutStep === 0) {
                // **The box is opened by the gesture itself.** A box put up any other way
                // leaves the gesture with no memory of the row, and every way out then passes for free — which is how
                // the blink survived a green run twice. What the reader did is two clicks, and
                // the second one is what the way out has to be weighed against.
                item.leftClick(0)
                acts.boxOutStep = 1
            } else if (acts.boxOutStep === 1) {
                if (graphPane.view.clickGuarded)
                    return
                item.leftClick(0)
                acts.boxOutStep = 2
            } else if (acts.boxOutStep === 2) {
                if (graphPane.namingOid === "")
                    return
                const route = boxOutTimer.route
                if (route === "same-row" || route === "other-row") {
                    // **A click on a row is a press and then a release, and the two are answered in different
                    // places**: the box holds the caret, so the press reaches `FocusRelease` first and takes the box
                    // down, and only then does the row see the click. A run that put the click in alone never
                    // reproduced what a hand does — which is how the blink survived a green run.
                    page.releasePressedAway(null)
                    const row = route === "same-row" ? item : graphPane.view.itemAtIndex(1)
                    if (!row)
                        return
                    row.leftClick(0)
                } else if (route === "escape") {
                    graphPane.view.namingCancelled()
                } else {
                    if (route === "typed-away") {
                        graphPane.view.namingText = "half-written"
                        item.takeNamingFocus()
                    }
                    page.releasePressedAway(null)
                }
                acts.boxOutStep = 3
            } else if (acts.boxOutStep === 3) {
                boxOutTimer.stop()
                Harness.report(
                "rename_box_out route=" + boxOutTimer.route
                + " box=" + (graphPane.namingOid !== "")
                // A wait left running is how a box that has just been walked away from comes back on its own.
                + " armed=" + graphPane.view.renameWaiting
                + " guarded=" + graphPane.view.clickGuarded
                + " typed=" + graphPane.namingText)
                driver.complete()
            }
        }
    }
}
