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
    /// This column is a name box right now, and which of the three it asks for ("branch" / "tag" / "rename"). One
    /// field for all: the two that make a name ask in the placeholder, a rename opens holding the name.
    required property bool naming
    required property string namingMode
    /// What a rename box is naming ("branch" / "remote" / "tag"), for the frame's tone; empty while making a name.
    required property string namingKind
    /// Whether what is typed can be accepted, and why not — the page's answer (デザイン規約 §可否・警告の出し場所).
    required property bool namingRefused
    required property string namingRefusedWhy

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
        field.inkText = chipColumn.namingMode === "rename" ? field.text : field.placeholderText
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
            /// The narrowest the box goes: the whole of what it opened holding (the placeholder, or a rename's name) —
            /// cut, a placeholder asks nothing and a name cannot be checked before Enter. Measured off a never-drawn
            /// label, since `TextMetrics` comes out a few pixels tight and a tight floor still elides; plus the
            /// field's paddings, which Controls takes out of `width` before laying the placeholder out.
            readonly property real nameBoxMinW:
                Math.ceil(placeholderInk.implicitWidth) + nameField.leftPadding + nameField.rightPadding
            /// What the box must be able to show, latched as it opens — measured per keystroke, the right edge would
            /// walk out from under the caret.
            property string inkText: ""
            width: Math.max(chipColumn.columnWidth - 2 * Theme.spaceXs, nameField.nameBoxMinW)
            // The frame names the kind (デザイン規約 §ref の種別「枠 = 種別」); a branch's colour is the focus ring's own,
            // so only a tag differs.
            focusTone: (chipColumn.namingMode === "rename" ? chipColumn.namingKind : chipColumn.namingMode) === "tag"
                       ? Theme.refTag : Theme.borderFocus
            // None for a rename, whose box is for the name it opened holding (デザイン規約 §左メニューの所作).
            placeholderText: chipColumn.namingMode === "rename" ? ""
                           : chipColumn.namingMode === "tag" ? qsTr("Create tag here?")
                                                             : qsTr("Create branch here?")
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
            // The floor's ruler, never drawn: the field's own placeholder cannot be measured before the box has a
            // width.
            Label {
                id: placeholderInk
                visible: false
                text: nameField.inkText
                font: nameField.font
            }
        }
    }
    // Nothing here is hovered on its own: the row's MouseArea, declared after this, takes every hover
    // (デザイン規約 §hover のツールチップ); stacked names are read in the RefListPopup it opens.
}
