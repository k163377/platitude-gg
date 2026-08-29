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
/// hangs off that driver; the names it owns are read back once below, so the code under them reads as it
/// did when it was all one file.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so what is written under them reads as it did.
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
    readonly property var refMenu: driver.refMenu
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "nav-rename-far") {
            // A box on a row the list had scrolled away from. The list has to bring it back (デザイン規約 §左メニューの所作)
            // — a name changing itself off screen is a name nobody agreed to. Entered on the folded rail's section
            // because that is the one list a run can scroll and read back through a single handle, and the row is
            // named the way the menu names it (`beginRename`), not by clicking it.
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
            AppBackend.report("nav_drop how=" + arg
                              + " collapsed=" + page.sidebarCollapsed
                              + " box=" + (sidebarPane.editKey !== "")
                              + " editing=" + sidebarPane.editKey)
        } else if (act === "nav-tip") {
            // `<section>:<row>`, or `head` for the current branch's sticky stand-in. The pointer goes in at the row's
            // own `pointedTipRow`, the same one the file lists carry.
            navTipTimer.begin(arg)
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
                return
            }
            sidebarPane.submitEdit(parts[1])
            if (act === "rename-remote-go")
                graphPane.completeHold()
            else
                renameAskTimer.start()
        } else if (act === "nav-branch-box" || act === "nav-rename-box" || act === "nav-tag-box") {
            // The two boxes the left menu opens on a row, left standing instead of submitted — the copy of the chip
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
    // PG_AUTO_ACT=nav-rename-far: the section is opened, scrolled until its first row is out of sight, and only then
    // asked for a box on that row. Each step waits for the one before to have landed — a list still building has no
    // height to scroll by, and a run that named the row before the scroll took would be watching the list stay put.
    property int farStep: 0
    SampleTimer {
        id: farTimer
        onTriggered: {
            const section = sidebarPane.peekSection
            if (acts.farStep === 0) {
                if (sidebarPane.peekKind === "") {
                    sidebarPane.peekAt("tag")
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
                AppBackend.report(
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
    // status pass after the move is what writes it — so a run that never landed waits out the watchdog rather than
    // photographing the tree it started in.
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
            AppBackend.report("rename_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode)
            driver.complete()
        }
    }
    // The sidebar's row tooltips, and the rows that answer with none. Every run lights a control row first — the
    // WORKTREES row always says where it leads — so a run that photographs an empty overlay has said in the same line
    // that the pointer and the shared instance were both working. Without that, "nothing came out" and "nothing was
    // pointed at" are one picture.
    SampleTimer {
        id: navTipTimer
        property string kind: "branch"
        property int row: 0
        property bool head: false
        /// What the stand-in is put on screen by: a filter its own branch does not answer to. No section overflows its
        /// rows in the sidebar proper (`NavList.Layout.maximumHeight`), so scrolling cannot take the current branch's
        /// row off — the two ways it has no row are a filter and a folded folder (`HeadPinRow`).
        property string hide: ""
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
            navTipTimer.start()
        }
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!navTipTimer.lit) {
                // Re-applied every beat: the delegate arrives on a later layout than the rows the model got, and a
                // miss reads exactly like a row that wants no tooltip (`NavList.clickRow`).
                sidebarPane.pointTipAt("worktree", 0)
                if (!tip.visible)
                    return
                navTipTimer.lit = true
                sidebarPane.pointTipAt("worktree", -1)
                // The stand-in stands while its own branch has no row of its own — and the filter that takes that row
                // away would take the control row with it, so it goes in only once the control has answered.
                if (navTipTimer.hide !== "")
                    sidebarPane.typeFilter(navTipTimer.hide)
                if (navTipTimer.head)
                    sidebarPane.headPinPointed = true
                return
            }
            const target = navTipTimer.head ? branchesModel.headRow : navTipTimer.row
            if (!navTipTimer.head)
                sidebarPane.pointTipAt(navTipTimer.kind, target)
            // Nothing is attached to the stand-in, so what is read there is that the pointer is on it and the
            // instance went back down.
            const name = navTipTimer.head ? branchesModel.headName
                       : sidebarPane.tipNameAt(navTipTimer.kind, target)
            if (name === "" || (navTipTimer.head && !sidebarPane.headPinLit))
                return
            const words = navTipTimer.head ? sidebarPane.headPinWords
                        : sidebarPane.tipWordsAt(navTipTimer.kind, target)
            // A row with something to say is not photographed until the instance is up; one with nothing to say is not
            // photographed until the control's own tip has left the screen.
            if ((words !== "") !== tip.visible)
                return
            navTipTimer.stop()
            AppBackend.report("nav_tip section=" + (navTipTimer.head ? "head" : navTipTimer.kind)
                + " row=" + target + " name=" + name
                + " lit=" + navTipTimer.lit + " wants=" + (words !== "")
                + " tip=" + tip.visible + " text=" + (tip.visible ? tip.text : "")
                // Last, and after a text that may carry anything: the judged trio above has to stay one substring.
                + (navTipTimer.head ? " pin=" + sidebarPane.headPinLit : ""))
            driver.complete()
        }
    }
    // PG_AUTO_ACT=nav-branch-box / nav-rename-box: the pane is set to the width the argument names, the ref it names
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
            const list = sidebarPane.listOf(navNameBoxTimer.kind)
            const drawn = list.rowBoxWidth(row)
            if (navNameBoxTimer.step === 1) {
                // On the row, and built: a row the view has not laid out yet answers 0 for its box, the same as a row
                // with no box on it. Whether the box then took the keyboard is reported rather than waited on — a box
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
                // Walked past: what is waited for now is the box going. **Not the box's width** — a delegate the view
                // did let go of answers 0 for everything, and waiting on that number again would wait for ever.
                return
            }
            navNameBoxTimer.stop()
            const whole = list.rowBoxWhole(row)
            AppBackend.report("nav_name_box mode=" + navNameBoxTimer.mode
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
    // PG_AUTO_ACT=rename-box-out: the ways out of the name box, one route per run. **The claim is what the box and the
    // gesture are left holding**, not how long nothing happened for: a wait still running is the whole of how a box
    // comes back by itself, so `armed=false` is what says it will not (2026-08-26 ユーザー報告 — the box on a row clicked
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
                // **The box is opened by the gesture itself, not by the page's own call.** A box put up any other way
                // leaves the gesture with no memory of the row, and every way out then passes for free — which is how
                // the blink survived a green run twice (2026-08-26 ユーザー報告). What the reader did is two clicks, and
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
                    // reproduced what a hand does — which is how the blink survived a green run (2026-08-26 ユーザー報告).
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
                AppBackend.report(
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
