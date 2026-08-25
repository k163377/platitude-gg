pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The chip column of one commit-graph row: the names this commit carries, or — on a row that carries none — the box
// that puts one there. The row owns the gestures; this owns what is drawn and what is under a point.
Item {
    id: chipColumn

    /// The chip's records, in the order it reads them out (see the row).
    required property var records
    /// How wide the column is. The chip and the box are laid out from it rather than from `width`, which a layout
    /// settles a frame later.
    required property real columnWidth
    /// This column is a name box right now, and which of the three names it is asking for ("branch" / "tag" /
    /// "rename"). One field for all of them — what is typed is a ref name either way — and the question it holds is
    /// the whole of the difference: the two that make a name ask it in the placeholder, and the one that changes a
    /// name opens holding the name (デザイン規約 §左メニューの所作「入力欄は行の名前の位置に出す」).
    required property bool naming
    required property string namingMode
    /// What a rename box is naming ("branch" / "remote" / "tag"), for the frame to say the kind the way the chip it
    /// stands in for does (§ref の種別: 枠 = 種別). Empty while the box is making a name rather than changing one.
    required property string namingKind
    /// Whether what is typed can be accepted, and the one line that says why not. Both the page's answer — the row
    /// only draws it (§可否・警告の出し場所: the frame says so where the typing is, the reason waits in the tooltip).
    required property bool namingRefused
    required property string namingRefusedWhy

    /// The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: rowChip
    /// What the name box came out to — the column, or its own floor where that is wider. The row reads it for the
    /// ground the box is standing on, and a headless run (`PG_AUTO_ACT=name-box`) to say which of the two it got.
    readonly property alias nameBoxWidth: nameField.width

    signal namingSubmitted(string name)
    signal namingEdited(string text)
    signal namingCancelled()

    /// Whether there is a name in this column at all — the whole of what the row needs to divide itself
    /// (`GraphRowDelegate.partAt`).
    ///
    /// **Not "which chip is under this point".** The row divides on this column's own edge rather than on the chip's
    /// frame, so the geometry belongs to the row, which is where the column's width already lives. A frame eighteen
    /// pixels tall in a row of twenty-eight is a boundary nobody can see (2026-08-21 ユーザー報告).
    ///
    /// **Every chip answers, stacked or not** (2026-08-21 ユーザー判断): a chip with one name on it is cut to the column
    /// just the same, and a name that cannot be read is a name that cannot be read — the reason a stack unfolds is the
    /// reason a single one does. What comes out is the same card either way.
    readonly property bool hasChip: rowChip.visible
    /// The narrowest the box goes: the whole of what it opened holding. The placeholder is all there is to say what a
    /// box that makes a name is for, so a cut one (`Create branch he…`) asks nothing — and the column is narrower than
    /// that at every width up to and including its default (2026-08-21 ユーザー指示). A rename measures the name instead:
    /// there the box is the name's own seat, and one cut short is a name the reader cannot check before Enter.
    ///
    /// Measured off a label that is never drawn, the way the diff's number column measures (app-ui.md): `TextMetrics`
    /// comes out a few pixels tighter than the label the words are actually set in, and a floor measured tight is a
    /// floor that still elides. Both terms come off the field itself, because it is the field's own placeholder that
    /// has to fit: Controls lays it out in `width` less the two paddings and elides whatever is left over.
    readonly property real nameBoxMinW:
        Math.ceil(placeholderInk.implicitWidth) + nameField.leftPadding + nameField.rightPadding
    /// What the box has to be able to show: the question where it is asking one, the name where it opened holding one.
    /// **Latched as the box opens rather than followed as it is typed into** — a box whose right edge walks out from
    /// under the caret is one nobody can aim at (`NavNameBox` measures its own the same way).
    property string inkText: ""
    /// Carries the box on from whatever the last delegate to hold it was left with. `text` comes off the view, not off
    /// this column: this delegate is recycled the moment the row scrolls off.
    ///
    /// **A rename opens with the name in it, selected whole** (デザイン規約 §左メニューの所作) — the box is the name's own seat,
    /// so what it comes up holding is what is being changed, and typing over it is the first thing a hand does. The
    /// two boxes that make a name come up empty and have nothing to select.
    function takeNamingFocus(text) {
        nameField.text = text === undefined ? "" : text
        chipColumn.inkText = chipColumn.namingMode === "rename" ? nameField.text
                                                                : nameField.placeholderText
        if (chipColumn.namingMode === "rename")
            nameField.selectAll()
        nameField.forceActiveFocus()
    }

    // The open box reaches past the column's edge on any width under its floor, and the lane cell is laid out after
    // this column — so without this the strokes would be drawn over it.
    z: chipColumn.naming ? 1 : 0

    RefChip {
        id: rowChip
        // Assigning `visible` here replaces the chip's own rule, so the "has anything to show" half has to be repeated:
        // without it a row with no refs draws an empty frame.
        visible: chipColumn.records.length > 0 && !chipColumn.naming
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        records: chipColumn.records
        // The names the chip cannot fit are read in the card, not squeezed here.
        maxWidth: chipColumn.columnWidth - Theme.spaceSm
    }
    // A row with nothing to move to answers the double-click with the one thing that would give it something: a name.
    // The question is asked where the chips would be, not over the window (デザイン規約: 表示の切り替えで足りるならダイアログを出さない).
    SlimField {
        id: nameField
        visible: chipColumn.naming
        // Anchored by the head, which is the edge that holds still, and grown the one way from there — into the graph
        // (規約 §グラフ行のダブルクリック「開く向きは 1 つ」). The card a chip unfolds into takes the same ground for the same
        // reason: what covers the lanes is out only while a hand is on it, and this box is not read at all if the
        // question on it cannot be.
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        // The column, until the column is narrower than the question. Left at the slim field's own size, which is the
        // chip's: what is typed here becomes the chip that stands in this column, so it is read at the size it will be
        // read at — but the box is an offer before it is a name, and one that cannot be read is not an offer.
        width: Math.max(chipColumn.columnWidth - 2 * Theme.spaceXs, chipColumn.nameBoxMinW)
        // The frame says which kind is being named — the same rule the chip this box turns into follows
        // (§ref の種別: 枠 = 種別). A branch's colour is the focus ring's own value, so only the tag has one to say, and a
        // rename is told by the kind it is changing rather than by the mode.
        focusTone: (chipColumn.namingMode === "rename" ? chipColumn.namingKind : chipColumn.namingMode) === "tag"
                   ? Theme.refTag : Theme.borderFocus
        // A rename has none: what the box is for is the name it opened holding (§左メニューの所作), and a question written
        // over that name would be answering for the reader.
        placeholderText: chipColumn.namingMode === "rename" ? ""
                       : chipColumn.namingMode === "tag" ? qsTr("Create tag here?")
                                                         : qsTr("Create branch here?")
        refused: chipColumn.namingRefused
        ToolTip.visible: nameField.refused && nameField.activeFocus && chipColumn.namingRefusedWhy !== ""
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: chipColumn.namingRefusedWhy
        // Refused text stays in the box: Enter that does nothing is the answer, and the frame and its tooltip say why
        // (§可否・警告の出し場所).
        onAccepted: {
            if (!chipColumn.namingRefused)
                chipColumn.namingSubmitted(nameField.text.trim())
        }
        // Held on the view, not here: this delegate is recycled the moment the row scrolls off, and half a name is
        // still worth not losing.
        onTextEdited: chipColumn.namingEdited(nameField.text)
        Keys.onEscapePressed: chipColumn.namingCancelled()
    }
    // The box's floor, measured. Never drawn — it stands in for what the field lays out inside itself, which cannot be
    // measured before the box it is being measured for has a width.
    Label {
        id: placeholderInk
        visible: false
        text: chipColumn.inkText
        font: nameField.font
    }
    // Nothing in this column can be hovered on its own: the row's MouseArea fills the row and is declared after it, so
    // it takes every hover the chips would have seen (デザイン規約 §hover の ツールチップ). What the stacked chips hold is read from
    // the RefListPopup that MouseArea opens under the pointer.
}
