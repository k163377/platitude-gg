import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Whoever a commit message credits alongside its author, in one of two forms (規約 §co-author の表示): "◯ Name +N"
// in the details pane's date row, or "Name, Name, Name" (`plain`) in the graph row's hover card. Nobody credited,
// nothing drawn: an ordinary state wears no mark.
//
// The pane's form: an `iconMd` identicon, the first name and `+N`, in `fontSm` `textSecondary` like the date. Folded to
// one name and a count, so the line keeps its width however many are credited; everyone, with addresses (the one clue
// that tells two people of one name apart), is on the card it opens (`CoAuthorCard`, `RefListPopup`'s ground, frame and
// row height) — this form is the pane's alone. Its face pairs with the author's avatar and reads as the face column.
// The pane's date row has no room to write everyone (about 230px), and the count goes unsaid only when it overflows:
// naming everyone is the pane's job, so that is accepted.
//
// The hover card's form: the names with commas, in the same `fontSm` `textSecondary`, with no face, no `+N`, no
// underline, and nothing opens. The card has no picture at all (author and date are words), so a face would be its only
// hue, on the least important of its four lines; and the place to check is the right pane — a preview that grows a
// second popup starts acting as the pane.
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

    // ---- folding the name away (user decisions — ask before changing) ----
    // Where the date row cannot give the name even its floor, the name goes: "◯ +2", the face standing for the first
    // and `+N` as it was before folding — a count that changes as the name goes reads as a different number of people.
    // One person folds to the face alone. A name cut to nothing would be a stray `…` or an underline over a gap, naming
    // nobody and looking broken. It happens near the right pane's least width, and on a commit with two or more parents
    // the parent line's floor takes its room first (`CommitAuthorRow`).
    /// The fewest characters a name is cut to, with its mark — the chip names' floor (`GraphColumnMetrics`), measured
    /// the same way (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」). A name shorter than that is its own floor.
    /// A named line needs one: a `LineText` cut to 0 width draws its `…` ground outside the field — a stray `·`, or
    /// an underline under nothing.
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
    /// what it asks for, or the two chase each other: the holder hands this, rounded up, as the line's
    /// `Layout.maximumWidth` (`CommitAuthorRow.creditCeiling`), so asking the folded width would shrink what is handed
    /// and the line would stay folded once room came back.
    readonly property real naturalWidth: Theme.iconMd + Theme.spaceXs + Math.ceil(line.nameNatural) + line.countSeat
    /// Folded, the face and the count: the least the line can be drawn in, which a holder keeps for it
    /// (`CommitAuthorRow`'s date gives way first).
    readonly property real foldedWidth: Theme.iconMd + line.countSeat
    /// The holder cannot give the name even its floor: the name goes and the face stands for the first, the count as
    /// it was — "◯ +2", or the face alone for one. The card still names them all. Judged on what the handed `width`
    /// leaves the name past the face and the count. Whether it folds splits by the OS font (two parents × one mate at
    /// `14:min` folds on Windows, Linux keeps four letters), so a check names only forms that fold on both. Pinned by
    /// `tst_matefold.qml`; headless `co-authors 0:min` / `co-authors 8:min --preset octopus-mates`.
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
    /// Automation: the drawn face's middle in `item`'s y (-1 for none), for a run to hold against the date's
    /// capitals' middle.
    function faceMiddle(item) {
        return line.visible && face.visible ? face.mapToItem(item, 0, Theme.iconMd / 2).y : -1
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
                // `iconMd`, not sized to the names' letters (a special design, user decision — ask before changing).
                width: Theme.iconMd
                height: Theme.iconMd
                // A picture heading the names, on their capitals' middle (デザイン規約 §アイコンの席). Checked by
                // `details-align`'s `face=` (the date's baseline − `WordFace.capitalMiddle` against the drawn face's
                // middle, within 0.5px; the value is `faceOff=`), not by this placement's own numbers.
                Layout.alignment: Qt.AlignVCenter
                transform: Translate {
                    y: WordFace.capitalDropOf(names.baselineOffset, names.pixelSize, names.height)
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
        // At rest one step down from the name it underlines (規約 §暗く落とした段), so it does not compete with the date;
        // the name's own colour when lit. One line from the face to `+N`.
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
