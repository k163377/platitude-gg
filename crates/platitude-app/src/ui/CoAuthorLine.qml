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

    /// The co-author records (`{name, email, face}` — `encode::Mates`).
    property var records: []
    /// Full weight on the rule — the owner sets this while its card is up.
    property bool lit: false
    /// How wide the first name may run before it elides; 0 leaves it its own width. A trailer's name is whatever the
    /// message says it is — git enforces no length — and this line reports its own width to whoever holds it, so an
    /// unbounded name widened both holders: the hover card went from a share of the graph pane to 1,022px over it
    /// (measured), and the details pane's date row runs past its column the same way. The owner passes its share, the
    /// way the card passes one for the message.
    property real nameWidth: 0
    /// Write them all out instead: the names alone. For a holder wide enough to name
    /// everybody, where the addresses are a pane away.
    property bool plain: false
    /// The pointer entered or left the underlined stretch.
    signal pointerChanged(bool inside)
    /// What the cut mark is drawn on, since the names are a field that clips
    /// (`LineText`): the pane's ground here, a card's ground where a card holds this line.
    property color ground: Theme.bgBase
    /// The names did not fit what the holder gave them. Nothing is drawn differently for it — the mark already says
    /// so — but a headless run cannot see a mark, and the width rule is the whole of the plain form.
    readonly property bool clipped: names.clipped
    /// Automation: drag cannot be injected, so a run selects the field the way `Ctrl+A` does and reads it back.
    function selectNames() { names.selectAll() }
    readonly property alias namesSelected: names.selected
    /// The one value this line draws, for a sweep over the row that holds it — empty where nobody is credited, since
    /// a line with nothing in it is not a place to land (`SweepRoom`).
    readonly property var valueFields: line.records.length > 0 ? [names] : []

    function nameAt(i) {
        const record = line.records[i]
        return record === undefined ? "" : record.name
    }
    function faceAt(i) {
        const record = line.records[i]
        return record === undefined ? 0 : record.face
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
        // A field: what this line credits is a person, and a person's name is something the reader
        // takes away (規約 §右のペインの字は掴める). A field has no `elide`, so the cut is a clip with a mark on the
        // holder's ground — the names in full are one hover away in the card either holder opens.
        LineText {
            id: names
            text: line.plain ? line.allNames() : line.nameAt(0)
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
            ground: line.ground
            Layout.alignment: Qt.AlignVCenter
            // Written out, the names take the room the holder gives them and stop there; as a chip, the name gives way
            // at the share its holder passed. Either way the name is what yields when the room runs out -- the face and
            // the count are one glyph each and have nothing to give.
            Layout.fillWidth: true
            Layout.maximumWidth: line.plain || line.nameWidth <= 0 ? Number.POSITIVE_INFINITY : line.nameWidth
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
