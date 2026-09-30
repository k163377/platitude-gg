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

    // ---- folding the name away (規約 §co-author の表示) ----
    /// The fewest characters a name is cut to, with its mark — the chip names' floor (`GraphColumnMetrics`), measured
    /// the same way (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」). A name shorter than that is its own floor.
    readonly property int nameMinChars: 3
    readonly property TextMetrics letterInk: TextMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        text: "n"
    }
    readonly property TextMetrics cutInk: TextMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        text: "…"
    }
    readonly property real nameFloor: Math.min(names.implicitWidth,
        Math.ceil(line.nameMinChars * line.letterInk.advanceWidth + line.cutInk.advanceWidth))
    /// The count as it stands beside the name, which the name's room is measured without.
    readonly property TextMetrics countInk: TextMetrics {
        font: countLabel.font
        text: "+" + (line.records.length - 1)
    }
    readonly property real countSeat: line.records.length > 1 ? Theme.spaceXs + Math.ceil(line.countInk.advanceWidth)
                                                              : 0
    /// The name's width at the most: its own, or `nameWidth` where that caps it.
    readonly property real nameNatural: line.nameWidth > 0 ? Math.min(names.implicitWidth, line.nameWidth)
                                                            : names.implicitWidth
    /// What the line asks of its holder whether or not it folds: folding on the width it is handed must not change
    /// what it asks for, or the two chase each other.
    readonly property real naturalWidth: Theme.iconMd + Theme.spaceXs + Math.ceil(line.nameNatural) + line.countSeat
    /// Folded, the face and the count: the least the line can be drawn in, which a holder keeps for it
    /// (`CommitAuthorRow`'s date gives way first).
    readonly property real foldedWidth: Theme.iconMd + line.countSeat
    /// The holder cannot give the name even its floor: the name goes and the face stands for the first, the count as
    /// it was — "◯ +2", or the face alone for one. The card still names them all.
    readonly property bool folded: !line.plain && line.records.length > 0
        && line.width - Theme.iconMd - Theme.spaceXs - line.countSeat < line.nameFloor
    /// The names did not fit — for a headless run, which cannot see the mark. Folded counts: nobody is named.
    readonly property bool clipped: line.folded || names.clipped
    /// Automation: the count as drawn, and how wide the drawn stretch is (the rule and the hover reach no further).
    readonly property alias countSaid: countLabel.text
    readonly property bool countStands: countLabel.visible
    readonly property bool nameStands: names.visible
    readonly property real stretchWidth: stretch.width
    /// Automation: where the name and the count sit on the page, in `item`'s y (-1 for one not drawn).
    function baselines(item) {
        return {
            name: names.visible ? names.mapToItem(item, 0, names.baselineOffset).y : -1,
            count: countLabel.visible ? countLabel.mapToItem(item, 0, countLabel.baselineOffset).y : -1,
            countSize: countLabel.font.pixelSize
        }
    }
    /// Automation: drag cannot be injected, so a run selects the field the way `Ctrl+A` does and reads it back.
    function selectNames() { names.selectAll() }
    readonly property alias namesSelected: names.selected
    /// The one value this line draws, for a sweep over its row (`SweepRoom`); empty where nobody is credited, or named.
    readonly property var valueFields: line.records.length > 0 && !line.folded ? [names] : []

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
    implicitWidth: line.plain ? row.implicitWidth : line.naturalWidth
    implicitHeight: row.implicitHeight
    /// Where the name's baseline is, or would be once folded away (centred in the row as the layout centres it): the
    /// count and a holder's other words sit on it (`CommitAuthorRow`'s date).
    readonly property real nameBase: names.visible ? names.y + names.baselineOffset
                                                   : (row.height - names.implicitHeight) / 2 + names.baselineOffset
    baselineOffset: line.nameBase
    /// The figures' ink — the date's, the count's, a capital's height — whose middle the face's middle is put on, so it
    /// stands as far over the letters as under them (規約 §co-author の表示). The line box keeps the descenders' room
    /// under the letters, so the box's middle is above theirs.
    readonly property TextMetrics figureInk: TextMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        text: "0"
    }
    /// Where the face's top goes, in the row's y: whole pixels by the box, the rest drawn (`IdentIcon.drop`).
    readonly property real faceTop: line.nameBase + line.figureInk.tightBoundingRect.y
        + line.figureInk.tightBoundingRect.height / 2 - Theme.iconMd / 2
    /// Automation: the drawn face's middle in `item`'s y (-1 for none), for a run to hold against the letters' ink.
    function faceMiddle(item) {
        return line.visible && face.visible ? face.mapToItem(item, 0, face.drop + Theme.iconMd / 2).y : -1
    }

    // What is drawn, and all the rule and the hover reach: folded, only the face and the count.
    Item {
        id: stretch
        width: line.folded ? row.implicitWidth : line.width
        height: line.height
        RowLayout {
            id: row
            anchors.fill: parent
            spacing: Theme.spaceXs
            IdentIcon {
                id: face
                visible: !line.plain
                code: line.faceAt(0)
                width: Theme.iconMd
                height: Theme.iconMd
                drop: line.faceTop - Math.floor(line.faceTop)
                Layout.alignment: Qt.AlignVCenter
                // Off the line box's middle onto the letters' (`faceTop`); a shift, so the row is not laid out again.
                transform: Translate {
                    y: Math.floor(line.faceTop) - face.y
                }
            }
            // A field, so the name can be taken away (規約 §右のペインの字は掴める).
            LineText {
                id: names
                visible: !line.folded
                text: line.plain ? line.allNames() : line.nameAt(0)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
                ground: line.ground
                Layout.alignment: Qt.AlignVCenter
                // The name is what yields when room runs out: the face and the count have nothing to give.
                Layout.fillWidth: true
                Layout.maximumWidth: line.plain || line.nameWidth <= 0 ? Number.POSITIVE_INFINITY : line.nameWidth
            }
            // Everybody after the first, whether the first is named or only faced: folding does not change the count.
            Label {
                id: countLabel
                visible: !line.plain && line.records.length > 1
                text: "+" + (line.records.length - 1)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                Layout.alignment: Qt.AlignVCenter
                // On the name's baseline: the two are set in faces that need not share one.
                transform: Translate {
                    y: line.nameBase - countLabel.y - countLabel.baselineOffset
                }
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
}
