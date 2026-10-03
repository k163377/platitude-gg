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
    /// A `NavIcon.kind` for the seat that is not a change code: a locked copy's padlock, a gone folder's bang, the
    /// WORKTREES mark on a branch another copy has out. Only one of fold arrow, change code and this ever draws.
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
    readonly property real foldTurn: foldSeat.item ? foldSeat.item.rotation : -1
    /// How wide the slot every row opens with is, taken on the ink (デザイン規約
    /// §余白「印が自分で持っている余白は、隣の詰めに数える」). The fold arrow and the state marks are drawn on the
    /// `iconSm` grid without reaching its edge, so the seat is a step under it. That width is the ceiling a mark
    /// drawn for this seat is designed against (the widest, the house, spans 11 of 16), and the widest change code,
    /// the pen, is as wide on the `iconMd` grid it is drawn on (10 of 16).
    ///
    /// One width for every list, so a name stands as far from its mark in the file lists as in the sidebar; measured
    /// per row, the padlock and the fold arrow would step the names in and out.
    readonly property int seatSize: Theme.iconXs
    /// Where an `iconSm` mark stands inside that seat: hard against its left edge, its box hanging out on the right.
    /// The air comes off the right alone — the mark's left is what the fold's step reads as nesting
    /// (`NavList.nestStep`).
    readonly property real seatNudge: (Theme.iconSm - nameCell.seatSize) / 2
    /// Where the name itself is drawn — the lines a row opens read their voice off it, and it knows whether the name
    /// was cut (`NavRowFacts`, rules/app-ui.md「測って押し出す値は…」).
    readonly property Item nameInk: newName

    spacing: Theme.spaceXs

    // The seat every row opens with. Each mark in it is built only on the rows that wear it
    // (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
    Item {
        // Invisible takes the layout's spacing after it as well (`seated`).
        visible: nameCell.seated
        Layout.preferredWidth: nameCell.seatSize
        Layout.preferredHeight: nameCell.seatSize
        Layout.alignment: Qt.AlignVCenter
        Loader {
            id: foldSeat
            active: nameCell.folder
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: nameCell.seatNudge
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: Theme.iconSm
                kind: "chevron"
                rotation: nameCell.folded ? 0 : 90
                tint: Theme.textSecondary
            }
        }
        // A change code is drawn a grid up from the seat's other marks, so its box is the wider: set to end where
        // theirs does, the widest of them (the pen) fills the seat and a name follows it as closely as it follows a
        // chevron. The box hangs out on the left, into the fold's step; filling the seat would draw it two grids down.
        Loader {
            active: !nameCell.folder && nameCell.showChange
            anchors.verticalCenter: parent.verticalCenter
            anchors.right: parent.right
            anchors.rightMargin: nameCell.seatSize - Theme.iconSm
            sourceComponent: ChangeIcon {
                change: nameCell.change
            }
        }
        // On the fold arrow's grid: these stand among the sidebar's arrows, and a wider grid's heavier line reads as a
        // bigger mark (デザイン規約 §左メニューの所作「畳みの山形は 2 階級ある」).
        //
        // Centred down the seat and set against its left edge (`seatNudge`): given the seat's own size, it would take
        // the seat's top-left corner and ride a step high.
        Loader {
            active: nameCell.seatMark !== ""
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: nameCell.seatNudge
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: Theme.iconSm
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
        Loader {
            id: renameSeat
            active: origSeat.active
            visible: origSeat.active
            Layout.preferredWidth: renameSeat.item ? renameSeat.item.inkWidth + Theme.spaceXs * 2 : 0
            Layout.preferredHeight: Theme.iconSm
            Layout.alignment: Qt.AlignVCenter
            sourceComponent: Item {
                readonly property real inkWidth: renameMark.inkWidth
                NavIcon {
                    id: renameMark
                    anchors.centerIn: parent
                    kind: "arrow"
                    // Beside a word: a step down, the line thinned to match
                    // (rules-refs/app-ui.md「語の隣に立つ印は `iconSm`」).
                    width: Theme.iconSm
                    height: Theme.iconSm
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                    tint: Theme.textSecondary
                }
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
