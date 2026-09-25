import QtQuick
import QtQuick.Layouts
import platitude.ui

// A caption above whatever a caller declares under it (デザイン規約 §設定の画面). The caption is a `CardText` field: one
// taking no press is a dead strip on a surface whose words can be dragged over.
ColumnLayout {
    id: field

    property string caption: ""

    Layout.fillWidth: true
    spacing: Theme.spaceXs

    CardText {
        text: field.caption
        color: Theme.textPrimary
        pixelSize: Theme.fontMd
    }
}
