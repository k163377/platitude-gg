pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Which branch each of the two conflict colours is. The names swap during a rebase and the model has already sorted
// that out (デザイン規約 §conflict の ours / theirs), so this shows whatever it is handed.
RowLayout {
    id: legend

    required property string oursName
    required property string theirsName
    required property var oursColor
    required property var theirsColor
    /// The widest a name may be laid out at before it elides.
    required property real nameCap
    /// Whether each name is a branch's — the mark it then wears (デザイン規約 §ref の種別「名前の印」) — rather than the
    /// words standing in for one git left nothing to name (`Words.ourSide`).
    property bool oursIsBranch: false
    property bool theirsIsBranch: false
    /// Each branch's own colour: the current branch's where it is the branch stood on (`DiffFileNotices.sideTint`),
    /// none for any other.
    property var oursTint: undefined
    property var theirsTint: undefined

    spacing: Theme.spaceSm
    Repeater {
        model: [{ swatch: legend.oursColor, name: legend.oursName, branch: legend.oursIsBranch,
                  tint: legend.oursTint },
                { swatch: legend.theirsColor, name: legend.theirsName, branch: legend.theirsIsBranch,
                  tint: legend.theirsTint }]
        delegate: RowLayout {
            id: side
            required property var modelData
            spacing: Theme.spaceXs
            Rectangle {
                implicitWidth: Theme.spaceXs
                implicitHeight: Theme.fontSm
                color: side.modelData.swatch
            }
            // A branch as code, its mark on the same ground (デザイン規約 §ref の種別「文の中の名前」), a step down
            // with the legend's words (§ref の種別「補足の行では名前も 1 段落とす」); the words standing in for one as
            // words.
            CodeChip {
                visible: side.modelData.branch
                word: side.modelData.name
                mark: "branch"
                tint: Words.nameInk(Theme.textSecondary, side.modelData.tint)
                size: Theme.fontSm
                cap: legend.nameCap
            }
            Label {
                visible: !side.modelData.branch
                text: side.modelData.name
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
                elide: Text.ElideMiddle
                Layout.maximumWidth: legend.nameCap
            }
        }
    }
    Item { Layout.fillWidth: true }
}
