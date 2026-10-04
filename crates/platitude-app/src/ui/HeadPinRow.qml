pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The current branch's sticky stand-in: while its row is scrolled off it rides the edge the row went out of, and it
// steps aside once the row is on screen. A branch a filter or a folded folder hides has no row, so the stand-in takes a
// seat of its own (`seated`, rules-refs/app-ui.md「サイドバーの張り付き行は」). None for a detached HEAD
// (デザイン規約 §左メニューの所作).
//
// Whoever uses this adopts it onto the list itself (`parent:`): a Flickable's declared children scroll with its
// content (rules-refs/app-ui.md「Flickable(ListView 含む)に宣言した子は contentItem に養子入りする」).
Rectangle {
    id: headPin

    required property var branchesModel
    /// The list's scroll position and height, which decide the edge this rides.
    required property real contentY
    required property real viewHeight
    /// The list's margin and fold step (`NavList.rowInset` / `nestStep`), to align the name with the rows'.
    required property int rowInset
    required property int nestStep

    /// The rows' pointer stand-in, for this row (PGG_AUTO_ACT=nav-open head): hover cannot be injected.
    property bool pointed: false

    signal activated(string oidHex)

    /// How far a row opened above the seat pushed it down (`NavList.openRoom` / `openIndex`) — counted in whole rows,
    /// the stand-in would cover the opened lines. 0 in the lists that hand nothing down.
    property real roomAbove: 0
    /// Where the seat begins in the list's content: its row, or the line under the folder closed over it
    /// (`seatedUnder`; the gap is `NavList.pinSeatRow`'s).
    readonly property real rowTop: headPin.seatedUnder
                                   ? (headPin.underRow + 1) * Theme.rowHeight + headPin.roomAbove
                                   : headPin.branchesModel.headRow * Theme.rowHeight
    /// The upstream is gone, read off the model since a stand-in has no row (`models::nav::drain` の
    /// `settle_head_marks`).
    readonly property bool goneUpstream: headPin.branchesModel.headUpstreamGone !== ""
    /// The branch is there but its row is not (a filter or a folded folder). Asks for a seat, which a list with no rows
    /// refuses (its 1px shows). Read off the model alone: the list's height answers to this, and a list with a top
    /// margin rests at a negative `contentY`, where the edges below cannot be asked.
    readonly property bool seated: headPin.branchesModel.headName !== "" && headPin.branchesModel.headRow < 0
    /// The folded row the branch is behind (`models::nav::view` の `folded_over_head`), and whether a fold took its
    /// row. It sits in a gap under that folder and rides an edge only once the gap leaves the view; a filter's half
    /// takes the list's head instead (`NavSections` reads `seated` for `topMargin`).
    readonly property int underRow: headPin.branchesModel.headUnderRow
    readonly property bool seatedUnder: headPin.seated && headPin.underRow >= 0
    readonly property bool rowAbove: (headPin.seated && !headPin.seatedUnder)
                                     || headPin.rowTop < headPin.contentY
    readonly property bool rowBelow: (!headPin.seated || headPin.seatedUnder)
                                     && headPin.rowTop + Theme.rowHeight > headPin.contentY + headPin.viewHeight

    visible: headPin.branchesModel.headName !== ""
             && (headPin.rowAbove || headPin.rowBelow || headPin.seatedUnder)
    // Grows by what it has open, as a row does (デザイン規約 §左メニューの所作): the whole name and the facts.
    height: Theme.rowHeight + headPin.nameOverflow + pinFacts.height
    /// The open stand-in's name field and how far it hangs below the line (as `NameCell.wholeOver`).
    readonly property Item nameField: pinWhole.item
    readonly property real nameOverflow:
        pinWhole.item ? Math.max(0, pinWhole.item.implicitHeight - pinWhole.item.lineHeight) : 0
    y: headPin.rowAbove ? 0
     : headPin.rowBelow ? headPin.viewHeight - height
     : headPin.rowTop - headPin.contentY
    // The list's own ground, opaque: the rows scroll under it.
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
        // Nested as its row is (`headDepth`: `main` at 0, a branch with a `/` one step in); 0 while seated.
        anchors.leftMargin: headPin.rowInset + headPin.branchesModel.headDepth * headPin.nestStep
        // The rows' own bar gutter, so the badges line up with theirs.
        anchors.rightMargin: Theme.navBarGutter
        spacing: Theme.spaceXs
        // The rows' mark slot, left empty so the name starts in their column (`NameCell.seatSize`).
        Item {
            Layout.preferredWidth: Theme.iconXs
            Layout.preferredHeight: Theme.iconXs
            Layout.alignment: Qt.AlignVCenter
        }
        CutName {
            id: pinName
            Layout.fillWidth: true
            // The part of the name its folders do not say (`models::nav::view` の `head_shown`); whole under a filter.
            text: headPin.branchesModel.headShownName
            color: Theme.textLink
            weight: Theme.fontWeightStrong
            pixelSize: Theme.fontMd
            // Swapped for the whole name while open, as the rows do (`NameCell.whole`); a wrap hangs below.
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
                    weight: Theme.fontWeightStrong
                    color: Theme.textLink
                }
            }
        }
        // The row's own listing pair (`drain::settle_head_marks`), not the status read's, which is zeroed right after a
        // switch (rules-refs/app-ui.md「行と同じ listing から」).
        HeadTrack {
            visible: headPin.branchesModel.headAhead > 0 || headPin.branchesModel.headBehind > 0
            ahead: headPin.branchesModel.headAhead
            behind: headPin.branchesModel.headBehind
            Layout.alignment: Qt.AlignVCenter
        }
        GoneBadge {
            visible: headPin.branchesModel.headHasRemote || headPin.branchesModel.headHasPr || headPin.goneUpstream
            pullRequest: headPin.branchesModel.headHasPr
            gone: headPin.goneUpstream
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
            Layout.alignment: Qt.AlignVCenter
        }
    }
    /// The full name, as its row's tip (デザイン規約 §hover のツールチップ); none where the facts open and say it.
    readonly property string tipWords: headPin.opensFacts ? "" : headPin.branchesModel.headName
    /// Out of the list as its rows' (`NavItemDelegate.tipRowSide`).
    readonly property string tipRowSide: "left"
    ToolTip.visible: (headRowMouse.containsMouse || headPin.pointed) && headPin.tipWords !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: headPin.tipWords

    /// Whether it opens facts under itself as a row does, the gestures holding which row is open, and the WORKTREES
    /// section (`NavRowFacts` / `SidebarRowGestures`).
    property bool opensFacts: false
    property var gestures: null
    property var worktreesModel: null
    /// The BRANCHES row's own key (`NavList.keyOf`).
    readonly property string factsKey: "branch:" + headPin.branchesModel.headName
    /// Whether this stand-in is the open one. Being visible is part of it: it shares its row's key, and a hidden
    /// stand-in calling itself open would take the row's sweep into fields that are not there.
    readonly property bool factsOpen: headPin.opensFacts && headPin.visible && headPin.gestures !== null
                                   && headPin.gestures.openKey === headPin.factsKey
    /// What it opens, read as it opens rather than bound (as `NavItemDelegate.gatherFacts`).
    property string factsName: ""
    property string factsHeldBy: ""
    property string factsUpstream: ""
    property bool factsGone: false
    /// The lines to draw (`NavFacts.lines`).
    property var factsLines: []

    /// The press pair every sidebar row answers (`NavItemDelegate.rowPressed`), also called by its facts. Leads where
    /// its row does and nowhere else: no menu, no second gesture.
    function rowPressed(button, modifiers, held) {
        if (button === Qt.LeftButton)
            headPin.activated(headPin.branchesModel.headOid)
    }
    function rowDoubled(button) {
    }
    /// No menu here, so none to keep the lines up for (`NavRowFacts`).
    function factsMenuAsked() {
    }
    /// A facts line pressed, answered as the row's (`NavItemDelegate.followFact`).
    function followFact(to) {
        if (headPin.gestures !== null)
            headPin.gestures.followLine(headPin.factsKey, to.oid)
    }
    function gatherFacts() {
        const branch = headPin.branchesModel.headName
        // The rows' table (`NavFacts`), handed a BRANCHES row's worth of answers; `bucket` is git's own `[gone]`.
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
        // The name always opens: it is the copy that can be dragged away.
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
    /// For the runs (PGG_AUTO_ACT=nav-open `head:…`): what is open, and the lines. Empty / null while closed, so the
    /// sections can ask it first.
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
    /// Whether all of it, facts included, is inside the list (as `NavList.openShown`). Only the seated case can
    /// overflow, but every case is read so a break shows.
    function openShown() {
        return headPin.y >= 0 && headPin.y + headPin.height <= headPin.viewHeight
    }
    // The pointer stand-in opens at once: a run has no hand to rest.
    onPointedChanged: {
        if (headPin.pointed)
            headPin.askFacts(headPin.mapToItem(null, headPin.width / 2, Theme.rowHeight / 2))
        else
            headPin.dropFacts()
    }
    onFactsOpenChanged: if (headPin.factsOpen) headPin.gatherFacts()
    // The rest before the facts open (規約 §hover のツールチップ「補足は待ってから開く」).
    Timer {
        id: pinFactsWait
        interval: Metrics.tipDelayMs
        onTriggered: if (headRowMouse.containsMouse) headPin.askFacts(headRowMouse.mapToItem(null, headRowMouse.mouseX,
                                                                                            headRowMouse.mouseY))
    }
    // The facts, inside the stand-in under its line (`NavRowFacts`); on the bottom edge the whole grows upward.
    Loader {
        id: pinFacts
        active: headPin.factsOpen
        visible: pinFacts.active
        // Seated off the row height, as a row seats its lines (`NavItemDelegate`).
        anchors.top: parent.top
        anchors.topMargin: Theme.rowHeight + headPin.nameOverflow
        // Wider by a gap each side, given back inside (`NavRowFacts.bandReach`).
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

    // Hairline on the side the rows pass under: the top while riding the bottom edge, the foot otherwise.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        y: headPin.rowBelow && !headPin.rowAbove ? 0 : parent.height - height
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }
    /// A press that starts to move goes for the words: the facts open now, not after the rest, and the drag carries
    /// into the name (rules-refs/app-ui.md「待たずに掴む道は行の側の 4 ハンドラ」).
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
        // One line each into the stand-in's own functions, which runs enter
        // (verify-ui implement.md「注入はハンドラ本体そのものへ入れる」). No release handler: `lineDragged` reads `pressed`.
        onPressed: mouse => headPin.linePressed(mouse.x, mouse.y)
        onPositionChanged: mouse => headPin.lineDragged(mouse.x, mouse.y)
        onClicked: headPin.lineClicked()
        // Rest before opening, drop on leaving — the facts are inside the stand-in, so reading them never leaves it.
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
