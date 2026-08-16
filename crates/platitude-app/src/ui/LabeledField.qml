import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A word above whatever answers it, and the air between the two. The
// dialogs' forms are stacks of these.
//
// The word is the quiet one: it names the box rather than saying anything,
// so it takes the secondary colour and the small step
// (デザイン規約 §タイポグラフィ). Whatever a caller declares here follows
// it down the column — a field, a field and a mark, a whole list — which
// is why the shape stops at the caption.
ColumnLayout {
    id: field

    /// The word above.
    property string caption: ""

    Layout.fillWidth: true
    spacing: Theme.spaceXs

    Label {
        text: field.caption
        color: Theme.textSecondary
        font.pixelSize: Theme.fontSm
    }
}
