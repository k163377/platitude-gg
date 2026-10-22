pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The mark and the name a list row opens with, shared by a commit's changed files (`FileRowDelegate`) and the working
// tree's (`NavItemDelegate`, which draws the ref rows too), down to the seat the names start after. What a click
// means, and whatever stands at the right edge, stays with the row.
RowLayout {
    id: nameCell

    /// A directory row: the seat holds a fold arrow, and the name takes the quieter colour.
    property bool folder: false
    /// The change code git reports for the file (`M`, `??`, `UU`, …). A folder row has none, and the sidebar's rows
    /// keep their fold state in this same field to spare a `NavItem` a second one (`models::nav::FOLDED`).
    property string change: ""
    /// Whether a folder row is shut. Asked as its own question: the sidebar packs it into `change`, a commit's changed
    /// files have their own field (`models::details::FileItem.collapsed`).
    property bool folded: false
    /// Whether a change mark belongs in the seat (a ref row has none). The seat is held open either way: a layout
    /// drops an invisible child, and every name at one depth has to begin in the same column.
    property bool showChange: true
    /// Whether the seat is in the row at all — off for a list whose rows never wear anything there. One answer per
    /// list, like `showChange`.
    property bool seated: true
    property string name: ""
    /// Where a renamed file came from, drawn ahead of the new name; empty otherwise. Already cut back the way the row
    /// writes names (`encode::rename_source`) — nothing here takes a path apart.
    property string origPath: ""
    /// The colour a named row wears; a folder always takes the quieter one.
    property color tone: Theme.textPrimary
    property int weight: Font.Normal
    /// A mark on the name's shoulder (`ActionButton.alert`'s kind), outside the layout so nothing in the row moves.
    property bool marked: false
    property color markTint: Theme.warning
    /// A `NavIcon.kind` for the seat that is not a change code: a locked worktree's padlock, a gone folder's bang, the
    /// WORKTREES mark on a branch another worktree has out. Only one of fold arrow, change code and this ever draws.
    property string seatMark: ""
    property color seatTint: Theme.textSecondary
    /// Whether the name is in the row at all; the seat stays, so a box opening over the name (`NavItemDelegate`'s
    /// rename) moves no other column. The owner's `Layout.fillWidth` follows this, so the slack goes to the box.
    property bool showName: true
    /// The same name in full, drawn over the cut one in its own place while the row is open (デザイン規約
    /// §左メニューの所作); empty leaves the cut name. Nothing beside it moves: a name too long for one line wraps
    /// downward out of the row, which grows by it (`NavItemDelegate.nameOverflow`).
    property string whole: ""
    /// That field, and how far it hangs below the one line a name is drawn on (a drag goes into the first; the row
    /// grows by the second). The overhang is counted off the field's own line, not the label's: the two differ by a
    /// hair, which would open as a gap under every name that fits.
    readonly property Item wholeField: wholeSeat.item
    readonly property real wholeOver:
        wholeSeat.item ? Math.max(0, wholeSeat.item.implicitHeight - wholeSeat.item.lineHeight) : 0
    /// Automation: the turn the fold arrow is drawn at, -1 without one. Read off the icon itself, so a run cannot go
    /// green with the arrow unwired.
    readonly property real foldTurn: foldSeat.item ? foldSeat.item.turn : -1
    /// How wide the slot every row opens with is (デザイン規約 §余白「その席は `iconXs` で、印は席の中央に置く」).
    /// The fold arrow, the change codes and the state marks are
    /// all drawn to the name's letters (`set: "lead"`) at one scale and centred across the seat; the widest, the
    /// filled house (11 squares), stays inside it at `fontMd`. The seat is the column the marks centre on, not a
    /// frame: a mark's ink may run a little past it.
    ///
    /// One width for every list, so a name stands as far from its mark in the file lists as in the sidebar; measured
    /// per row, the padlock and the fold arrow would step the names in and out.
    readonly property int seatSize: Theme.iconXs
    /// Where the name itself is drawn — the lines a row opens read their voice off it, and it knows whether the name
    /// was cut (`NavRowFacts`, rules/app-ui.md「測って押し出す値は…」).
    readonly property Item nameInk: newName
    /// Where the name's baseline runs, in the cell's y: a mark set beside the name from outside the cell stands on
    /// it (`NavRowBody`'s remote and push marks).
    readonly property real nameBase: nameRow.y + newName.y + newName.baselineOffset
    /// The same in the seat: every mark there is set against it, centred on the capitals' middle (デザイン規約
    /// §アイコンの席 の文頭記号).
    readonly property real seatBase: nameCell.nameBase - markSeat.y

    spacing: Theme.spaceXs

    // The seat every row opens with. Each mark in it is built only on the rows that wear it
    // (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」), and every one is the head of the row's words: centred
    // across the seat, drawn to the name's letters and set against its baseline, centred on the capitals' middle —
    // one column of marks, the same air either side (デザイン規約 §アイコンの席 の文頭記号).
    Item {
        id: markSeat
        // Invisible takes the layout's spacing after it as well (`seated`).
        visible: nameCell.seated
        Layout.preferredWidth: nameCell.seatSize
        Layout.preferredHeight: newName.implicitHeight
        Loader {
            id: foldSeat
            active: nameCell.folder
            anchors.horizontalCenter: parent.horizontalCenter
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: markSeat.height
                kind: "chevron"
                set: "lead"
                size: newName.pixelSize
                wordBase: nameCell.seatBase
                turn: nameCell.folded ? 0 : 90
                tint: Theme.textSecondary
            }
        }
        Loader {
            active: !nameCell.folder && nameCell.showChange
            anchors.horizontalCenter: parent.horizontalCenter
            sourceComponent: ChangeIcon {
                width: Theme.iconSm
                height: markSeat.height
                set: "lead"
                size: newName.pixelSize
                wordBase: nameCell.seatBase
                change: nameCell.change
            }
        }
        Loader {
            id: stateSeat
            active: nameCell.seatMark !== ""
            anchors.horizontalCenter: parent.horizontalCenter
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: markSeat.height
                set: "lead"
                size: newName.pixelSize
                wordBase: nameCell.seatBase
                kind: nameCell.seatMark
                tint: nameCell.seatTint
            }
        }
    }
    // A rename is two names with a drawn arrow between them — U+2192 is East Asian Ambiguous, so the CJK families draw
    // it full-width (規約 §寸法「印は描いて出す」). The old name is capped at its own width
    // (rules-refs/app-ui.md「`Layout.fillWidth` を書いていない子は縮まない床」); the new one takes the slack, or the
    // engine centres what no child fills.
    RowLayout {
        id: nameRow
        visible: nameCell.showName
        Layout.fillWidth: true
        spacing: 0
        // Built only on a rename (the seat's rule). Invisible while inactive: an empty loader would still take the
        // spacing.
        Loader {
            id: origSeat
            active: !nameCell.folder && nameCell.origPath !== ""
            visible: origSeat.active
            Layout.fillWidth: true
            // The name's own width, which `CutName` reports already rounded up
            // (rules-refs/app-ui.md「自然幅の上限は切り上げる」).
            Layout.maximumWidth: origSeat.implicitWidth
            sourceComponent: CutName {
                text: nameCell.origPath
                pixelSize: Theme.fontMd
                // Where it came from is context, in the context colour (§テキスト); `textMuted` is the disabled signal
                // (§無効) and cannot carry a second meaning.
                color: Theme.textSecondary
            }
        }
        // A sign between the names (デザイン規約 §アイコンの席 の修飾): smaller and quieter than the names, so it does
        // not read as the change-kind arrow at the row's head. It joins two words, so a word's step stands either side
        // of its room. Its box is the names' line, so it is set against their baseline on the face's own `+`'s middle.
        Loader {
            id: renameSeat
            active: origSeat.active
            visible: origSeat.active
            Layout.preferredWidth: renameSeat.item ? renameSeat.item.wordCell + 2 * Theme.spaceXs : 0
            Layout.preferredHeight: newName.implicitHeight
            sourceComponent: NavIcon {
                kind: "arrow"
                set: "sign"
                size: newName.pixelSize
                wordBase: newName.baselineOffset
                width: renameSeat.width
                height: renameSeat.height
                tint: Theme.textSecondary
            }
        }
        CutName {
            id: newName
            Layout.fillWidth: true
            text: nameCell.name
            pixelSize: Theme.fontMd
            weight: nameCell.weight
            color: nameCell.folder ? Theme.textSecondary : nameCell.tone
            // Hidden under the whole name but still worked out, so this column keeps its width.
            inked: nameCell.whole === ""
            // The name in full, anchored rather than laid out: outside the row's layout, a wrapping name hangs below
            // the row (`NavItemDelegate.nameOverflow`) instead of widening it. Built only while asked for (the seat's
            // rule).
            Loader {
                id: wholeSeat
                active: nameCell.whole !== ""
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: wholeSeat.item ? wholeSeat.item.implicitHeight : 0
                sourceComponent: CardText {
                    text: nameCell.whole
                    pixelSize: Theme.fontMd
                    weight: nameCell.weight
                    color: nameCell.folder ? Theme.textSecondary : nameCell.tone
                }
            }
            // Built only on a marked row (the seat's rule).
            Loader {
                active: nameCell.marked
                // Half a gap back into the name's trailing bearing, so it reads as part of the word; clamped to the
                // column's edge when the name is cut.
                x: Math.min(newName.inkWidth - Theme.spaceXs / 2,
                            newName.width - Theme.iconSm)
                y: 0
                sourceComponent: NavIcon {
                    kind: "bang"
                    tint: nameCell.markTint
                    width: Theme.iconSm
                    height: Theme.iconSm
                }
            }
        }
    }
}
