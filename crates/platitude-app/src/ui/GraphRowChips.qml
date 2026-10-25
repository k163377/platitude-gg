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
    /// What a rename box is naming ("branch" / "remote" / "tag" / "worktreeFolder"), for the frame's tone and the
    /// placeholder; empty while making a name.
    required property string namingKind
    /// The name a rename box came up holding (`GraphList.namingOpenedWith`): its least width, and whether a rebuilt
    /// box holds the name still or what was typed over it.
    required property string namingOpenedWith
    /// The farthest the box may reach to the right, in this column's frame: the cards' `spaceXxl` stop, measured from
    /// the pane's edge, where the list clips.
    required property real boxReach
    /// Why the spaced second click on this row's chip opened no box (a worktree's lock or gone folder —
    /// `GraphPane.withholdRename`), empty for none: said on the chip, as a refused name's reason is on the box.
    required property string withheldWhy
    /// The worktree's folder in that line, behind the tree mark (デザイン規約 §ref の種別「名前の印」).
    required property string withheldMark
    /// Whether what is typed can be accepted, and why not — the page's answer (デザイン規約 §可否・警告の出し場所).
    required property bool namingRefused
    required property string namingRefusedWhy
    /// A worktree's folder in that line, empty for none (`RepoPage.graphNameRefusedMark`).
    required property string namingRefusedMark
    /// The branch, tag or remote in that line, behind their marks (`RepoPage.graphNameRefusedMarks`).
    required property var namingRefusedMarks

    /// The chip's card is standing on it (`GraphRowDelegate.listOnThisChip`): the sheets unstack so none peeks out
    /// from under the card.
    required property bool listOpen

    /// Where the chip's card stands: the whole stack, not the front card — the card has to cover the fan too
    /// (`RefListPopup.coverWidth`).
    readonly property alias chipItem: rowStack
    /// For the runs: the wait mark as the chip wears it (`RefChip.waiting`) — read off the chip, so a cut binding
    /// shows.
    readonly property alias chipWaiting: rowStack.chipWaiting
    /// The name box's left edge and width in this column's frame; 0 without a box. Read by the row, whose presses stop
    /// at the box, and by `PGG_AUTO_ACT=name-box` / `graph-rename-fit`.
    readonly property real nameBoxX: nameSeat.item ? nameSeat.x : 0
    readonly property real nameBoxWidth: nameSeat.item ? nameSeat.item.width : 0
    /// What the box says it takes, rubbed out (PGG_AUTO_ACT=worktree-rename-box).
    readonly property string nameBoxPlaceholder: nameSeat.item ? nameSeat.item.placeholderText : ""
    /// Whether the box holds any of its words selected (PGG_AUTO_ACT=graph-rename-fit).
    readonly property bool nameBoxSelected: nameSeat.item ? nameSeat.item.selectedText !== "" : false
    /// The column's room for the box: its width less the `spaceXs` the chip keeps off each side.
    readonly property real boxRoom: chipColumn.columnWidth - 2 * Theme.spaceXs

    signal namingSubmitted(string name)
    signal namingEdited(string text)
    signal namingCancelled()

    /// Whether this column shows a name at all — all the row needs to divide itself (`GraphRowDelegate.partAt`, on the
    /// column's edge, not the chip's frame). A single chip unfolds into the same card as a stack: it is cut too.
    readonly property bool hasChip: rowStack.visible

    /// What the box opens holding, kept here for whichever of the ask and the box lands second (`focusBox`).
    property string askedText: ""
    /// Opens the box on `text`, held on the view since this delegate is recycled. A rename opens with the name selected
    /// whole, to be typed over (デザイン規約 §左メニューの所作); a box rebuilt holding what was typed over it takes the
    /// caret at its end.
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
        if (chipColumn.namingMode === "rename" && field.text === chipColumn.namingOpenedWith)
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
        // Ends where the chip ended while the column holds it, and from there grows to the left; once it outgrows the
        // column the head stays at the column's and the box grows into the graph (規約 §グラフ行のダブルクリック
        // 「改名の箱は名前の幅」/「開く向きは 1 つ」).
        x: Theme.spaceXs + Math.max(0, chipColumn.boxRoom - (nameSeat.item ? nameSeat.item.width : 0))
        anchors.verticalCenter: parent.verticalCenter
        onLoaded: chipColumn.focusBox()
        sourceComponent: SlimField {
            id: nameField
            /// The words' widths, paddings included, which Controls takes out of `width` before laying the text and
            /// the placeholder out. The placeholder and the name the box opened holding are measured off never-drawn
            /// labels, since `TextMetrics` comes out a few pixels tight and a tight width still elides; what is typed
            /// is the field's own `contentWidth`, the width at which it stands without scrolling.
            readonly property real placeholderW: Math.ceil(placeholderInk.implicitWidth) + nameField.padsW
            readonly property real openedW: Math.ceil(openedInk.implicitWidth) + nameField.padsW
            readonly property real typedW: Math.ceil(nameField.contentWidth) + nameField.padsW
            readonly property real padsW: nameField.leftPadding + nameField.rightPadding
            /// What the box asks for. A box making a name takes the column, and the placeholder whole where that is
            /// wider. A rename takes its words: what is typed, never narrower than the name it opened holding, and the
            /// placeholder whole once rubbed out — a box cut short of either says nothing.
            readonly property real wantW: chipColumn.namingMode !== "rename"
                ? Math.max(chipColumn.boxRoom, nameField.placeholderW)
                : Math.max(nameField.openedW, nameField.text === "" ? nameField.placeholderW : nameField.typedW)
            /// A worktree's folder in the refusal's line, which the tip stands the tree mark in front of, and the
            /// names it seats itself — read off this field by the seat (`TipAsk.marksNow`, デザイン規約 §ref の種別
            /// 「名前の印」).
            readonly property string tipMarkWord: chipColumn.namingRefusedMark
            readonly property var tipMarks: chipColumn.namingRefusedMarks
            /// The kind of ref being typed: the mode, or on a rename the kind it renames.
            readonly property string namesKind:
                chipColumn.namingMode === "rename" ? chipColumn.namingKind : chipColumn.namingMode
            // Up to the reach at most — past it what is typed scrolls in the box. The reach never cuts the box below
            // the column's room.
            width: Math.min(nameField.wantW,
                            Math.max(chipColumn.boxRoom, chipColumn.boxReach - Theme.spaceXs))
            // The frame names the kind (デザイン規約 §ref の種別「枠 = 種別」); a branch's colour is the focus ring's own,
            // so a tag differs, and so does a branch made out in another worktree — that worktree's green, as its chip
            // wears.
            focusTone: nameField.namesKind === "tag" ? Theme.refTag
                     : nameField.namesKind === "worktree" || nameField.namesKind === "worktreeFolder" ? Theme.success
                     : Theme.borderFocus
            // What goes in the box, by kind: a new worktree and a remote's are branch names too, a worktree on no
            // branch is named by its folder (デザイン規約 §グラフ行のダブルクリック「箱のプレースホルダ」).
            placeholderText: nameField.namesKind === "tag" ? qsTr("Tag name")
                           : nameField.namesKind === "worktreeFolder" ? qsTr("Worktree name") : qsTr("Branch name")
            refused: chipColumn.namingRefused
            /// That tip answers what is typed, not a hand: it stands until the typing changes it (`TipAsk.typed`).
            /// A worktree's folder in the refusal's line has the tree mark stood in front of it (デザイン規約 §ref の種別
            /// 「名前の印」).
            readonly property bool tipTyped: nameField.refused && nameField.activeFocus
                                             && chipColumn.namingRefusedWhy !== ""
            readonly property TipAsk nameTip: TipAsk {
                target: nameField
                asked: nameField.tipTyped
                typed: true
                words: chipColumn.namingRefusedWhy
                markWord: chipColumn.namingRefusedMark
            }
            // Refused text stays in the box; Enter does nothing, and the frame and tooltip say why.
            onAccepted: {
                if (!chipColumn.namingRefused)
                    chipColumn.namingSubmitted(nameField.text.trim())
            }
            // Held on the view, which outlives this recycled delegate.
            onTextEdited: chipColumn.namingEdited(nameField.text)
            Keys.onEscapePressed: chipColumn.namingCancelled()
            // The rulers, never drawn: the field's own placeholder cannot be measured before the box has a width.
            Label {
                id: placeholderInk
                visible: false
                text: nameField.placeholderText
                font: nameField.font
            }
            Label {
                id: openedInk
                visible: false
                text: chipColumn.namingMode === "rename" ? chipColumn.namingOpenedWith : ""
                font: nameField.font
            }
        }
    }
    // Built only while it says so (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
    Loader {
        id: withheldSeat
        active: chipColumn.withheldWhy !== ""
        anchors.fill: rowStack
        sourceComponent: Item {
            id: withheldBox
            // Answers the press that asked, not a hand (`TipAsk.typed`); the folder in it behind the tree mark.
            readonly property TipAsk withheldTip: TipAsk {
                target: withheldBox
                asked: withheldBox.visible && chipColumn.withheldWhy !== ""
                typed: true
                words: chipColumn.withheldWhy
                markWord: chipColumn.withheldMark
            }
        }
    }
    // Nothing here is hovered on its own: the row reads the pane's hand and says which part it is on
    // (`GraphRowDelegate.syncHand`); stacked names are read in the RefListPopup the chip's part opens.
}
