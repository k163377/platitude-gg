import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Whoever a commit message credits alongside its author, in one of two forms (規約 §co-author の表示): "◯ Name +N"
// in the details pane's date row, or "Name, Name, Name" (`plain`) in the graph row's hover card.
//
// It draws and reports; the card that names everyone belongs to the owner, since a popup cannot live inside a recycled
// delegate.
Item {
    id: line

    /// The co-author records (`{name, email, face}` — `encode::Mates`).
    property var records: []
    /// Full weight on the rule — the owner sets this while its card is up.
    property bool lit: false
    /// How wide the first name may run before it is cut; 0 leaves it its own width. A trailer's name has no length
    /// git enforces, and this line reports its width to its holder, so an unbounded name widens the holder.
    property real nameWidth: 0
    /// Write out the names alone, for a holder wide enough to name everybody.
    property bool plain: false
    /// The pointer entered or left the underlined stretch.
    signal pointerChanged(bool inside)
    /// What the cut mark is drawn on (`LineText`): the pane's ground, or a card's where a card holds this line.
    property color ground: Theme.bgBase
    /// The names did not fit — for a headless run, which cannot see the mark.
    readonly property bool clipped: names.clipped
    /// Automation: drag cannot be injected, so a run selects the field the way `Ctrl+A` does and reads it back.
    function selectNames() { names.selectAll() }
    readonly property alias namesSelected: names.selected
    /// The one value this line draws, for a sweep over its row (`SweepRoom`); empty where nobody is credited.
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
        // A field, so the name can be taken away (規約 §右のペインの字は掴める).
        LineText {
            id: names
            text: line.plain ? line.allNames() : line.nameAt(0)
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
            ground: line.ground
            Layout.alignment: Qt.AlignVCenter
            // The name is what yields when room runs out: the face and the count have nothing to give.
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
    // At rest one step down from the name it underlines (規約 §暗く落とした段), the name's own colour when lit.
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
