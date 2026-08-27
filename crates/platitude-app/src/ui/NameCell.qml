pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The mark and the name a list row opens with, in the one place both file lists read it from: a commit's changed files
// (`FileRowDelegate`) and the working tree's (`NavItemDelegate`, which draws the ref rows with the same part). What a
// click means, and whatever else a row puts at its right edge, stays with the row — this is the half the two lists were
// saying twice, down to the seat the names start after.
RowLayout {
    id: nameCell

    /// A directory row: the seat holds a fold arrow instead of a change mark, and the name speaks in the quieter colour
    /// — it is the shape of the names under it rather than a name git knows.
    property bool folder: false
    /// The change code git reports for the file (`M`, `??`, `UU`, …). A folder row has none, and the sidebar's rows
    /// keep their fold state in this same field to spare a `NavItem` a second one (`models::nav::FOLDED`).
    property string change: ""
    /// Whether a folder row is shut — which way its arrow points. **Asked as its own question**, because the two lists
    /// do not keep the answer in the same place: the sidebar packs it into `change` (above), a commit's changed files
    /// have a field of their own (`models::details::FileItem.collapsed`). Reading only the sidebar's is what left the
    /// commit list's arrow lying open in both states.
    property bool folded: false
    /// Whether a change mark belongs in the seat at all — a ref row has no change to report. **The seat is held open
    /// either way**: letting it collapse is what put a leaf's name to the *left* of the folder it sits under (layouts
    /// drop invisible children entirely), and at a given depth every name has to begin in the same column.
    property bool showChange: true
    property string name: ""
    /// Where a renamed file came from, drawn ahead of the new name with the way between them. Empty on everything else.
    /// **Already written the way the row writes names** — the models cut it back exactly as far as they cut the new one
    /// (`encode::rename_source`), so nothing here takes a path apart.
    property string origPath: ""
    /// The colour a named row wears. A folder is not asked: it is the quieter one wherever it appears, in both lists.
    property color tone: Theme.textPrimary
    property int weight: Font.Normal
    /// A mark on the name's shoulder, the way a button in trouble wears one (`ActionButton.alert`) — **outside the
    /// layout**, so no row moves and whatever sits at the right edge keeps its own seat.
    property bool marked: false
    property color markTint: Theme.warning
    /// A mark for the seat that is not a change code: a locked working copy's padlock, the bang on one whose folder
    /// has gone, and the WORKTREES mark on a branch another copy has out. `NavIcon.kind`, empty where the seat's
    /// usual tenants have it. **Only one of the three ever draws** — a row is a folder, a file with a change, or one
    /// of these.
    property string seatMark: ""
    property color seatTint: Theme.textSecondary
    /// Whether the name is in the row at all. The seat stays either way: a box opening over the name
    /// (`NavItemDelegate`'s rename) must not walk the row's other columns sideways. The owner takes the slack back the
    /// same way it gives it — `Layout.fillWidth` follows this.
    property bool showName: true
    /// Automation: the turn the fold arrow is drawn at, -1 on a row that has none. Read off the icon rather than off
    /// the condition behind it, so a run cannot go green with the arrow unwired (verify-ui — the same reading as
    /// `FileRowDelegate.litKey`).
    readonly property real foldTurn: foldArrow.visible ? foldArrow.rotation : -1
    /// How wide the slot every row opens with is. **The seat is taken on the ink, not on the box the mark is drawn
    /// in** — the same reading `NavHeader`'s own fold seat makes, where an `iconMd` chevron stands in an `iconSm` seat
    /// and overflows it (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」). A change code fills its box, so a list
    /// that shows one takes the box outright (`iconMd`); the marks that stand here where no change can — the fold
    /// arrow and the sidebar's state marks — are drawn on the `iconSm` grid and none of them reaches its edge, so that
    /// seat comes down a step further and the air stops being paid for twice (2026-08-27 ユーザー指示: the widest of them,
    /// the padlock, spans 9 of the 16, so with the line it carries it fills an `iconXs` seat almost exactly).
    ///
    /// **One answer per list.** `showChange` is a section's kind rather than a row's, so no row's name begins in a
    /// different column from its neighbours' — the invariant the seat is held open for in the first place. It is the
    /// same reason the seat is not taken on each row's own mark: the padlock and the fold arrow hold different
    /// amounts of air, and a seat that measured them one row at a time would step the names in and out.
    readonly property int seatSize: nameCell.showChange ? Theme.iconMd : Theme.iconXs
    /// Where a mark stands inside that seat: hard against its left edge, which for the trimmed seat above means the
    /// box it is drawn in hangs a pixel out on the right. **The air is taken off the right alone** (2026-08-27
    /// ユーザー指示) — the left of the mark is what the fold's own step reads as nesting, and centring the mark in a seat
    /// narrower than its box would walk it back towards the frame by half of whatever the right gave up. Zero where
    /// the change mark fills its seat: there the box *is* the ink, and the arrow it shares the slot with is centred in
    /// it the way it always was.
    readonly property real seatNudge: nameCell.showChange ? 0 : (Theme.iconSm - nameCell.seatSize) / 2

    spacing: Theme.spaceXs

    // The seat every row opens with.
    Item {
        Layout.preferredWidth: nameCell.seatSize
        Layout.preferredHeight: nameCell.seatSize
        Layout.alignment: Qt.AlignVCenter
        NavIcon {
            id: foldArrow
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: nameCell.seatNudge
            visible: nameCell.folder
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "chevron"
            rotation: nameCell.folded ? 0 : 90
            tint: Theme.textSecondary
        }
        ChangeIcon {
            anchors.fill: parent
            visible: !nameCell.folder && nameCell.showChange
            change: nameCell.change
        }
        // The fold arrow's step, not the change mark's. These stand in the sidebar alone, beside the arrows of the
        // rows they are nested among, and a mark drawn on the wider grid carried a heavier line than the folds it
        // sits between — weight is what the eye reads as size (`NavIcon.stroke`; 2026-08-27 ユーザー指示). The line
        // itself is not scaled with the grid: it is levelled *with* the arrow, so both wear the family's own
        // (デザイン規約 §寸法「畳みの山形は 2 階級」).
        //
        // **Placed, not filled.** A mark given the seat's own size takes the seat's top-left corner, and on a row
        // whose seat is centred in it that reads as a mark riding a step high (2026-08-27 ユーザー報告 — the slip the
        // first pass at this made). So it is centred down the seat and set against its left edge (`seatNudge`).
        NavIcon {
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: nameCell.seatNudge
            width: Theme.iconSm
            height: Theme.iconSm
            visible: nameCell.seatMark !== ""
            kind: nameCell.seatMark
            tint: nameCell.seatTint
        }
    }
    // A rename is two names with the way between them drawn rather than typed: U+2192 is East Asian Ambiguous, so the
    // CJK families this app names hold it in a full-width cell and each draws its own arrow inside it (規約
    // §寸法「印はフォントの字に任せない」). Both names give ground when the row is narrow. The old name is capped at its own width so it
    // shrinks without growing (app-ui.md §`Layout.fillWidth` を書いていない子は縮まない床); **the new one is not capped** — someone
    // in the row has to take the slack, and a row where every child refuses it hands the leftover to the engine, which
    // centres what it cannot fill (measured: plain rows drifted to the middle of the pane).
    RowLayout {
        visible: nameCell.showName
        Layout.fillWidth: true
        spacing: 0
        CutName {
            id: origName
            visible: !nameCell.folder && nameCell.origPath !== ""
            Layout.fillWidth: true
            // The ceiling is the name's own width, and `CutName` reports that already rounded up — the layout hands an
            // item the whole pixel below a fractional ceiling, and a ceiling one hair under the name's own width cuts
            // it (app-ui.md §自然幅の上限は切り上げる).
            Layout.maximumWidth: origName.implicitWidth
            text: nameCell.origPath
            pixelSize: Theme.fontMd
            // The name the file has now is the subject; where it came from is context, and wears the colour the rest of
            // the app gives context (§テキスト: author / 日時 / 短縮ハッシュ). **Not `textMuted`** — that one is the disabled
            // signal (§無効), and a second meaning for it cannot be read apart.
            color: Theme.textSecondary
        }
        Item {
            visible: origName.visible
            Layout.preferredWidth: renameMark.inkWidth + Theme.spaceXs * 2
            Layout.preferredHeight: Theme.iconSm
            Layout.alignment: Qt.AlignVCenter
            NavIcon {
                id: renameMark
                anchors.centerIn: parent
                kind: "arrow"
                // Beside a word, so a step under the row's own mark, with the line taken down by the same ratio
                // (app-ui.md §語の隣に立つ印).
                width: Theme.iconSm
                height: Theme.iconSm
                stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
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
            NavIcon {
                visible: nameCell.marked
                kind: "bang"
                tint: nameCell.markTint
                width: Theme.iconSm
                height: Theme.iconSm
                // Clamped: a cut name ends at the column's own right edge, and the mark belongs to the name that is on
                // screen. Half a gap back into the name's own trailing bearing, so it reads as part of the word rather
                // than as the next column.
                x: Math.min(newName.inkWidth - Theme.spaceXs / 2,
                            newName.width - Theme.iconSm)
                y: 0
            }
        }
    }
}
