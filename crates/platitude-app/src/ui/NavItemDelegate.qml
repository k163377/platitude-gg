pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
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
    /// The pointed row's path, handed down by the pane — the model keeps the line-ending words once rather than on
    /// every row, and a row checks the answer is its own before reading them (`eolPointedAt`).
    property string pointedEolPath: ""
    /// The pointer arrived at, or left, a row carrying the mark.
    signal eolPointed(string path, bool on)
    /// Stands in for the pointer where headless cannot put one, so a cut-down row's tooltip can be photographed
    /// (PG_AUTO_ACT=path-tip). -1 points at no row.
    property int pointedTipRow: -1
    /// Whether the pointer is on this row. Written by the handler below rather than by the `MouseArea` that fills the
    /// row: hover goes to the topmost item that takes it, and the stage `+` is a `Control` that takes its own, so the
    /// row stopped being "under the hand" exactly when the hand arrived at the mark (rules-refs/app-ui.md 「行の hover
    /// を `MouseArea` で取らない」; 実測 qmltestrunner: `containsMouse=false` with the pointer in the middle of the `+`,
    /// which took the row's wash out from under the hand reaching for it and closed its line-ending card).
    property bool pointed: false
    /// A right-click menu of the page's is standing over this list.
    property bool menuStanding: false
    property string kindHint: "branch"
    /// The remote this repository sends pushes to, empty where none is marked (`RepoTab.pushDefault`). Handed down
    /// rather than read here: one answer for the whole list, and a delegate is recycled row to row.
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
    /// The current branch's ahead / behind. What travels is the two numbers — the row draws the arrows itself
    /// (`HeadTrack`).
    property bool headTracks: false
    property int headAhead: 0
    property int headBehind: 0
    property real listWidth: 200
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
    /// model rather than worked out here (`WorkTreeModel.sideOurs` / `sideTheirs`); empty where git left nothing to
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
    /// (デザイン規約 §左メニューの所作). Its own signal because what opens is a different menu: a remote is configuration, not a ref.
    signal remoteMenuRequested(string name)
    /// Right-click on a working-tree file row, for the same reason. Undoing a rename takes both of its names, but the
    /// menu reads them off the chosen rows (`orig_path`), so the row itself is enough.
    signal fileMenuRequested(string bucket, string path)

    width: listWidth
    height: Theme.rowHeight

    // The current branch stays highlighted inside the list (the sidebar's sticky row only stands in for it while this
    // row is scrolled off).
    Rectangle {
        anchors.fill: parent
        color: Theme.accentMuted
        visible: navRow.is_head && !navRow.folder
    }
    // The row a click last landed on. Without it the second click of the rename gesture would be aimed at nothing, and
    // a click on a worktree row — which has nowhere to jump to — would look like it missed.
    Rectangle {
        id: chosenBox
        anchors.fill: parent
        color: Theme.bgSelected
        visible: !navRow.folder && (navRow.chosen || (navRow.rowKey !== "" && navRow.activeKey === navRow.rowKey))
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        // The stand-in lights the row as the pointer does, so a picture taken of a row that says nothing still shows
        // where the pointer was standing (`tipPointedAt`).
        visible: navRow.pointed || navRow.tipPointedAt
    }
    // No mark: nothing asks a question about a row in this list any more. What one of these rows takes away is held
    // down on the menu row that names it, and that menu is standing over the row while it is held (デザイン規約 §長押し).
    // The ink of the row, one column after another (`NavRowBody`). Handed the row itself rather than its fields — a
    // delegate is recycled, and mirroring them here would double every binding it pays on reuse.
    NavRowBody {
        id: rowLayout
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs + navRow.depth * Theme.spaceMd
        // Not padding: the gutter the list's own scroll bar is drawn in. This row ends in a right-aligned column (the
        // branch a worktree has out), and the thumb is drawn over it — the bar's box is 10 wide and holds 2 of its
        // own, so the ink reaches 8 in and a gutter of `spaceSm` ends the row exactly where that ink begins
        // (デザイン規約 §余白).
        anchors.rightMargin: Theme.scrollBarWidth
        row: navRow
    }
    // The name, in a box, where the name was — drawn outside the list, which is the box's own business (`NavNameBox`).
    NavNameBox {
        id: editField
        row: navRow
        seat: rowLayout.boxSeat
        drawnIn: navRow.boxLayer
        // Where the seat sits inside the row is the layout's to say, so what the box is told is where the row's own
        // columns begin; it adds the seat's place to that itself.
        rowsX: navRow.boxRowsX + rowLayout.x
        rowsY: navRow.boxRowsY
        rowsTop: navRow.boxRowsTop
        rowsHeight: navRow.boxRowsHeight
        editing: navRow.editing
        mode: navRow.editMode
        // What the box is naming, which is not always what the row is: `Create tag here…` opens on a branch row too.
        namesKind: navRow.editMode === "tag" ? "tag"
                 : navRow.editMode === "branch" ? "branch" : navRow.kindHint
        carried: navRow.editText
        refused: navRow.editRefused
        refusedWhy: navRow.editRefusedWhy
        onTyped: text => navRow.editTyped(text)
        onSubmitted: text => navRow.editAccepted(text)
        onCancelled: navRow.editCancelled()
    }
    // Rows whose name can be changed from here. A remote branch is one of them even though git has no rename over there
    // — core builds the rename out of a push and a delete, and the bar asks before it runs. A folder is not: it is the
    // shape of the names below it, not a name.
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
    readonly property bool editFocused: editField.activeFocus
    /// What the box came out as, for the runs that photograph it (`NavNameBox`): as drawn, as what is in it wants,
    /// whether it is on screen at all, and where it landed.
    readonly property real editBoxWidth: editField.width
    readonly property real editBoxWhole: editField.wantWidth
    readonly property real editBoxSeat: rowLayout.boxSeat.width
    readonly property bool editBoxShown: editField.visible
    readonly property string editBoxAt: editField.cameOut
    /// A left click, as this row answers one. Named so that a run with no pointer to press with puts its click in at
    /// the row itself rather than at a copy of what the row would have decided (PG_AUTO_ACT=nav-reclick).
    function leftClick(modifiers, held) {
        // The second click of a double-click is not a click of its own: the first one already did what a click does,
        // and the gesture is the double.
        //
        // What a second click on this row would name is read now rather than when the wait runs out: by then this
        // delegate may be showing another row's name (`ReclickGesture`).
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
        } else if (navRow.kindHint === "worktree") {
            // Another repository: nothing here to jump to, so a click only takes the row (the double-click opens it as
            // a tab).
        } else if (navRow.oid_hex !== "") {
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
    onPointedChanged: if (navRow.eol_mark) navRow.eolPointed(navRow.fullName, navRow.pointed)
    MouseArea {
        id: itemMouse
        anchors.fill: parent
        // While the box is open the row belongs to it.
        visible: !navRow.editing
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        /// When the button went down, so the gesture can take the time it was held off the wait it has left — Qt
        /// measures a double-click press to press, and `clicked` arrives at the release.
        property real pressAt: 0
        onPressed: mouse => {
            if (mouse.button === Qt.LeftButton)
                itemMouse.pressAt = Date.now()
        }
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton) {
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
            navRow.leftClick(mouse.modifiers, Date.now() - itemMouse.pressAt)
        }
        onDoubleClicked: mouse => {
            if (mouse.button !== Qt.LeftButton || navRow.folder)
                return
            if (navRow.reclick)
                navRow.reclick.drop()
            navRow.activateRequested()
        }
    }
    // Hover stage/unstage affordance.
    HoverToolButton {
        id: stageButton
        visible: navRow.showStage && !navRow.folder && (navRow.pointed || navRow.stagePeer)
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        padding: 0
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        // The list is told which row the pointer is on, so the rows that would go with it can put their own marks out.
        onHoveredChanged: navRow.stageHovered(
            navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name,
            stageButton.hovered)
        // On a conflicted row the same `git add` means something else: it does not move a change into the staging area,
        // it tells git the conflict has been dealt with. The word says that rather than the command's other job (デザイン規約
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
    // Hover says the name in full — the one thing the row itself cannot show (デザイン規約 §hover のツールチップ). What a row
    // shows is a part of it: a leaf folded into its folders shows the last segment, and a name wider than the pane
    // shows a middle-elided one. Nothing is added to the name — where the row leads is what the section and the
    // gesture already say. A stash is the same rule read on what it is named by: the row *is* the message, so the
    // message in full is its name, and the selector (`stash@{0}`) is not something anybody hovers to learn.
    readonly property string hoverText: {
        const full = navRow.fullName
        // The one folder row that is a thing rather than a shape says what it is for when it holds the mark. The role
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
        // things opening off one pointer would sit on top of each other.
        if (navRow.eolPointedAt)
            return ""
        // What is left is a file row: hover says the path, whatever the row shows and however wide the pane is
        // (デザイン規約 §hover のツールチップ).
        return full
    }
    /// Whether the words on the model are this row's. Only one row can be pointed at, so they are kept once there
    /// rather than on every row (`NavSectionModel::point_eol`), and the row has to check that the answer is its own
    /// before reading it.
    readonly property bool eolPointedAt: navRow.eol_mark && navRow.pointedEolPath === navRow.fullName
    /// Whether the headless stand-in points at this row. The report still reads the ToolTip's own visible — the output
    /// side, as everywhere.
    /// **`>= 0` first**: a delegate the view has put back in its reuse pool reports `index` -1, and -1 is also "the
    /// pointer is on no row" — without the guard every pooled row claims the shared tooltip, and the one row actually
    /// pointed at never gets it (the instance is one per window).
    readonly property bool tipPointedAt: navRow.pointedTipRow >= 0 && navRow.pointedTipRow === navRow.index
    // Not behind a standing menu: the pointer is in the menu, and a tip that comes out now is drawn over the rows the
    // hand is reading (デザイン規約 §メニュー).
    ToolTip.visible: (navRow.pointed || navRow.tipPointedAt) && !navRow.editing && !navRow.menuStanding
                     && navRow.hoverText !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: navRow.hoverText
}
