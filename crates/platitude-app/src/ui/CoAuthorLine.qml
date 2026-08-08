import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// "◯ Name +N" — whoever a commit message credits alongside its author,
// written the same way wherever it appears: the details pane's date row
// and the graph row's hover both use this one.
//
// It draws and reports; the card that names everyone belongs to whoever
// owns this, because a popup cannot live inside a recycled delegate.
Item {
    id: line

    /// Packed co-author records (see `encode::encode_co_authors`).
    property string packed: ""
    /// Full weight on the rule — the owner sets this while its card is up.
    property bool lit: false
    /// How wide the first name may run before it elides; 0 leaves it its
    /// own width. A trailer's name is whatever the message says it is —
    /// git enforces no length — and this line reports its own width to
    /// whoever holds it, so an unbounded name widened both holders: the
    /// hover card went from a share of the graph pane to 1,022px over it
    /// (measured), and the details pane's date row runs past its column
    /// the same way. The owner passes its share, the way the card passes
    /// one for the message.
    property real nameWidth: 0
    /// The pointer entered or left the underlined stretch.
    signal pointerChanged(bool inside)

    readonly property var records:
        line.packed === "" ? [] : line.packed.split(String.fromCharCode(31))
    function nameAt(i) {
        const record = line.records[i]
        return record === undefined
               ? "" : record.split(String.fromCharCode(30))[0]
    }
    function faceAt(i) {
        const record = line.records[i]
        return record === undefined
               ? 0 : parseInt(record.split(String.fromCharCode(30))[2])
    }

    visible: line.records.length > 0
    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight

    RowLayout {
        id: row
        anchors.fill: parent
        spacing: Theme.spaceXs
        IdentIcon {
            code: line.faceAt(0)
            width: Theme.iconMd
            height: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
        }
        Label {
            text: line.nameAt(0)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            Layout.alignment: Qt.AlignVCenter
            Layout.maximumWidth: line.nameWidth > 0
                                 ? line.nameWidth : Number.POSITIVE_INFINITY
            elide: Text.ElideRight
        }
        Label {
            visible: line.records.length > 1
            text: "+" + (line.records.length - 1)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            Layout.alignment: Qt.AlignVCenter
        }
    }
    // Drawn at rest one step down from the name it underlines (規約
    // §暗く落とした段 names borderStrong as textSecondary's step), and up
    // to the name's own value under the pointer.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: line.lit ? Theme.textSecondary : Theme.borderStrong
    }
    HoverHandler {
        id: lineHover
        onHoveredChanged: line.pointerChanged(lineHover.hovered)
    }
}
