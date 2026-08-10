pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One sidebar row: ref / file / folder, shared by every section and the
// WIP file list. What a click means is the owner's business — the row
// only reports it.
Item {
    id: navRow
    required property int index
    required property string name
    required property string full
    required property string oid_hex
    required property string change
    required property string bucket
    required property string orig_path
    required property bool is_head
    required property bool has_remote
    required property bool only_remote
    required property bool has_pr
    required property int depth
    required property bool folder
    required property bool eol_mark
    /// The pointed row's line-ending words, handed down by the pane — the
    /// section model keeps one copy rather than every row keeping its own.
    property string pointedEolPath: ""
    property string pointedEolKind: ""
    property string pointedEolFrom: ""
    property string pointedEolTo: ""
    property int pointedEolLines: 0
    property string pointedEolScope: ""
    property string pointedEolExt: ""
    /// The pointer arrived at, or left, a row carrying the mark.
    signal eolPointed(string path, bool on)
    property string kindHint: "branch"
    property string headTrack: ""
    property real listWidth: 200
    // Shows the hover stage/unstage affordance (WIP view).
    property bool showStage: false
    /// Whether this row is one of those chosen (working-tree list). Held
    /// by the list, since a delegate is recycled the moment its row
    /// scrolls off.
    property bool chosen: false
    /// The pointer is on another chosen row's stage affordance, and this
    /// row goes with it. The marks come out together so that what one
    /// press moves is seen before it is pressed
    /// (デザイン規約 §その他の操作).
    property bool stagePeer: false
    /// What each side of a conflict is called. **The two swap over during
    /// a rebase**, so they are handed down from the model rather than
    /// worked out here (`WorkTreeModel.sideOurs` / `sideTheirs`); empty
    /// where git left nothing to name a side by.
    property string sideOurs: ""
    property string sideTheirs: ""

    /// What the two sides each did to this file (デザイン規約 §conflict
    /// の種別). The sentence itself lives in `Words`: the diff pane says
    /// the same one on the conflicts git prints no patch for, and two
    /// copies of it would be two answers.
    function conflictWords() {
        return Words.conflict(navRow.change, navRow.sideOurs, navRow.sideTheirs)
    }

    // ---- the two-click gestures ------------------------------------
    // Which row was clicked last, and which is being typed into, are held
    // by the sidebar: delegates are recycled the moment a row scrolls off
    // (デザイン規約 §左メニューの所作).
    property string rowKey: ""
    property string activeKey: ""
    property string editKey: ""
    /// "rename" (the name is in the box) or "branch" (a name for a new
    /// branch on this row's commit).
    property string editMode: ""
    /// What has been typed so far, held by the sidebar so a row that
    /// scrolls off and comes back does not lose it.
    property string editText: ""
    property bool editRefused: false
    property string editRefusedWhy: ""
    readonly property bool editing: navRow.editKey !== ""
                                    && navRow.editKey === navRow.rowKey
    /// The name git knows this row by.
    readonly property string fullName: navRow.full !== "" ? navRow.full : navRow.name

    signal refClicked(string oidHex)
    /// A working-tree file row was clicked. `modifiers` carries Ctrl and
    /// Shift, which is how several rows are chosen at once.
    signal fileClicked(string bucket, string path, string origPath, int modifiers)
    signal folderClicked(string key)
    signal stageClicked(string bucket, string path)
    /// The pointer arrived at (or left) this row's stage affordance.
    signal stageHovered(string bucket, string path, bool on)
    /// A left click landed on this row, whatever it then meant.
    signal rowClicked()
    /// Double-click: go where this row leads.
    signal activateRequested()
    /// A second click, once the double-click window has passed: put this
    /// row's name in a box.
    signal renameRequested()
    /// The box: typed into, accepted, walked away from.
    signal editTyped(string text)
    signal editAccepted(string text)
    signal editCancelled()
    /// Right-click on a ref row; the page owns the menu because
    /// delegates are recycled out from under an open popup. `name` is
    /// what the row shows, `full` what git knows it by (a stash shows a
    /// message and answers to a selector).
    signal refMenuRequested(string name, string full, string oidHex)
    /// Right-click on a working-tree file row, for the same reason.
    /// Undoing a rename takes both of its names, but the menu reads them
    /// off the chosen rows (`orig_path`), so the row itself is enough.
    signal fileMenuRequested(string bucket, string path)

    width: listWidth
    height: Theme.rowHeight

    // The current branch stays highlighted inside the list (the sidebar's
    // sticky row only stands in for it while this row is scrolled off).
    Rectangle {
        anchors.fill: parent
        color: Theme.accentMuted
        visible: navRow.is_head && !navRow.folder
    }
    // The row a click last landed on. Without it the second click of the
    // rename gesture would be aimed at nothing, and a click on a worktree
    // row — which has nowhere to jump to — would look like it missed.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgSelected
        visible: !navRow.folder
                 && (navRow.chosen
                     || (navRow.rowKey !== ""
                         && navRow.activeKey === navRow.rowKey))
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: itemMouse.containsMouse
    }
    // No mark: nothing asks a question about a row in this list any more.
    // What one of these rows takes away is held down on the menu row that
    // names it, and that menu is standing over the row while it is held
    // (デザイン規約 §長押し).
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd + navRow.depth * Theme.spaceMd
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceXs
        // Every row opens with this slot, held open even when empty, so
        // that at a given depth all names begin in the same column: a
        // folder's fold arrow, a worktree file's change icon, and later
        // the mark for a hidden branch all live here. Letting the slot
        // collapse is what put a leaf's name to the *left* of the folder
        // it sits under (layouts drop invisible children entirely).
        Item {
            Layout.preferredWidth: Theme.iconMd
            Layout.preferredHeight: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
            NavIcon {
                anchors.centerIn: parent
                visible: navRow.folder
                width: Theme.iconSm
                height: Theme.iconSm
                kind: "chevron"
                // A folder row has no change code, so it keeps its fold
                // state in that field (`models::nav::FOLDED`).
                rotation: navRow.change === "FOLDED" ? 0 : 90
                tint: Theme.textSecondary
            }
            ChangeIcon {
                anchors.fill: parent
                visible: !navRow.folder && navRow.kindHint === "wt"
                change: navRow.change
            }
        }
        // The name says where the ref is, the way a chip's does: grey for
        // one this repository does not hold (デザイン規約 §ref の種別).
        Label {
            id: nameLabel
            visible: !navRow.editing
            Layout.fillWidth: true
            text: navRow.name
            elide: Text.ElideMiddle
            font.weight: navRow.is_head ? Font.DemiBold : Font.Normal
            color: navRow.folder ? Theme.textSecondary
                   : navRow.is_head ? Theme.textLink
                   : navRow.only_remote ? Theme.textSecondary : Theme.textPrimary
            font.pixelSize: Theme.fontMd
            // A pending file whose change says something about its line
            // endings. On the name's shoulder, the way a button in trouble
            // wears one (`ActionButton.alert`) — **outside the layout**, so
            // no row moves and the stage button's seat at the right edge
            // stays its own. What it is about is the row's hover; the
            // sentence in full is the diff pane's.
            NavIcon {
                visible: navRow.kindHint === "wt" && navRow.eol_mark
                kind: "bang"
                tint: Theme.warning
                width: Theme.iconSm
                height: Theme.iconSm
                // Clamped: an elided name is wider than its box, and the
                // mark belongs to the name that is on screen. Half a gap
                // back into the name's own trailing bearing, so it reads
                // as part of the word rather than as the next column.
                x: Math.min(nameLabel.implicitWidth - Theme.spaceXs / 2,
                            nameLabel.width - Theme.iconSm)
                y: 0
            }
        }
        // The name, in a box, where the name was. Nothing is asked before
        // it opens or when it is walked away from: what it costs is the
        // typing (デザイン規約 §可否・警告の出し場所).
        SlimField {
            id: editField
            visible: navRow.editing
            Layout.fillWidth: true
            font.pixelSize: Theme.fontMd
            refused: navRow.editRefused
            placeholderText: navRow.editMode === "branch"
                             ? qsTr("Create branch here?") : ""
            onTextEdited: navRow.editTyped(editField.text)
            // Refused text stays in the box: Enter that does nothing is
            // the answer, and the frame and its tooltip say why.
            onAccepted: {
                if (!navRow.editRefused)
                    navRow.editAccepted(editField.text)
            }
            Keys.onEscapePressed: navRow.editCancelled()
            ToolTip.visible: navRow.editRefused && editField.activeFocus
                             && navRow.editRefusedWhy !== ""
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: navRow.editRefusedWhy
        }
        // Worktree rows: checked-out branch on the right.
        Label {
            visible: !navRow.folder && navRow.kindHint === "worktree"
            text: navRow.bucket !== "" ? navRow.bucket : qsTr("detached")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            elide: Text.ElideMiddle
            Layout.maximumWidth: navRow.listWidth / 2
        }
        // Current branch's ahead/behind, left of the state icon.
        Label {
            visible: !navRow.folder && navRow.kindHint === "branch"
                     && navRow.is_head && navRow.headTrack !== ""
            text: navRow.headTrack
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        // Branch remote state: nothing = local only, remote icon =
        // has a remote, PR icon = has a PR (real data in Phase 4;
        // PG_FAKE_PR previews the look). Remote-branch and worktree
        // rows show the PR state too. A tag reads the same way — the
        // badge answers "is this only here?" whatever it is on, and
        // the fetch carries the bit for it (`ls-remote --tags`).
        NavIcon {
            visible: !navRow.folder
                     && (((navRow.kindHint === "branch"
                           || navRow.kindHint === "tag")
                          && (navRow.has_remote || navRow.has_pr))
                         || ((navRow.kindHint === "remote"
                              || navRow.kindHint === "worktree")
                             && navRow.has_pr))
            kind: navRow.has_pr ? "pr" : "remote"
            tint: navRow.has_pr ? Theme.success : Theme.textSecondary
            // The size the graph's chips wear the same badge at: one
            // question, one mark, one size (デザイン規約 §寸法).
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
    // Rows whose name can be changed from here. A remote branch is one of
    // them even though git has no rename over there — core builds the
    // rename out of a push and a delete, and the bar asks before it runs.
    // A folder is not: it is the shape of the names below it, not a name.
    readonly property bool nameable: !navRow.folder
        && (navRow.kindHint === "branch" || navRow.kindHint === "tag"
            || navRow.kindHint === "stash" || navRow.kindHint === "remote")
    // Long enough that the second click of a double-click falls inside
    // it; the system's own setting, since it is the system that decides
    // what counts as a double-click.
    Timer {
        id: doubleGuard
        interval: Application.styleHints.mouseDoubleClickInterval
    }
    // A second click on a row already clicked means the name, but only
    // once a double-click can be ruled out — the same wait Explorer makes
    // (デザイン規約 §左メニューの所作).
    Timer {
        id: renameTimer
        interval: Application.styleHints.mouseDoubleClickInterval
        onTriggered: navRow.renameRequested()
    }
    // The box has to carry the name into itself when it opens, and again
    // when a scrolled-off row is built anew (the delegate is recycled; the
    // text is not this row's to keep).
    onEditingChanged: navRow.takeEditFocus()
    Component.onCompleted: navRow.takeEditFocus()
    function takeEditFocus() {
        if (!navRow.editing)
            return
        editField.text = navRow.editText
        editField.selectAll()
        editField.forceActiveFocus()
    }
    function ordinaryClick(modifiers) {
        if (navRow.folder) {
            navRow.folderClicked(navRow.full)
        } else if (navRow.kindHint === "wt") {
            navRow.fileClicked(navRow.bucket, navRow.fullName, navRow.orig_path,
                               modifiers === undefined ? Qt.NoModifier : modifiers)
        } else if (navRow.kindHint === "worktree") {
            // Another repository: nothing here to jump to, so a click only
            // takes the row (the double-click opens it as a tab).
        } else if (navRow.oid_hex !== "") {
            navRow.refClicked(navRow.oid_hex)
        }
    }
    MouseArea {
        id: itemMouse
        anchors.fill: parent
        hoverEnabled: true
        // While the box is open the row belongs to it.
        visible: !navRow.editing
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // The row that carries the mark tells the model it is the one
        // being read, so the sentence can be built for it alone. The row
        // itself has no field left to hold it (`NavItem::eol_mark`).
        onContainsMouseChanged: {
            if (navRow.eol_mark)
                navRow.eolPointed(navRow.fullName, itemMouse.containsMouse)
        }
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton) {
                // Only rows with operations behind them open a menu.
                if (!navRow.folder && navRow.oid_hex !== ""
                        && (navRow.kindHint === "branch"
                            || navRow.kindHint === "remote"
                            || navRow.kindHint === "tag"
                            || navRow.kindHint === "stash"))
                    navRow.refMenuRequested(navRow.name, navRow.fullName,
                                            navRow.oid_hex)
                else if (!navRow.folder && navRow.kindHint === "wt")
                    navRow.fileMenuRequested(navRow.bucket, navRow.fullName)
                return
            }
            // The second click of a double-click: the first one already
            // did what a click does, and the gesture is the double.
            if (doubleGuard.running)
                return
            const wasActive = navRow.activeKey === navRow.rowKey
            doubleGuard.restart()
            navRow.rowClicked()
            navRow.ordinaryClick(mouse.modifiers)
            if (wasActive && navRow.nameable)
                renameTimer.restart()
        }
        onDoubleClicked: mouse => {
            if (mouse.button !== Qt.LeftButton || navRow.folder)
                return
            renameTimer.stop()
            navRow.activateRequested()
        }
    }
    // Hover stage/unstage affordance.
    HoverToolButton {
        id: stageButton
        visible: navRow.showStage && !navRow.folder
                 && (itemMouse.containsMouse || hovered || navRow.stagePeer)
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        padding: 0
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        // The list is told which row the pointer is on, so the rows that
        // would go with it can put their own marks out.
        onHoveredChanged: navRow.stageHovered(
            navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name,
            stageButton.hovered)
        ToolTip.visible: hovered
        ToolTip.delay: Metrics.tipDelayMs
        // On a conflicted row the same `git add` means something else:
        // it does not move a change into the staging area, it tells git
        // the conflict has been dealt with. The word says that rather
        // than the command's other job (デザイン規約 §diff の中のステージ).
        ToolTip.text: navRow.bucket === "conflicts" ? qsTr("Mark resolved")
                      : navRow.bucket === "staged" ? qsTr("Unstage file")
                                                   : qsTr("Stage file")
        onClicked: navRow.stageClicked(
            navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name)
        contentItem: NavIcon {
            kind: navRow.bucket === "staged" ? "minus" : "plus"
            tint: navRow.bucket === "staged" ? Theme.diffRemovedFg
                                             : Theme.diffAddedFg
        }
    }
    // Hover says where this row leads — the one thing the row itself
    // cannot show (デザイン規約 §hover のツールチップ). A name that the
    // sentence carries doubles as the full name of a nested leaf, which
    // is why the wording always spells the row out; where there is no
    // sentence to write, the full name stands on its own. Local branches
    // and tags have no sentence to write: the section names the kind and
    // the gesture is the same on every row of it, so `Switch to <name>`
    // and `Create a branch at <name>` only read the label back
    // (§hover のツールチップ「足すものが 1 つも無い的には、何も出さない」)
    // — a shortened name is all that is left to say, the same answer the
    // row the working tree is on already gave. That one keeps its colour
    // here too, and colouring means rich text, so the name is escaped
    // (a refname may hold & and <).
    readonly property string hoverText: {
        const full = navRow.fullName
        if (navRow.folder)
            return ""
        if (navRow.kindHint === "branch" && navRow.is_head)
            return full === navRow.name ? ""
                   : "<font color=\"" + Theme.textLink + "\">"
                     + navRow.escapeMarkup(full) + "</font>"
        if (navRow.kindHint === "remote")
            return qsTr("Switch to %1").arg(full)
        if (navRow.kindHint === "worktree")
            return qsTr("Open %1 in a new tab").arg(full)
        // A stash is named by a message that the row has to cut short.
        if (navRow.kindHint === "stash")
            return navRow.name
        // A row carrying the line-ending mark has a card of its own, which
        // names the path as its first line — two things opening off one
        // pointer would sit on top of each other.
        if (navRow.eolPointedAt)
            return ""
        return full !== navRow.name ? full : ""
    }
    /// Whether the words on the model are this row's. Only one row can be
    /// pointed at, so they are kept once there rather than on every row
    /// (`NavSectionModel::point_eol`), and the row has to check that the
    /// answer is its own before reading it.
    readonly property bool eolPointedAt: navRow.eol_mark
        && navRow.pointedEolPath === navRow.fullName
    /// The sentence for this row, when the pointer is on it.
    readonly property string eolText:
        navRow.eolPointedAt && navRow.pointedEolKind !== ""
        ? Words.lineEndings(navRow.pointedEolKind, navRow.pointedEolFrom,
                            navRow.pointedEolTo, navRow.pointedEolLines,
                            navRow.pointedEolScope, navRow.pointedEolExt)
        : ""
    function escapeMarkup(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;")
                   .replace(/>/g, "&gt;")
    }
    ToolTip.visible: itemMouse.containsMouse && !navRow.editing
                     && navRow.hoverText !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: navRow.hoverText
}
