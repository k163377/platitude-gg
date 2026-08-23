import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Whoever a commit message credits alongside its author, in one of two forms — the same records, written for the room
// they are written in (規約 §co-author の表示):
//
//   "◯ Name +N"        the details pane's date row: a face, the first
//                      name, the rest counted, and a rule saying the
//                      card with everybody's address opens here.
//   "Name, Name, Name"  the graph row's hover card: everyone written
//                      out, comma separated, elided at the width, and
//                      offering nothing — the pane is where a commit is
//                      read in full.
//
// It draws and reports; the card that names everyone belongs to whoever owns this, because a popup cannot live inside a
// recycled delegate.
Item {
    id: line

    /// Packed co-author records (see `encode::encode_co_authors`).
    property string packed: ""
    /// Full weight on the rule — the owner sets this while its card is up.
    property bool lit: false
    /// How wide the first name may run before it elides; 0 leaves it its own width. A trailer's name is whatever the
    /// message says it is — git enforces no length — and this line reports its own width to whoever holds it, so an
    /// unbounded name widened both holders: the hover card went from a share of the graph pane to 1,022px over it
    /// (measured), and the details pane's date row runs past its column the same way. The owner passes its share, the
    /// way the card passes one for the message.
    property real nameWidth: 0
    /// Write them all out instead, and offer nothing: no face, no rule, no card. For a holder wide enough to name
    /// everybody, where the addresses are a pane away rather than a hover away.
    property bool plain: false
    /// The pointer entered or left the underlined stretch.
    signal pointerChanged(bool inside)
    /// The names did not fit what the holder gave them. Nothing is drawn differently for it — the ellipsis already says
    /// so — but a headless run cannot see an ellipsis, and the width rule is the whole of the plain form.
    readonly property bool clipped: names.truncated

    readonly property var records: line.packed === "" ? [] : line.packed.split(String.fromCharCode(31))
    function nameAt(i) {
        const record = line.records[i]
        return record === undefined ? "" : record.split(String.fromCharCode(30))[0]
    }
    function faceAt(i) {
        const record = line.records[i]
        return record === undefined ? 0 : parseInt(record.split(String.fromCharCode(30))[2])
    }
    function allNames() {
        let out = ""
        for (let i = 0; i < line.records.length; i++)
            out += (i === 0 ? "" : ", ") + line.nameAt(i)
        return out
    }

    visible: line.records.length > 0
    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight

    RowLayout {
        id: row
        anchors.fill: parent
        spacing: Theme.spaceXs
        IdentIcon {
            visible: !line.plain
            code: line.faceAt(0)
            width: Theme.iconMd
            height: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
        }
        Label {
            id: names
            text: line.plain ? line.allNames() : line.nameAt(0)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            Layout.alignment: Qt.AlignVCenter
            // Written out, the names take the room the holder gives them and stop there; as a chip, the name gives way
            // at the share its holder passed. Either way the name is what yields when the room runs out -- the face and
            // the count are one glyph each and have nothing to give.
            Layout.fillWidth: true
            Layout.maximumWidth: line.plain || line.nameWidth <= 0 ? Number.POSITIVE_INFINITY : line.nameWidth
            elide: Text.ElideRight
        }
        Label {
            visible: !line.plain && line.records.length > 1
            text: "+" + (line.records.length - 1)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            Layout.alignment: Qt.AlignVCenter
        }
    }
    // Drawn at rest one step down from the name it underlines (規約 §暗く落とした段 names borderStrong as textSecondary's step),
    // and up to the name's own value under the pointer.
    BandRule {
        visible: !line.plain
        color: line.lit ? Theme.textSecondary : Theme.borderStrong
    }
    HoverHandler {
        id: lineHover
        enabled: !line.plain
        onHoveredChanged: line.pointerChanged(lineHover.hovered)
    }
}
