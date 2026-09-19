pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One sidebar row: ref / file / folder, shared by every section and the WIP file list. What a click means is the
// owner's business — the row only reports it.
Item {
    id: navRow
    required property int index
    required property string name
    required property string full
    required property string oid_hex
    required property string change
    required property string bucket
    required property string orig_path
    /// The same source written the way this row writes names — what the row shows. `orig_path` stays whole beside it:
    /// that one addresses a diff (`models::nav`).
    required property string orig_name
    required property bool is_head
    required property bool has_remote
    required property bool only_remote
    required property bool has_pr
    required property int depth
    required property bool folder
    required property bool eol_mark
    /// The path the line-ending card is out for, handed down by the pane — the model keeps the words once, and the
    /// card's own close is what clears it (`WipPane`). A row reads it to know the card standing
    /// is **its** card (`eolCardOut`).
    property string pointedEolPath: ""
    /// How far a local branch stands from its upstream, as of the last fetch. Zero on every other kind of row, and on
    /// a branch that is level with its upstream or has none — the row draws the arrows itself (`HeadTrack`).
    required property int ahead
    required property int behind
    /// The pointer arrived at, or left, a row carrying the mark.
    signal eolPointed(string path, bool on)
    /// Stands in for the pointer where headless cannot put one, so a cut-down row's tooltip can be photographed
    /// (PGG_AUTO_ACT=path-tip). -1 points at no row.
    property int pointedTipRow: -1
    /// Whether the pointer is on this row. Written by the handler below: hover goes to the topmost item that takes
    /// it, and the stage `+` is a `Control` that takes its own, so the row stopped being "under the hand" exactly
    /// when the hand arrived at the mark (rules-refs/app-ui.md 「行の hover は `HoverHandler`」; measured
    /// qmltestrunner: `containsMouse=false` with the pointer in the middle of the `+`, which took the row's wash out
    /// from under the hand reaching for it and closed its line-ending card).
    property bool pointed: false
    /// A right-click menu of the page's is standing over this list.
    property bool menuStanding: false
    property string kindHint: "branch"
    /// The remote this repository sends pushes to, empty where none is marked (`RepoTab.pushDefault`). Handed down:
    /// one answer for the whole list, and a delegate is recycled row to row.
    property string markedRemote: ""
    /// The configured remote names, so a folder row can tell whether it stands for a remote or only for the shape of
    /// the names below it. Both exist in this section: a remote called `my/fork` puts a plain `my` folder above its
    /// own row, and only the second of the two is a remote.
    property var remoteNames: []
    /// Whether this row is a remote itself. Its fold key is the remote's whole name, which is what makes the test
    /// above work on a name with a slash in it.
    readonly property bool isRemoteRow:
        navRow.kindHint === "remote" && navRow.folder && navRow.remoteNames.indexOf(navRow.fullName) >= 0
    /// Whether this row is the marked one.
    readonly property bool pushesHere: navRow.isRemoteRow && navRow.fullName === navRow.markedRemote
    property real listWidth: 200
    /// Where this row's ink begins, and how far each fold of the names steps it in. Handed down because the left panel
    /// sets its rows in from the pane's edge by the room its own bar takes at the other one, so a row stands between
    /// two equal margins (`NavList` — デザイン規約 §余白); the working tree's file list keeps the pane's inner margin it
    /// shares with a commit's own list (`FileRowDelegate`). **Two values**: a list is free to start its rows on one
    /// step and fold them on another, and the file lists do (the pane's inner margin, then a group's step). The left
    /// panel is the one that asks for the same value twice, so that a name at any depth stands on the grid its own
    /// left margin sets.
    property int rowInset: Theme.spaceXs
    property int nestStep: Theme.spaceMd
    /// Where an open name box is drawn, and where the list showing this row sits in it: where its rows begin, where
    /// the list itself begins, and how tall it is. All of it comes from the list (`NavList`), so that a scroll moves
    /// the box with its row and a row carried out of the list takes the box with it. No layer means the box stays in
    /// its seat, which is what a list nobody can type into hands down.
    property Item boxLayer: null
    property real boxRowsX: 0
    property real boxRowsY: 0
    property real boxRowsTop: 0
    property real boxRowsHeight: 0
    // Shows the hover stage/unstage affordance (WIP view).
    property bool showStage: false
    /// Whether this row is one of those chosen (working-tree list). Held by the list, since a delegate is recycled the
    /// moment its row scrolls off.
    property bool chosen: false
    /// The pointer is on another chosen row's stage affordance, and this row goes with it. The marks come out together
    /// so that what one press moves is seen before it is pressed (デザイン規約 §その他の操作).
    property bool stagePeer: false
    /// What each side of a conflict is called. **The two swap over during a rebase**, so they are handed down from the
    /// model (`WorkTreeModel.sideOurs` / `sideTheirs`); empty where git left nothing to
    /// name a side by.
    property string sideOurs: ""
    property string sideTheirs: ""

    /// What the two sides each did to this file (デザイン規約 §conflict の種別). The sentence itself lives in `Words`: the diff
    /// pane says the same one on the conflicts git prints no patch for, and two copies of it would be two answers.
    function conflictWords() {
        return Words.conflict(navRow.change, navRow.sideOurs, navRow.sideTheirs)
    }

    // ---- the two-click gestures ------------------------------------
    // Which row was clicked last, and which is being typed into, are held by the sidebar: delegates are recycled the
    // moment a row scrolls off (デザイン規約 §左メニューの所作).
    property string rowKey: ""
    property string activeKey: ""
    property string editKey: ""
    /// "rename" (the name is in the box) or "branch" (a name for a new branch on this row's commit).
    property string editMode: ""
    /// What has been typed so far, held by the sidebar so a row that scrolls off and comes back does not lose it.
    property string editText: ""
    property bool editRefused: false
    property string editRefusedWhy: ""
    readonly property bool editing: navRow.editKey !== "" && navRow.editKey === navRow.rowKey
    /// The name git knows this row by.
    readonly property string fullName: navRow.full !== "" ? navRow.full : navRow.name
    /// What the walk over this list calls this row, empty on a folder — the one name both file lists' rows answer to
    /// (`FileRowWalk`).
    readonly property string walkKey: navRow.folder ? "" : navRow.fullName
    /// That name while the row is painted as one of the chosen, empty otherwise — what a headless run reads off the
    /// list. The rectangle's own `visible`, since reading `chosen` back would go green with the rectangle unwired.
    readonly property string litKey: chosenBox.visible ? navRow.walkKey : ""
    /// The same rectangle as a bare answer, so a count of the lit rows does not go through the name each of them
    /// carries (`FileRowWalk.litRows`, and `FileRowDelegate.litNow` — the other list's rows answer the same pair).
    readonly property bool litNow: chosenBox.visible

    signal refClicked(string oidHex)
    /// A working-tree file row was clicked. `modifiers` carries Ctrl and Shift, which is how several rows are chosen at
    /// once.
    signal fileClicked(string bucket, string path, string origPath, int modifiers)
    signal folderClicked(string key)
    signal stageClicked(string bucket, string path)
    /// The pointer arrived at (or left) this row's stage affordance.
    signal stageHovered(string bucket, string path, bool on)
    /// A left click landed on this row, whatever it then meant.
    signal rowClicked()
    /// Double-click: go where this row leads.
    signal activateRequested()
    /// The box: typed into, accepted, walked away from.
    signal editTyped(string text)
    signal editAccepted(string text)
    signal editCancelled()
    /// Right-click on a ref row; the page owns the menu because delegates are recycled out from under an open popup.
    /// `name` is what the row shows, `full` what git knows it by (a stash shows a message and answers to a selector).
    signal refMenuRequested(string name, string full, string oidHex)
    /// Right-click on the row a remote itself stands on — the only folder row in this list that has anything behind it
    /// (デザイン規約 §左メニューの所作). Its own signal because what opens is a different menu: a remote is configuration.
    signal remoteMenuRequested(string name)
    /// Right-click on a working-tree file row, for the same reason. Undoing a rename takes both of its names, but the
    /// menu reads them off the chosen rows (`orig_path`), so the row itself is enough.
    signal fileMenuRequested(string bucket, string path)

    width: listWidth
    // **The row grows by what it has open under it** and the rows below it move down: the list opens rather than
    // something landing on top of it (デザイン規約 §左メニューの所作). Every wash below fills the whole of it, because
    // what grew is this row.
    //
    // Two things grow it: the name shown whole, where it needed more than the one line the row draws it on, and the
    // facts under that.
    height: Theme.rowHeight + navRow.nameOverflow + factsSeat.height
    /// How far the name shown whole hangs below the line the row draws it on — 0 while the row is closed, and while
    /// the whole of it fits the line it was already on (`NameCell.wholeOver`).
    readonly property real nameOverflow: rowLayout.nameWholeOver

    // The current branch stays highlighted inside the list (the sidebar's sticky row only stands in for it while this
    // row is scrolled off).
    Rectangle {
        anchors.fill: parent
        color: Theme.accentMuted
        visible: navRow.is_head && !navRow.folder
    }
    // The row a click last landed on. Without it the second click of the rename gesture would be aimed at nothing, and
    // a click on a row whose commit is already the one being read — a worktree standing where the graph already is —
    // would look like it missed.
    Rectangle {
        id: chosenBox
        anchors.fill: parent
        color: Theme.bgSelected
        visible: !navRow.folder && (navRow.chosen || (navRow.rowKey !== "" && navRow.activeKey === navRow.rowKey))
    }
    Rectangle {
        id: washBox
        anchors.fill: parent
        color: Theme.bgHover
        // The stand-in lights the row as the pointer does, so a picture taken of a row that says nothing still shows
        // where the pointer was standing (`tipPointedAt`). **An open row stays lit** whichever of the two put it
        // there: the facts under it are part of the row, and a row that went dark under its own open lines would
        // leave the reader looking at facts belonging to nothing.
        visible: navRow.pointed || navRow.tipPointedAt || navRow.factsOpen
    }
    // Nothing asks a question about a row in this list any more. What one of these rows takes away is held down on
    // the menu row that names it, and that menu is standing over the row while it is held (デザイン規約 §長押し).
    // The ink of the row, one column after another (`NavRowBody`). Handed the row itself — a delegate is recycled,
    // and mirroring them here would double every binding it pays on reuse.
    NavRowBody {
        id: rowLayout
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: Theme.rowHeight
        anchors.leftMargin: navRow.rowInset + navRow.depth * navRow.nestStep
        // The gutter the list's own scroll bar is drawn in. This row ends in a right-aligned column (the branch a
        // worktree has out) and the bar is drawn over it, so the row stops where the bar's ink begins. Both lists
        // that show these rows are a pane's own — the left panel's and the working tree's — so they ask for the one
        // gutter that bar takes (デザイン規約 §QML 実装ルール の摘みの項).
        anchors.rightMargin: Theme.navBarGutter
        row: navRow
    }
    // The name, in a box, where the name was — drawn outside the list, which is the box's own business (`NavNameBox`).
    //
    // **Built only while this row is being typed into.** A delegate is built per row on screen, and the box is a text
    // field with a ruler of its own — on every row nobody is naming it was built and hidden, which is the heap the
    // rows are measured by (rules-refs/app-ui.md, the Loader rule). The box parents itself out of here on its own
    // (`NavNameBox.parent`); the loader only owns it.
    Loader {
        id: editSeat
        active: navRow.editing
        sourceComponent: NavNameBox {
            row: navRow
            seat: rowLayout.boxSeat
            drawnIn: navRow.boxLayer
            // Where the seat sits inside the row is the layout's to say, so what the box is told is where the row's
            // own columns begin; it adds the seat's place to that itself.
            rowsX: navRow.boxRowsX + rowLayout.x
            rowsY: navRow.boxRowsY
            rowsTop: navRow.boxRowsTop
            rowsHeight: navRow.boxRowsHeight
            editing: navRow.editing
            mode: navRow.editMode
            // What the box is naming, which is not always what the row is: `Create tag here…` opens on a branch row
            // too.
            namesKind: navRow.editMode === "tag" ? "tag"
                     : navRow.editMode === "branch" ? "branch" : navRow.kindHint
            carried: navRow.editText
            refused: navRow.editRefused
            refusedWhy: navRow.editRefusedWhy
            onTyped: text => navRow.editTyped(text)
            onSubmitted: text => navRow.editAccepted(text)
            onCancelled: navRow.editCancelled()
        }
    }
    // Rows whose name can be changed from here. A remote branch is one of them even though git has no rename over
    // there — core builds the rename out of a push and a delete, and the bar asks before it runs. A folder is left
    // out: it is the shape of the names below it.
    readonly property bool nameable: !navRow.folder
        && (navRow.kindHint === "branch" || navRow.kindHint === "tag"
            || navRow.kindHint === "stash" || navRow.kindHint === "remote")
    /// The gesture the rows of this section share, or null for a list whose rows cannot be typed into (the working
    /// tree's files). Held by the sidebar, since it has to outlive this delegate (`SidebarRowGestures`).
    property ReclickGesture reclick: null
    /// Whether this row is holding the wait the name box opens after, and whether a click landing now would still be
    /// counted as the other half of a double-click. What a headless run reads to see the gesture armed, and to know
    /// when a second click of its own counts as a second (app-ui.md §UI 自動化の因果性). Both are the sidebar's answer:
    /// the gesture lives with it, because this delegate is recycled the moment its row scrolls off
    /// (`SidebarRowGestures`).
    readonly property bool renameArmed: navRow.reclick ? navRow.reclick.armedFor(navRow.rowKey) : false
    readonly property bool clickGuarded: navRow.reclick ? navRow.reclick.guarded : false
    /// Whether the box on this row has the keyboard. The output side: a box drawn where nothing can be typed reads as
    /// a box, and the folded list's section is a popup, which takes the keyboard only when something in it asks.
    readonly property bool editFocused: editSeat.item ? editSeat.item.activeFocus : false
    /// What the box came out as, for the runs that photograph it (`NavNameBox`): as drawn, as what is in it wants,
    /// whether it is on screen at all, and where it landed.
    readonly property real editBoxWidth: editSeat.item ? editSeat.item.width : 0
    readonly property real editBoxWhole: editSeat.item ? editSeat.item.wantWidth : 0
    readonly property real editBoxSeat: rowLayout.boxSeat.width
    readonly property bool editBoxShown: editSeat.item ? editSeat.item.visible : false
    readonly property string editBoxAt: editSeat.item ? editSeat.item.cameOut : ""
    /// Whether the one shared tooltip is standing **on this row's box** — the reason a refused name gives, which is
    /// said nowhere else (`NavNameBox`). Taken off the box's own attached read, because that is the one that weighs
    /// the instance's target against this item (`tests/qml/tst_tipowner.qml`); the instance's own `visible` would
    /// answer the same for anybody's tip. The shape `SignatureMark.tipShown` and `MessageEditor.summaryTipShown`
    /// already carry.
    readonly property bool editTipShown: editSeat.item ? editSeat.item.ToolTip.visible : false
    /// A left click, as this row answers one. Named so that a run with no pointer to press with puts its click in at
    /// the row itself (PGG_AUTO_ACT=nav-reclick).
    function leftClick(modifiers, held) {
        // The second click of a double-click belongs to the double: the first one already did what a click
        // does.
        //
        // What a second click on this row would name is read now: by the time the wait runs out this delegate may
        // be showing another row's name (`ReclickGesture`).
        if (navRow.reclick
                && !navRow.reclick.click(navRow.rowKey,
                                         navRow.nameable ? navRow.renameNames() : null,
                                         held))
            return
        navRow.rowClicked()
        navRow.ordinaryClick(modifiers)
    }
    /// What the box a second click opens is named after: the row's kind, what git knows it by, the commit it is on,
    /// and the name it shows (a stash is typed by its message, not by its selector).
    function renameNames() {
        return {
            "kind": navRow.kindHint,
            "id": navRow.full !== "" ? navRow.full : navRow.name,
            "oid": navRow.oid_hex,
            "name": navRow.name
        }
    }
    function ordinaryClick(modifiers) {
        if (navRow.folder) {
            navRow.folderClicked(navRow.full)
        } else if (navRow.kindHint === "wt") {
            navRow.fileClicked(navRow.bucket, navRow.fullName, navRow.orig_path,
                               modifiers === undefined ? Qt.NoModifier : modifiers)
        } else if (navRow.oid_hex !== "") {
            // Every row that names a commit goes the one way, a worktree row with the rest of them: the commit its
            // checkout is standing on (デザイン規約 §左メニューの所作). A bare entry names none and is the one row here that
            // answers a click with nothing.
            navRow.refClicked(navRow.oid_hex)
        }
    }
    // Which row the hand is on. A handler because handlers are passive: the wash, the mark and the row's own card go
    // on answering the one pointer however many children of this row take hover of their own — and one of them,
    // `stageButton`, is a `Control` that takes its own. Gated the way the area below is: while the box is open the row
    // belongs to it.
    HoverHandler {
        id: rowHover
        enabled: !navRow.editing
        onHoveredChanged: navRow.pointed = rowHover.hovered
    }
    // The row that carries the mark tells the model it is the one being read, so the sentence can be built for it
    // alone. The row itself has no field left to hold it (`NavItem::eol_mark`).
    //
    // **On a rest, the same one every other hover in this app opens after** (規約 §hover のツールチップ「手が止まって
    // から開く」): these rows stand in a list, and a hand crossing it passes over every marked one on the way. **The
    // letting go is at once** — what the pointer has left is not what a beat is for, and the card keeps its own
    // (`WipPane.pointEol`).
    //
    // **The hand coming back from the card is answered at once** (規約「出ているものの的へ戻る手は即通す」): the
    // card opens off the row's own bottom edge, so reading it takes the pointer off the row and returning puts it back.
    // Rested on a second time, the card would go at `hoverKeepMs` and come back at `tipDelayMs` — a blink that
    // punishes the ordinary way of reading what the row put out.
    function pointEol() {
        if (!navRow.eol_mark)
            return
        if (!navRow.pointed) {
            eolRest.stop()
            navRow.eolPointed(navRow.fullName, false)
            return
        }
        if (navRow.eolCardOut)
            navRow.eolPointed(navRow.fullName, true)
        else
            eolRest.restart()
    }
    // The row's own facts sit out the same rest, and go the moment the pointer leaves: they are **inside** the row,
    // so the hand reading them never leaves it and there is no beat to keep (規約 §hover のツールチップ).
    function pointFacts() {
        if (!navRow.expands)
            return
        if (navRow.pointed) {
            factsWait.restart()
        } else {
            factsWait.stop()
            navRow.factsAsked(false, Qt.point(0, 0))
        }
    }
    onPointedChanged: {
        navRow.pointEol()
        navRow.pointFacts()
    }
    // The rest itself. **It asks the row again when it runs out**: a delegate is recycled the moment its row scrolls
    // off, so the one holding this timer need not be the row the hand was resting on.
    Timer {
        id: eolRest
        interval: Metrics.tipDelayMs
        onTriggered: {
            if (navRow.eol_mark && navRow.pointed)
                navRow.eolPointed(navRow.fullName, true)
        }
    }
    MouseArea {
        id: itemMouse
        anchors.fill: parent
        // While the box is open the row belongs to it.
        visible: !navRow.editing
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        /// When the button went down, so the gesture can take the time it was held off the wait it has left — Qt
        /// measures a double-click press to press, and `clicked` arrives at the release.
        property real pressAt: 0
        /// Where it went down, and whether this press has already been handed on to the name. **Each handler is
        /// one line into the row's own** — a run enters those, so what it drives is this wiring rather than a copy
        /// of it (verify-ui スキル §注入はハンドラ本体そのものへ入れる).
        property point pressFrom: Qt.point(0, 0)
        property bool pressing: false
        property bool handedOn: false
        onPressed: mouse => navRow.linePressed(mouse.button, mouse.x, mouse.y)
        onPositionChanged: mouse => navRow.lineDragged(mouse.x, mouse.y)
        onReleased: navRow.lineReleased()
        onCanceled: navRow.lineReleased()
        onClicked: mouse => navRow.lineClicked(mouse.button, mouse.modifiers)
        onDoubleClicked: mouse => navRow.rowDoubled(mouse.button)
    }
    /// The name as the open row shows it, and what a drag over it came away with — null and empty while the row
    /// is closed (`NameCell.whole`).
    readonly property Item nameField: rowLayout.nameWhole
    readonly property string nameTook: navRow.nameField ? navRow.nameField.selected : ""
    readonly property bool nameCaret: !!navRow.nameField && navRow.nameField.hasCaret

    /// A press on the row's **own line**, and what a hand that starts to move with it means. A press that never
    /// moved is this row's click; one that moves is a reader going for the name, so the row opens **now** rather
    /// than after the rest and the drag carries straight on into the name it put there
    /// (デザイン規約 §左メニューの所作「開いた字は掴める」). The lines under the row answer their own presses
    /// (`NavRowFacts`); this is the half on the row's line, where the name is.
    function linePressed(button, x, y) {
        if (button !== Qt.LeftButton)
            return
        itemMouse.pressAt = Date.now()
        itemMouse.pressFrom = Qt.point(x, y)
        itemMouse.pressing = true
        itemMouse.handedOn = false
        // One selection in the window: a press up here takes the last one off the name and off the lines below, so
        // a click that takes no words is read as a click and not as the sweep before it.
        if (navRow.factsItem !== null)
            navRow.factsItem.dropSweep()
        if (navRow.nameField !== null)
            navRow.nameField.deselect()
    }
    function lineDragged(x, y) {
        if (!navRow.expands || !itemMouse.pressing)
            return
        // The platform's own answer to "has this hand moved on purpose", so a click with a tremor in it is still a
        // click (`RebasePlanRow` reads the same one).
        if (!itemMouse.handedOn
                && Math.abs(x - itemMouse.pressFrom.x) < Qt.styleHints.startDragDistance
                && Math.abs(y - itemMouse.pressFrom.y) < Qt.styleHints.startDragDistance)
            return
        if (!itemMouse.handedOn) {
            // The rest is what this press is skipping — the reader has already chosen this row.
            factsWait.stop()
            if (!navRow.factsOpen)
                navRow.askFacts(navRow.mapToItem(null, x, y))
            // Refused (a box is open, a menu is standing): there is no field to sweep, and the press stays a press.
            if (navRow.nameField === null)
                return
            // Anchored where the button went down. The field clamps a point outside its own box to the nearest
            // character, so a press in the air beside a short name starts at the end of it (`CardText.inBox`).
            navRow.nameField.anchorFrom(navRow, itemMouse.pressFrom.x, itemMouse.pressFrom.y)
            itemMouse.handedOn = true
        }
        navRow.nameField.extendFrom(navRow, x, y)
    }
    function lineReleased() {
        itemMouse.pressing = false
    }
    /// The click at the end of it. **A drag that took the name away is not a click** — the reader was copying, and
    /// the second click of a gesture opens a name box (`NavRowFacts.handClicked`, the same test one line down).
    function lineClicked(button, modifiers) {
        if (button === Qt.LeftButton && navRow.nameTook !== "")
            return
        navRow.rowPressed(button, modifiers, Date.now() - itemMouse.pressAt)
    }
    /// A press on this row, and a double-click on it — **the handler above is one line and this is the whole of
    /// what it does**, because the lines the row opens under itself answer with these two as well (`NavRowFacts`):
    /// the hand there takes every press over them so the words can be dragged out, and a press that never moved is
    /// this row's own click.
    function rowPressed(button, modifiers, held) {
        if (button === Qt.RightButton) {
            // Only rows with operations behind them open a menu.
            if (!navRow.folder && navRow.oid_hex !== ""
                    && (navRow.kindHint === "branch"
                        || navRow.kindHint === "remote"
                        || navRow.kindHint === "tag"
                        || navRow.kindHint === "stash"))
                navRow.refMenuRequested(navRow.name, navRow.fullName, navRow.oid_hex)
            else if (!navRow.folder && navRow.kindHint === "wt")
                navRow.fileMenuRequested(navRow.bucket, navRow.fullName)
            else if (navRow.isRemoteRow)
                navRow.remoteMenuRequested(navRow.fullName)
            return
        }
        navRow.leftClick(modifiers, held)
    }
    function rowDoubled(button) {
        if (button !== Qt.LeftButton || navRow.folder)
            return
        if (navRow.reclick)
            navRow.reclick.drop()
        navRow.activateRequested()
    }
    // Hover stage/unstage affordance.
    HoverToolButton {
        id: stageButton
        visible: navRow.showStage && !navRow.folder && (navRow.pointed || navRow.stagePeer)
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        // The row's own line, not the whole of a row that has facts open under it (the working tree's rows never do,
        // and a mark that centred on both would sit in different places in the two lists).
        anchors.verticalCenter: rowLayout.verticalCenter
        padding: 0
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        // The list is told which row the pointer is on, so the rows that would go with it can put their own marks out.
        onHoveredChanged: navRow.stageHovered(
            navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name,
            stageButton.hovered)
        // On a conflicted row the same `git add` means something else: it tells git the conflict has been dealt
        // with, and the word on this button says so (デザイン規約
        // §diff の中のステージ).
        tip: navRow.bucket === "conflicts" ? qsTr("Mark resolved")
             : navRow.bucket === "staged" ? qsTr("Unstage file")
                                          : qsTr("Stage file")
        onClicked: navRow.stageClicked(navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name)
        contentItem: NavIcon {
            kind: navRow.bucket === "staged" ? "minus" : "plus"
            tint: navRow.bucket === "staged" ? Theme.diffRemovedFg : Theme.diffAddedFg
        }
    }
    /// Whether the list this row is in opens a row's facts under it, and which row is open there — handed down
    /// because a delegate is recycled the moment its row scrolls off (`SidebarRowGestures.openKey`).
    property bool opensFacts: false
    property string openKey: ""
    /// This row's facts are asked for, or let go of, at the place the pointer was standing. Which row is open is the
    /// pane's to hold: data comes down, gestures go up.
    signal factsAsked(bool open, point at)
    /// The menu about to be raised was asked for from those facts, so it is not one that takes them down
    /// (`NavRowFacts.handClicked`).
    signal factsMenuAsked()
    /// Where this row's answers come off: its own section's model (`upstreamOf`) and the worktrees' section
    /// (`worktreeHolding`), which is the only one holding the list of working copies.
    property var sectionModel: null
    property var worktreesModel: null
    /// Whether this row answers a rest by opening. **The whole of the section the facts were handed to**, folder rows
    /// included: a fold parent that answered with a tooltip while the rows under it opened would be the same question
    /// answered two ways in one list (デザイン規約 §左メニューの所作).
    ///
    /// **The one row of REMOTES that is left out is the remote's own** — it is a thing in itself rather than the
    /// shape of the names under it, and what it has to say is the role it holds (the default remote), which is a
    /// sentence and not a name (`hoverText`). Every other row of that section opens, as every row of BRANCHES does.
    readonly property bool expands:
        navRow.opensFacts
        && (navRow.kindHint === "branch"
            || (navRow.kindHint === "remote" && !navRow.isRemoteRow))
    /// Whether this row is the open one. The marks it spells out go from the row's own line while it is, so the
    /// reader sees them move down rather than stand twice; the wash stays, because the row grew.
    ///
    /// **The key carries the section** (`NavList.keyOf`, the key a click is remembered by): a name a remote
    /// carries and a branch here carries are two different refs (デザイン規約 §ref の種別), and a bare name would
    /// open both of their rows at once.
    readonly property bool factsOpen: navRow.expands && navRow.fullName !== "" && navRow.openKey === navRow.rowKey
    /// Whether the row is painted as the one under the hand — the rectangle's own answer, so a run cannot go green
    /// with the wash unwired (the reading `litKey` already makes of the chosen row).
    readonly property bool washLit: washBox.visible
    /// The lines this row has open under it, where a run drives the hand that sweeps them (`NavRowFacts`). Null
    /// wherever the row is closed. `lineHeight` is the row's own line beside them — what a run reads to see that the
    /// lines were seated under it and not over it.
    readonly property Item factsItem: factsSeat.item
    readonly property real lineHeight: rowLayout.height

    // Hover says the name in full — the one thing the row itself cannot show (デザイン規約 §hover のツールチップ). What a row
    // shows is a part of it: a leaf folded into its folders shows the last segment, and a name wider than the pane
    // shows a middle-elided one. The name alone — where the row leads is what the section and the gesture already
    // say. A stash is the same rule read on what it is named by: the row *is* the message, so the message in full is
    // its name, and the selector (`stash@{0}`) is not something anybody hovers to learn.
    readonly property string hoverText: {
        const full = navRow.fullName
        // A row that opens says it all under itself, name included — two things opening off one pointer would sit on
        // top of each other (the reading the line-ending mark's own row makes above).
        if (navRow.expands)
            return ""
        // The one folder row that is a thing in itself says what it is for when it holds the mark. The role
        // leads and the name follows it (デザイン規約 §hover のツールチップ: 結論から 1 行 — the same shape a working copy's row
        // says its state in), and `origin` is the word git gives the role (§リモートを書き留める).
        if (navRow.pushesHere)
            return qsTr("Default remote (origin) — %1").arg(full)
        // A folder in the working tree's list says its own path, the same as the file rows under it — the path rides
        // in `orig_path` (`full` is the fold key, and a ref folder's fold key is its own path).
        if (navRow.folder)
            return navRow.kindHint === "wt" ? navRow.orig_path : full
        if (navRow.kindHint === "worktree") {
            // The state of the checkout comes before where the row leads: it is what the mark in the seat cannot spell
            // out, and on a locked row the words git was given are the whole of what a reader hovers to learn (デザイン規約
            // §hover のツールチップ: 結論から 1 行). A lock taken without a reason has nothing after the word.
            if (navRow.change === "LOCKED")
                return navRow.orig_path === "" ? qsTr("Locked")
                                               : qsTr("Locked — %1").arg(navRow.orig_path)
            if (navRow.change === "PRUNABLE")
                return navRow.orig_path === "" ? qsTr("Folder is gone")
                                               : qsTr("Folder is gone — %1").arg(navRow.orig_path)
            return qsTr("Open %1 in a new tab").arg(full)
        }
        if (navRow.kindHint === "stash")
            return navRow.name
        if (navRow.kindHint === "branch" || navRow.kindHint === "tag" || navRow.kindHint === "remote")
            return full
        // A row carrying the line-ending mark has a card of its own, which names the path as its first line — two
        // things opening off one pointer would sit on top of each other (規約 §hover のツールチップ「1 つのポインタが
        // 開けるものは 1 つ」). **The mark is the test**: both open after the same rest, so a row that asked "is my
        // card up yet?" would raise a tip in the turn before it was.
        if (navRow.eol_mark)
            return ""
        // What is left is a file row: hover says the path, whatever the row shows and however wide the pane is
        // (デザイン規約 §hover のツールチップ).
        return full
    }
    /// Whether the card standing is this row's. Only one row can have it, so the pane keeps the answer once and the
    /// row weighs it against its own name before reading it (`NavSectionModel::point_eol`).
    readonly property bool eolCardOut: navRow.eol_mark && navRow.pointedEolPath === navRow.fullName
    /// Whether the headless stand-in points at this row. The report still reads the ToolTip's own visible — the output
    /// side, as everywhere.
    /// **`>= 0` first**: a delegate the view has put back in its reuse pool reports `index` -1, and -1 is also "the
    /// pointer is on no row" — without the guard every pooled row claims the shared tooltip, and the one row actually
    /// pointed at never gets it (the instance is one per window).
    readonly property bool tipPointedAt: navRow.pointedTipRow >= 0 && navRow.pointedTipRow === navRow.index
    // Held down while a menu stands: the pointer is in the menu, and a tip coming out now lands on the rows the
    // hand is reading (デザイン規約 §メニュー).
    ToolTip.visible: (navRow.pointed || navRow.tipPointedAt) && !navRow.editing && !navRow.menuStanding
                     && navRow.hoverText !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: navRow.hoverText

    /// What the row opens under itself, read as it opens rather than bound: the answers come off slots, which a
    /// binding freezes at the value they had when it ran (app-ui.md), and this is the same "asked once, as it opens"
    /// a menu makes of its own row.
    property string factsName: ""
    property string factsLocal: ""
    property string factsHeldBy: ""
    property string factsUpstream: ""
    property bool factsGone: false

    /// Read them, and answer whether there is anything to open. **The name alone is enough**: it is the copy a
    /// reader can drag away, so every row that expands has it and every one of them opens. A row with no name to
    /// say — one the model has not filled yet — opens nothing.
    function gatherFacts() {
        // The branch measured against this reading, and the copy holding **that** one: a remote-tracking ref is
        // nobody's checkout, so the working copy this row can name is the one holding the branch above it.
        const local = navRow.kindHint === "remote" && !navRow.folder && navRow.sectionModel !== null
                    ? navRow.sectionModel.trackedBy(navRow.fullName) : ""
        const upstream = navRow.kindHint === "branch" && navRow.sectionModel !== null
                       ? navRow.sectionModel.upstreamOf(navRow.fullName) : ""
        // Configured and not here — the reading the branch is measured against cannot be reached, and git spells the
        // same answer `[gone]` (デザイン規約 §ref の種別). **Off the row rather than the section**: the badge the row
        // wears is drawn from the same slot, so the mark and the line under it cannot disagree
        // (`models::nav::field` の `Role::Bucket`).
        const gone = navRow.kindHint === "branch" ? navRow.bucket : ""
        // The name the row asks the worktrees section about: its own on a BRANCHES row, the branch named above it
        // on a REMOTES one.
        const holds = navRow.kindHint === "remote" ? local : navRow.fullName
        // The worktrees section answers with the path git prints; the rows of that section show the folder it ends
        // in, and this says the same name they do.
        const held = navRow.folder || holds === "" || navRow.worktreesModel === null ? ""
                   : navRow.worktreesModel.worktreeHolding(holds)
        navRow.factsLocal = local
        navRow.factsUpstream = gone !== "" ? gone : upstream
        navRow.factsGone = gone !== ""
        navRow.factsHeldBy = GitFacts.pathLeaf(held)
        navRow.factsName = navRow.fullName
        return navRow.factsName !== ""
    }
    /// Ask for this row to be the open one, with the pointer where it was standing when it asked.
    function askFacts(at) {
        if (navRow.expands && !navRow.editing && !navRow.menuStanding && navRow.gatherFacts())
            navRow.factsAsked(true, at)
    }
    // The rest before the facts come out — what this adds is not on the row's line, and a hand crossing a column of
    // rows passes over every one of them (規約 §hover のツールチップ「補足は待ってから開く」).
    Timer {
        id: factsWait
        interval: Metrics.tipDelayMs
        onTriggered: if (navRow.pointed) navRow.askFacts(rowHover.point.scenePosition)
    }
    // The pointer's stand-in opens the same facts the hand does, with no rest to sit out — a run has no hand to rest
    // (verify-ui スキル). The point it names is the row's own middle, so two rows are never asked for from one place.
    onTipPointedAtChanged: {
        if (navRow.tipPointedAt)
            navRow.askFacts(navRow.mapToItem(null, navRow.width / 2, Theme.rowHeight / 2))
        else if (navRow.expands)
            navRow.factsAsked(false, Qt.point(0, 0))
    }
    // A row that comes back open — recycled onto the open name while it was scrolled off — reads its answers again.
    onFactsOpenChanged: if (navRow.factsOpen) navRow.gatherFacts()
    // The facts, under the row's own line and inside it (`NavRowFacts`). **Built only while the row is open**: a
    // delegate is built per row on screen, and one built and hidden on every row is the heap the rows are measured
    // by (rules-refs/app-ui.md, the Loader rule).
    Loader {
        id: factsSeat
        active: navRow.factsOpen
        visible: factsSeat.active
        // Under the row's own line, in the columns it draws in. **Measured off the row's height rather than off the
        // layout above**: a `RowLayout` takes the height its content asks for, which is less than a row's, and lines
        // seated on its foot would be drawn over the name (measured — the name went under them). A name that wrapped
        // hangs below that line, so these begin under the last line of it.
        anchors.top: parent.top
        anchors.topMargin: Theme.rowHeight + navRow.nameOverflow
        anchors.left: parent.left
        anchors.leftMargin: navRow.rowInset + navRow.depth * navRow.nestStep
        anchors.right: parent.right
        anchors.rightMargin: Theme.navBarGutter
        height: factsSeat.item ? factsSeat.item.implicitHeight : 0
        sourceComponent: NavRowFacts {
            row: navRow
            localBranch: navRow.factsLocal
            // The counts ride the row itself (`models::nav::field` の `Role::Ahead`), the way the `[gone]` state
            // does: they are a role, so they answer again when a fetch moves them.
            ahead: navRow.ahead
            behind: navRow.behind
            heldBy: navRow.factsHeldBy
            upstream: navRow.factsUpstream
            gone: navRow.factsGone
        }
    }
}
