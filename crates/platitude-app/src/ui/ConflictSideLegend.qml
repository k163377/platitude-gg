pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Which branch each of the two colours is. The colours are the ones the
// graph already gives those branches, so this line is the whole of what
// has to be learned. The names swap over during a rebase and the model
// has already sorted that out (デザイン規約 §conflict の ours / theirs),
// so this says whatever it is handed.
RowLayout {
    id: legend

    required property string oursName
    required property string theirsName
    required property var oursColor
    required property var theirsColor
    /// The widest a name may be laid out at before it elides.
    required property real nameCap

    spacing: Theme.spaceSm
    Repeater {
        model: [{ swatch: legend.oursColor, name: legend.oursName },
                { swatch: legend.theirsColor, name: legend.theirsName }]
        delegate: RowLayout {
            required property var modelData
            spacing: Theme.spaceXs
            Rectangle {
                implicitWidth: Theme.spaceXs
                implicitHeight: Theme.fontSm
                color: parent.modelData.swatch
            }
            Label {
                text: parent.modelData.name
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
                elide: Text.ElideMiddle
                Layout.maximumWidth: legend.nameCap
            }
        }
    }
    Item { Layout.fillWidth: true }
}
