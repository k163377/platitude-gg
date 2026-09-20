import QtQuick
import QtQuick.Layouts
import platitude.ui

// A word above whatever answers it, and the air between the two. The dialogs' forms are stacks of these.
//
// **The word is a field** (`CardText`, 規約 §右のペインの字は掴める): on a surface whose words can be dragged over,
// a caption that took no press would be a dead strip in the middle of them — a press on it would start its selection
// in whatever line stood nearest instead (`SweepPad.nearestTo`).
//
// The word is the label of a control and reads as one: `textPrimary`, `Font.Normal`, and the same step as the value
// under it, because the step is chosen by what a thing is for (デザイン規約 §タイポグラフィ). **Primary ink** — the explanations
// beside it are what is secondary here, and a form whose labels wear the same ink as its notes has nothing left to
// tell them apart with (observed). What keeps it under the chapter heading above is weight, case and that heading's
// rule (`SettingsSection`). Whatever a caller declares here follows it down the column — a field, a field and a
// mark, a whole list — which is why the shape stops at the caption.
ColumnLayout {
    id: field

    /// The word above.
    property string caption: ""

    Layout.fillWidth: true
    spacing: Theme.spaceXs

    CardText {
        text: field.caption
        color: Theme.textPrimary
        pixelSize: Theme.fontMd
    }
}
