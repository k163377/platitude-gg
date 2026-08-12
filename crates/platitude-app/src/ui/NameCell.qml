pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The mark and the name a list row opens with, in the one place both file
// lists read it from: a commit's changed files (`FileRowDelegate`) and the
// working tree's (`NavItemDelegate`, which draws the ref rows with the
// same part). What a click means, and whatever else a row puts at its
// right edge, stays with the row — this is the half the two lists were
// saying twice, down to the seat the names start after.
RowLayout {
    id: nameCell

    /// A directory row: the seat holds a fold arrow instead of a change
    /// mark, and the name speaks in the quieter colour — it is the shape
    /// of the names under it rather than a name git knows.
    property bool folder: false
    /// The change code git reports for the file (`M`, `??`, `UU`, …). A
    /// folder row has none, so it keeps its fold state in the same field
    /// (`models::nav::FOLDED`) and this reads it back.
    property string change: ""
    /// Whether a change mark belongs in the seat at all — a ref row has no
    /// change to report. **The seat is held open either way**: letting it
    /// collapse is what put a leaf's name to the *left* of the folder it
    /// sits under (layouts drop invisible children entirely), and at a
    /// given depth every name has to begin in the same column.
    property bool showChange: true
    property string name: ""
    /// Where a renamed file came from, drawn ahead of the new name with
    /// the way between them. Empty on everything else. **Already written
    /// the way the row writes names** — the models cut it back exactly as
    /// far as they cut the new one (`encode::rename_source`), so nothing
    /// here takes a path apart.
    property string origPath: ""
    /// The colour a named row wears. A folder is not asked: it is the
    /// quieter one wherever it appears, in both lists.
    property color tone: Theme.textPrimary
    property int weight: Font.Normal
    /// A mark on the name's shoulder, the way a button in trouble wears
    /// one (`ActionButton.alert`) — **outside the layout**, so no row
    /// moves and whatever sits at the right edge keeps its own seat.
    property bool marked: false
    property color markTint: Theme.warning
    /// Whether the name is in the row at all. The seat stays either way:
    /// a box opening over the name (`NavItemDelegate`'s rename) must not
    /// walk the row's other columns sideways. The owner takes the slack
    /// back the same way it gives it — `Layout.fillWidth` follows this.
    property bool showName: true

    /// Whether the row had to cut the name short. This is the tooltip's
    /// trigger in both lists (デザイン規約 §hover のツールチップ): a name
    /// the pane elided is the one thing the row leaves unsaid, and both
    /// panes were reading it off a label of their own.
    readonly property bool truncated:
        nameCell.showName
        && (newName.truncated || (origName.visible && origName.truncated))

    spacing: Theme.spaceXs

    // The seat every row opens with.
    Item {
        Layout.preferredWidth: Theme.iconMd
        Layout.preferredHeight: Theme.iconMd
        Layout.alignment: Qt.AlignVCenter
        NavIcon {
            anchors.centerIn: parent
            visible: nameCell.folder
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "chevron"
            rotation: nameCell.change === "FOLDED" ? 0 : 90
            tint: Theme.textSecondary
        }
        ChangeIcon {
            anchors.fill: parent
            visible: !nameCell.folder && nameCell.showChange
            change: nameCell.change
        }
    }
    // A rename is two names with the way between them drawn rather than
    // typed: U+2192 is East Asian Ambiguous, so the CJK families this app
    // names hold it in a full-width cell and each draws its own arrow
    // inside it (規約 §寸法「印はフォントの字に任せない」). Both names
    // give ground when the row is narrow. The old name is capped at its
    // own width so it shrinks without growing (app-ui.md
    // §`Layout.fillWidth` を書いていない子は縮まない床); **the new one is
    // not capped** — someone in the row has to take the slack, and a row
    // where every child refuses it hands the leftover to the engine, which
    // centres what it cannot fill (measured: plain rows drifted to the
    // middle of the pane).
    RowLayout {
        visible: nameCell.showName
        Layout.fillWidth: true
        spacing: 0
        Label {
            id: origName
            visible: !nameCell.folder && nameCell.origPath !== ""
            Layout.fillWidth: true
            Layout.maximumWidth: origName.implicitWidth
            text: nameCell.origPath
            elide: Text.ElideMiddle
            font.pixelSize: Theme.fontMd
            // The name the file has now is the subject; where it came
            // from is context, and wears the colour the rest of the app
            // gives context (§テキスト: author / 日時 / 短縮ハッシュ).
            // **Not `textMuted`** — that one is the disabled signal
            // (§無効), and a second meaning for it cannot be read apart.
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
                // Beside a word, so a step under the row's own mark, with
                // the line taken down by the same ratio (app-ui.md
                // §語の隣に立つ印).
                width: Theme.iconSm
                height: Theme.iconSm
                stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                tint: Theme.textSecondary
            }
        }
        Label {
            id: newName
            Layout.fillWidth: true
            text: nameCell.name
            elide: Text.ElideMiddle
            font.pixelSize: Theme.fontMd
            font.weight: nameCell.weight
            color: nameCell.folder ? Theme.textSecondary : nameCell.tone
            NavIcon {
                visible: nameCell.marked
                kind: "bang"
                tint: nameCell.markTint
                width: Theme.iconSm
                height: Theme.iconSm
                // Clamped: an elided name is wider than its box, and the
                // mark belongs to the name that is on screen. Half a gap
                // back into the name's own trailing bearing, so it reads
                // as part of the word rather than as the next column.
                x: Math.min(newName.implicitWidth - Theme.spaceXs / 2,
                            newName.width - Theme.iconSm)
                y: 0
            }
        }
    }
}
