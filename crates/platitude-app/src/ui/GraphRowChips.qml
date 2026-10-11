pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The chip column of one commit-graph row: the names this commit carries, or the box that puts one there. The row owns
// the gestures; this owns what is drawn. The box is built only while open, the chip on every row
// (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
Item {
    id: chipColumn

    required property var records
    /// The column's width, read instead of `width`, which a layout settles a frame later.
    required property real columnWidth
    /// Waiting out the double-click window after a second click (`RefChip.waiting`).
    required property bool waiting
    /// This column is a name box right now, and which of the four it asks for ("branch" / "tag" / "worktree" /
    /// "rename"). One field for all: the placeholder names the kind being typed (a rename rubbed out shows it too), a
    /// rename opens holding the name.
    required property bool naming
    required property string namingMode
    /// What a rename box is naming ("branch" / "remote" / "tag"), for the frame's tone and the placeholder; empty
    /// while making a name.
    required property string namingKind
    /// Whether what is typed can be accepted, and why not — the page's answer (デザイン規約 §可否・警告の出し場所).
    required property bool namingRefused
    required property string namingRefusedWhy
    /// A worktree's folder in that line, empty for none (`RepoPage.graphNameRefusedMark`).
    required property string namingRefusedMark

    /// The chip's card is standing on it (`GraphRowDelegate.listOnThisChip`): the sheets unstack so none peeks out
    /// from under the card.
    required property bool listOpen

    /// Where the chip's card stands: the whole stack, not the front card — the card has to cover the fan too
    /// (`RefListPopup.coverWidth`).
    readonly property alias chipItem: rowStack
    /// For the runs: the wait mark as the chip wears it (`RefChip.waiting`) — read off the chip, so a cut binding
    /// shows.
    readonly property alias chipWaiting: rowStack.chipWaiting
    /// The name box's width (the column, or its floor where wider); 0 without a box. Read by the row and by
    /// `PGG_AUTO_ACT=name-box`.
    readonly property real nameBoxWidth: nameSeat.item ? nameSeat.item.width : 0

    signal namingSubmitted(string name)
    signal namingEdited(string text)
    signal namingCancelled()

    /// Whether this column shows a name at all — all the row needs to divide itself (`GraphRowDelegate.partAt`, on the
    /// column's edge, not the chip's frame). A single chip unfolds into the same card as a stack: it is cut too.
    readonly property bool hasChip: rowStack.visible

    /// What the box opens holding, kept here for whichever of the ask and the box lands second (`focusBox`).
    property string askedText: ""
    /// Opens the box on `text`, held on the view since this delegate is recycled. A rename opens with the name selected
    /// whole, to be typed over (デザイン規約 §左メニューの所作).
    function takeNamingFocus(text) {
        chipColumn.askedText = text === undefined ? "" : text
        chipColumn.focusBox()
    }
    /// Puts `askedText` in the box and the keyboard on it. The box's build and the ask come in no fixed order, so both
    /// call here (`onLoaded` and the ask) and whichever is second finds both in place.
    function focusBox() {
        const field = nameSeat.item
        if (field === null || !chipColumn.naming)
            return
        field.text = chipColumn.askedText
        field.inkText = chipColumn.namingMode === "rename" ? field.text : ""
        if (chipColumn.namingMode === "rename")
            field.selectAll()
        field.forceActiveFocus()
    }

    // The open box can reach past the column onto the lane cell, which is laid out after this column and would draw
    // over it.
    z: chipColumn.naming ? 1 : 0

    RefChipStack {
        id: rowStack
        // Assigning `visible` replaces the chip's own rule, so "has anything to show" is repeated here, or a row with
        // no refs draws an empty frame. Hidden rather than unbuilt (see the top).
        visible: chipColumn.records.length > 0 && !chipColumn.naming
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        // Centres the card and its fan together (`RefChipStack.lift`); on the card alone a deep stack sits on the
        // row's floor.
        anchors.verticalCenterOffset: -rowStack.lift
        records: chipColumn.records
        waiting: chipColumn.waiting
        unstacked: chipColumn.listOpen
        // The names the chip cannot fit are read in the card.
        maxWidth: chipColumn.columnWidth - Theme.spaceSm
    }
    // A double-click on a row with nowhere to switch to asks for a name, where the chips would be
    // (デザイン規約 §可否・警告の出し場所「表示の切り替えで足りるならそれで済ませる」).
    Loader {
        id: nameSeat
        active: chipColumn.naming
        // Anchored by the head and grown into the graph (規約 §グラフ行のダブルクリック「開く向きは 1 つ」).
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        onLoaded: chipColumn.focusBox()
        sourceComponent: SlimField {
            id: nameField
            /// The narrowest the box goes: the whole of the placeholder and of what it opened holding (a rename's
            /// name) — cut, neither says what it is. Measured off never-drawn labels, since `TextMetrics` comes out a
            /// few pixels tight and a tight floor still elides; plus the field's paddings, which Controls takes out of
            /// `width` before laying the placeholder out.
            readonly property real nameBoxMinW:
                Math.ceil(Math.max(placeholderInk.implicitWidth, openedInk.implicitWidth))
                + nameField.leftPadding + nameField.rightPadding
            /// The name a rename box opened holding, latched — measured per keystroke, the right edge would walk out
            /// from under the caret. Empty for the boxes that make a name, even rebuilt holding what was typed.
            property string inkText: ""
            /// A worktree's folder in the refusal's line, which the shared tip stands the tree mark in front of
            /// (`SharedToolTip.tipMarkWord`, デザイン規約 §ref の種別「名前の印」).
            readonly property string tipMarkWord: chipColumn.namingRefusedMark
            /// The kind of ref being typed: the mode, or on a rename the kind it renames.
            readonly property string namesKind:
                chipColumn.namingMode === "rename" ? chipColumn.namingKind : chipColumn.namingMode
            width: Math.max(chipColumn.columnWidth - 2 * Theme.spaceXs, nameField.nameBoxMinW)
            // The frame names the kind (デザイン規約 §ref の種別「枠 = 種別」); a branch's colour is the focus ring's own,
            // so a tag differs, and so does a branch made out in another worktree — that worktree's green, as its chip
            // wears.
            focusTone: nameField.namesKind === "tag" ? Theme.refTag
                     : nameField.namesKind === "worktree" ? Theme.success : Theme.borderFocus
            // What goes in the box, by kind: a new worktree and a remote's are branch names too
            // (デザイン規約 §グラフ行のダブルクリック「箱のプレースホルダ」).
            placeholderText: nameField.namesKind === "tag" ? qsTr("Tag name") : qsTr("Branch name")
            refused: chipColumn.namingRefused
            ToolTip.visible: nameField.refused && nameField.activeFocus && chipColumn.namingRefusedWhy !== ""
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: chipColumn.namingRefusedWhy
            // Refused text stays in the box; Enter does nothing, and the frame and tooltip say why.
            onAccepted: {
                if (!chipColumn.namingRefused)
                    chipColumn.namingSubmitted(nameField.text.trim())
            }
            // Held on the view, which outlives this recycled delegate.
            onTextEdited: chipColumn.namingEdited(nameField.text)
            Keys.onEscapePressed: chipColumn.namingCancelled()
            // The floor's rulers, never drawn: the field's own placeholder cannot be measured before the box has a
            // width.
            Label {
                id: placeholderInk
                visible: false
                text: nameField.placeholderText
                font: nameField.font
            }
            Label {
                id: openedInk
                visible: false
                text: nameField.inkText
                font: nameField.font
            }
        }
    }
    // Nothing here is hovered on its own: the row's MouseArea, declared after this, takes every hover
    // (デザイン規約 §hover のツールチップ); stacked names are read in the RefListPopup it opens.
}
