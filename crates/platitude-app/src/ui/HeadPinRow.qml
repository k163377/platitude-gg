pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The current branch never leaves the viewport: while its own row is scrolled off, this stand-in rides the edge the row
// went out of, and it steps aside the moment the row itself is on screen — so the sidebar never shows the branch twice.
// A branch a filter or a folded folder hides has no row at all, so there is no edge to ride: the stand-in takes a
// place of its own (`seated`) — the seat a folded folder opens under itself (`seatedUnder`), or the head of the
// list where a filter left no folder to sit under.
//
// **A detached HEAD is said elsewhere.** There is no branch to keep on screen, and the words for one are not
// a name; where HEAD is standing is said by the graph's pin, by the WORKTREES row and by the commit button's own
// wording (デザイン規約 §左メニューの所作).
//
// Whoever uses this has to adopt it onto the list itself (`parent:`): a Flickable's declared
// children are taken by its content item and scroll away with it (app-ui.md).
Rectangle {
    id: headPin

    required property var branchesModel
    /// Where the list is standing and how tall it is — which edge this rides, and whether it is needed at all, is read
    /// off the two.
    required property real contentY
    required property real viewHeight
    /// The list's own margin and fold step (`NavList.rowInset` / `nestStep`), so the name begins in the column the
    /// rows' names do.
    required property int rowInset
    required property int nestStep

    /// The pointer stand-in the rows carry, for the row this one stands
    /// for (PGG_AUTO_ACT=nav-tip head): hover cannot be injected, so what
    /// the pointer would light is written in the same one place the
    /// pointer's own arrival writes.
    property bool pointed: false

    signal activated(string oidHex)

    /// How much a row that opened above the seat has pushed it down — the rows of this list are whole rows except
    /// for the one that is open, and that one is above the seat or it is not (`NavList.openRoom` / `openIndex`).
    /// **Without it the stand-in is drawn over the lines that opened**: the seat moves with the rows and a place
    /// counted in whole rows does not. 0 in the lists that hand nothing down.
    property real roomAbove: 0
    /// Where the seat this stands on begins, in the list's own content: its row, or — with the row folded away —
    /// the line under the folder that closed over it (`seatedUnder`, and the gap `NavList.pinSeatRow` opens there).
    readonly property real rowTop: headPin.seatedUnder
                                   ? (headPin.underRow + 1) * Theme.rowHeight + headPin.roomAbove
                                   : headPin.branchesModel.headRow * Theme.rowHeight
    /// Whether the upstream this branch is measured against is one git cannot reach — the same answer the rows read
    /// off their own slot, read here off the model because a stand-in has no row to read (`models::nav::drain` の
    /// `settle_head_marks`).
    readonly property bool goneUpstream: headPin.branchesModel.headUpstreamGone !== ""
    /// The branch is there but its row is not — a filter or a folded folder is holding it.
    /// **Asks for a seat**: a list with no rows at all keeps its hairline and grants nothing, so this stays true
    /// while the seat is refused and what is drawn is the 1px of it the shut section leaves. Read off the model
    /// alone: the list's own height answers to this, so reading its geometry back would be a loop, and a list with a
    /// top margin rests at a negative `contentY` — the edges below cannot be asked in that state.
    readonly property bool seated: headPin.branchesModel.headName !== "" && headPin.branchesModel.headRow < 0
    /// The folded row the branch is behind, and whether a fold is what took its row (`models::nav::view` の
    /// `folded_over_head`). **Where a folded branch belongs is under the folder that closed on it** — opening that
    /// folder is what brings it back — so the list opens a gap there and this sits in it, scrolling with the rows and
    /// riding an edge only once that gap has left the view. A filter leaves no folder to sit under, and that half
    /// keeps the head of the list: the list begins one row lower for it (`NavSections` reads `seated` for
    /// `topMargin`).
    readonly property int underRow: headPin.branchesModel.headUnderRow
    readonly property bool seatedUnder: headPin.seated && headPin.underRow >= 0
    readonly property bool rowAbove: (headPin.seated && !headPin.seatedUnder)
                                     || headPin.rowTop < headPin.contentY
    readonly property bool rowBelow: (!headPin.seated || headPin.seatedUnder)
                                     && headPin.rowTop + Theme.rowHeight > headPin.contentY + headPin.viewHeight

    visible: headPin.branchesModel.headName !== ""
             && (headPin.rowAbove || headPin.rowBelow || headPin.seatedUnder)
    // **It grows by what it has open under it**, the way a row does (デザイン規約 §左メニューの所作). Riding the
    // bottom edge its foot is what is pinned, so growing takes the top of it upward and the rows it stands over stay
    // where they are. Two things grow it: the name shown whole where one line could not hold it, and the facts.
    height: Theme.rowHeight + headPin.nameOverflow + pinFacts.height
    /// The name field the open stand-in shows, and how far it hangs below the line — the pair the rows carry
    /// (`NavItemDelegate` / `NameCell.wholeOver`, where the reading is spelled out).
    readonly property Item nameField: pinWhole.item
    readonly property real nameOverflow:
        pinWhole.item ? Math.max(0, pinWhole.item.implicitHeight - pinWhole.item.lineHeight) : 0
    // Riding an edge takes the whole of it to that edge; standing in the gap the list opened, it sits where the row
    // it stands for would have sat and scrolls with the rows around it.
    y: headPin.rowAbove ? 0
     : headPin.rowBelow ? headPin.viewHeight - height
     : headPin.rowTop - headPin.contentY
    // Dressed as the row it stands for, down to the margins — which leaves it the list's own ground, since a row of
    // this list carries none of its own (`NavItemDelegate`). **Opaque all the same**: the rows scroll under it, and
    // the hairline at its foot is what says they do.
    color: Theme.bgSurface
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: headRowMouse.containsMouse || headPin.pointed
    }
    RowLayout {
        id: pinLine
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: Theme.rowHeight
        // Where the list's rows begin and the folds the row it stands for is nested by (`rowInset` / `nestStep`).
        // **The fold count comes from that row**
        // (`headDepth`): a branch carrying a `/` sits one step in and `main` sits at none, so a stand-in that always
        // took one step began its name in a column no row was in. While it is seated there is no row to follow and the
        // model answers 0, which is the column the list's own rows begin in.
        anchors.leftMargin: headPin.rowInset + headPin.branchesModel.headDepth * headPin.nestStep
        // The rows' own gutter, so the stand-in's ahead/behind and badge stand in the same column as theirs — the one
        // a pane's own bar takes (`NavItemDelegate` / `PaneScrollBar`).
        anchors.rightMargin: Theme.navBarGutter
        spacing: Theme.spaceXs
        // The rows' mark slot, left empty: the stand-in has no mark of its own, but its name has to begin in the same
        // column as the rows it rides above — so it takes their seat, which is the one a fold arrow and a state mark
        // stand in (`NameCell.seatSize` on a list with no change codes in it — the ink of an `iconSm` mark).
        Item {
            Layout.preferredWidth: Theme.iconXs
            Layout.preferredHeight: Theme.iconXs
            Layout.alignment: Qt.AlignVCenter
        }
        CutName {
            id: pinName
            Layout.fillWidth: true
            // What the rows say: the part of the name the folders standing over it do not
            // (`models::nav::view` の `head_shown`). Whole while there is no fold — the filter's half
            // has flattened every folder away, so nothing on screen says any of it.
            text: headPin.branchesModel.headShownName
            color: Theme.textLink
            weight: Font.DemiBold
            pixelSize: Theme.fontMd
            // Whole, in its own place, while the stand-in is open — the same swap the rows make
            // (`NameCell.whole`), anchored so a name that wraps hangs below rather than moving the columns.
            inked: !headPin.factsOpen
            Loader {
                id: pinWhole
                active: headPin.factsOpen
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: pinWhole.item ? pinWhole.item.implicitHeight : 0
                sourceComponent: CardText {
                    text: headPin.factsName
                    pixelSize: Theme.fontMd
                    weight: Font.DemiBold
                    color: Theme.textLink
                }
            }
        }
        // Dressed as the row it stands for, down to where the pair comes from: the same listing the row draws it
        // out of, read off the snapshot by name (`drain::settle_head_marks`), so the stand-in and the row cannot
        // say different numbers. **The status read's pair waits** (`workTree.ahead`) — it is held back until a
        // status has been read on the branch HEAD is on (`WorkTreeModel.countsSettled`), which is the moment after
        // a switch when this stand-in is the one on screen. The seat is empty wherever there is nothing to count:
        // level with the upstream, or no upstream to measure against.
        HeadTrack {
            visible: headPin.branchesModel.headAhead > 0 || headPin.branchesModel.headBehind > 0
            ahead: headPin.branchesModel.headAhead
            behind: headPin.branchesModel.headBehind
            Layout.alignment: Qt.AlignVCenter
        }
        // The badge the row it stands for wears, out of the part both draw it from (`GoneBadge`).
        GoneBadge {
            visible: headPin.branchesModel.headHasRemote || headPin.branchesModel.headHasPr || headPin.goneUpstream
            pullRequest: headPin.branchesModel.headHasPr
            gone: headPin.goneUpstream
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
            Layout.alignment: Qt.AlignVCenter
        }
    }
    /// The name in full, as the row this one stands for would say it (デザイン規約 §hover のツールチップ). **What opens
    /// under it says it** wherever it opens — a branch answers the same way whether the reader is on its own row or
    /// on this stand-in (デザイン規約 §左メニューの所作: 同じ問いに、置かれた場所で違う答え方をしない).
    readonly property string tipWords: headPin.opensFacts ? "" : headPin.branchesModel.headName
    ToolTip.visible: (headRowMouse.containsMouse || headPin.pointed) && headPin.tipWords !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: headPin.tipWords

    /// Whether this stand-in opens its facts under itself the way a row does, the gestures that hold which row is
    /// open, and the section that knows which working copy has a branch out (`NavRowFacts` / `SidebarRowGestures`).
    /// The current branch is out in this copy, so that last answer is empty here unless another one has it as well,
    /// which git refuses.
    property bool opensFacts: false
    property var gestures: null
    property var worktreesModel: null
    /// The key this stand-in answers to: the BRANCHES row's own (`NavList.keyOf`), since standing in for that row
    /// is the whole of what it does.
    readonly property string factsKey: "branch:" + headPin.branchesModel.headName
    /// Whether this stand-in is the open one. The key is the row's, and **being on screen is part of the
    /// answer**: the row and the stand-in answer to the same key, so a stand-in that called itself open while its
    /// row was the one showing would open lines nobody can see — and a hand reading the row's would be reaching into
    /// them (measured: the sweep came away empty, because a hidden item holds no fields).
    readonly property bool factsOpen: headPin.opensFacts && headPin.visible && headPin.gestures !== null
                                   && headPin.gestures.openKey === headPin.factsKey
    /// What it opens, read as it opens rather than bound (the reading `NavItemDelegate.gatherFacts` makes).
    property string factsName: ""
    property string factsHeldBy: ""
    property string factsUpstream: ""
    property bool factsGone: false
    /// Those answers as the lines to draw (`NavFacts.lines`) — the part below draws what it is handed.
    property var factsLines: []

    /// A press on this stand-in, the pair every row of the sidebar answers (`NavItemDelegate.rowPressed`) — the facts
    /// under it hand a press back the same way a row's do. The stand-in leads where its own row does and nowhere
    /// else: no menu, and no second gesture.
    function rowPressed(button, modifiers, held) {
        if (button === Qt.LeftButton)
            headPin.activated(headPin.branchesModel.headOid)
    }
    function rowDoubled(button) {
    }
    /// The stand-in raises no menu, so there is never one of its own to keep its lines up for (`NavRowFacts`).
    function factsMenuAsked() {
    }
    /// A line of its facts pressed — the same answer a row's gives (`NavItemDelegate.followFact`), the row being the
    /// one it stands for.
    function followFact(to) {
        if (headPin.gestures !== null)
            headPin.gestures.followLine(headPin.factsKey, to.oid)
    }
    function gatherFacts() {
        const branch = headPin.branchesModel.headName
        // **The same table the rows read** (`NavFacts`): the stand-in stands for a BRANCHES row, so it says what
        // that row would — the same question, answered the same way wherever it is asked (デザイン規約
        // §左メニューの所作). What it hands over is a row's worth of answers: the current branch's name, its
        // section, and the state the badge above is drawn from (`headUpstreamGone`, git's own `[gone]`).
        const said = NavFacts.answers({
            "kindHint": "branch",
            "folder": false,
            "name": branch,
            "fullName": branch,
            "bucket": headPin.branchesModel.headUpstreamGone,
            "oid_hex": headPin.branchesModel.headOid,
            "change": "",
            "orig_path": "",
            "ahead": headPin.branchesModel.headAhead,
            "behind": headPin.branchesModel.headBehind,
            "sectionModel": headPin.branchesModel,
            "worktreesModel": headPin.worktreesModel,
            "branchesModel": headPin.branchesModel
        })
        headPin.factsUpstream = said.upstream
        headPin.factsGone = said.gone
        headPin.factsHeldBy = said.heldBy
        headPin.factsLines = NavFacts.lines("branch", said)
        // **The name is always what opens**: it is the copy that can be dragged away, and
        // the line above takes its own off while this is out.
        headPin.factsName = said.name
        return headPin.factsName !== ""
    }
    function askFacts(at) {
        if (headPin.opensFacts && headPin.gestures !== null && headPin.gatherFacts())
            headPin.gestures.openFacts(headPin.factsKey, at)
    }
    function dropFacts() {
        if (headPin.gestures !== null)
            headPin.gestures.closeFacts(headPin.factsKey)
    }
    /// What this stand-in has open under it, and the lines themselves — for the runs alone (PGG_AUTO_ACT=nav-open
    /// `head:…`). Empty and null while it is closed, which is what lets the sections ask it first.
    function openWords() {
        return headPin.factsOpen
            ? headPin.branchesModel.headName + " local= track="
              + headPin.branchesModel.headAhead + "/" + headPin.branchesModel.headBehind
              + " held=" + headPin.factsHeldBy
              + " up=" + headPin.factsUpstream + " gone=" + headPin.factsGone
            : ""
    }
    function openFactsItem() {
        return headPin.factsOpen ? pinFacts.item : null
    }
    /// Whether the whole of it — its own line and what it opened — is inside the list it rides (the answer the rows
    /// give off their own geometry: `NavList.openShown`). While it rides an edge its foot is pinned there, so what
    /// it opens grows the other way and there is nothing to scroll; standing in the gap a fold opened it can grow
    /// past the bottom edge, which is what these numbers say. Read all the same in the pinned half, because a rule
    /// nobody reads back is a rule nobody can see break.
    function openShown() {
        return headPin.y >= 0 && headPin.y + headPin.height <= headPin.viewHeight
    }
    // The pointer's stand-in opens what the hand does, with no rest to sit out — a run has no hand to rest
    // (verify-ui スキル).
    onPointedChanged: {
        if (headPin.pointed)
            headPin.askFacts(headPin.mapToItem(null, headPin.width / 2, Theme.rowHeight / 2))
        else
            headPin.dropFacts()
    }
    onFactsOpenChanged: if (headPin.factsOpen) headPin.gatherFacts()
    // The rest a hand sits out before this opens — the same one every other hover that adds a fact asks for
    // (規約 §hover のツールチップ「補足は待ってから開く」).
    Timer {
        id: pinFactsWait
        interval: Metrics.tipDelayMs
        onTriggered: if (headRowMouse.containsMouse) headPin.askFacts(headRowMouse.mapToItem(null, headRowMouse.mouseX,
                                                                                            headRowMouse.mouseY))
    }
    // The facts, under this stand-in's own line and inside it (`NavRowFacts`). Riding the bottom edge the stand-in
    // keeps its foot there, so what grows goes upward and the line stays above what it opened.
    Loader {
        id: pinFacts
        active: headPin.factsOpen
        visible: pinFacts.active
        // The same seat a row gives its own lines, measured off the row's height rather than off the layout above it
        // (`NavItemDelegate`).
        anchors.top: parent.top
        anchors.topMargin: Theme.rowHeight + headPin.nameOverflow
        // A gap wider either side, which the lines give back inside (`NavRowFacts.bandReach`, the row's own reading).
        anchors.left: parent.left
        anchors.leftMargin: headPin.rowInset + headPin.branchesModel.headDepth * headPin.nestStep - Theme.spaceXs
        anchors.right: parent.right
        anchors.rightMargin: Theme.navBarGutter - Theme.spaceXs
        height: pinFacts.item ? pinFacts.item.implicitHeight : 0
        sourceComponent: NavRowFacts {
            row: headPin
            lines: headPin.factsLines
        }
    }

    // Hairline on the side the scrolled rows pass under — its head only while it rides the bottom edge, and its foot
    // everywhere else: seated, and standing in the gap a fold opened, what is under it is the next row down.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        y: headPin.rowBelow && !headPin.rowAbove ? 0 : parent.height - height
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }
    /// A press that starts to move is a reader going for the words: what is open comes out **now** rather than
    /// after the rest, and the drag carries on into it (the pair `NavItemDelegate`
    /// carries, and the one hand over this whole stand-in, its open lines included).
    function linePressed(x, y) {
        headRowMouse.pressFrom = Qt.point(x, y)
        headRowMouse.handedOn = false
        if (pinFacts.item !== null)
            pinFacts.item.dropSweep()
        if (headPin.nameField !== null)
            headPin.nameField.deselect()
    }
    function lineDragged(x, y) {
        if (!headPin.opensFacts || !headRowMouse.pressed)
            return
        if (!headRowMouse.handedOn
                && Math.abs(x - headRowMouse.pressFrom.x) < Qt.styleHints.startDragDistance
                && Math.abs(y - headRowMouse.pressFrom.y) < Qt.styleHints.startDragDistance)
            return
        if (!headRowMouse.handedOn) {
            pinFactsWait.stop()
            if (!headPin.factsOpen)
                headPin.askFacts(headPin.mapToItem(null, x, y))
            if (headPin.nameField === null)
                return
            headPin.nameField.anchorFrom(headPin, headRowMouse.pressFrom.x, headRowMouse.pressFrom.y)
            headRowMouse.handedOn = true
        }
        headPin.nameField.extendFrom(headPin, x, y)
    }
    /// A drag that took the name away is not a click — the reader was copying (`NavRowFacts.handClicked`).
    function lineClicked() {
        if (headPin.nameField !== null && headPin.nameField.selected !== "")
            return
        headPin.rowPressed(Qt.LeftButton, Qt.NoModifier, 0)
    }
    MouseArea {
        id: headRowMouse
        anchors.fill: parent
        hoverEnabled: true
        enabled: headPin.branchesModel.headOid !== ""
        /// Where the button went down, and whether this press has already been handed on to the lines below.
        property point pressFrom: Qt.point(0, 0)
        property bool handedOn: false
        // **Each handler is one line into the stand-in's own** — a run enters those, so what it drives is this
        // wiring rather than a copy of it (verify-ui スキル §注入はハンドラ本体そのものへ入れる).
        // Nothing to undo at the release: what says the press is still down is the `MouseArea`'s own `pressed`,
        // which `lineDragged` reads (the rows keep a flag because their hand is the row's, not a handler's).
        onPressed: mouse => headPin.linePressed(mouse.x, mouse.y)
        onPositionChanged: mouse => headPin.lineDragged(mouse.x, mouse.y)
        onClicked: headPin.lineClicked()
        // The hand sits out the rest here too, and what it opened goes the moment it leaves — the facts are inside
        // this stand-in, so the hand reading them never leaves it (規約 §hover のツールチップ).
        onContainsMouseChanged: {
            if (headRowMouse.containsMouse) {
                pinFactsWait.restart()
            } else {
                pinFactsWait.stop()
                headPin.dropFacts()
            }
        }
    }
}
