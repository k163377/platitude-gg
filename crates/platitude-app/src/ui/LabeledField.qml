import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A word above whatever answers it, and the air between the two. The dialogs' forms are stacks of these.
//
// The word is the label of a control and reads as one: `textPrimary`, `Font.Normal`, and the same step as the value
// under it, because the step is chosen by what a thing is for rather than by how loud it should be
// (デザイン規約 §タイポグラフィ). **Never `textSecondary`** — the explanations beside it are what is secondary here,
// and a form whose labels wear the same ink as its notes has nothing left to tell them apart with (observed). What
// keeps it under the chapter heading above is weight, case and that heading's rule (`SettingsSection`). Whatever a
// caller declares here follows it down the column — a field, a field and a mark, a whole list — which is why the
// shape stops at the caption.
ColumnLayout {
    id: field

    /// The word above.
    property string caption: ""

    Layout.fillWidth: true
    spacing: Theme.spaceXs

    Label {
        text: field.caption
        color: Theme.textPrimary
        font.pixelSize: Theme.fontMd
    }
}
