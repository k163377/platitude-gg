import QtQuick
import QtQuick.Layouts
import platitude.ui

// One line of what a sidebar row opens under itself: a mark in the row's own mark column, the words beginning where
// its name does, and — where the line names a branch — how far that branch stands from what it is measured against
// (デザイン規約 §左メニューの所作).
//
// **The seat is held open whether or not a mark stands in it**, which is what puts every line's words in one column:
// the name in full has no mark and still begins where the marked lines do. It is the same rule the rows themselves
// keep (`NameCell`).
//
// The words are a field, not a label — what the row opens is there to be taken away
// (規約 §hover のツールチップ「出したものは持ち帰れる」), and the hand that sweeps them is the pad under the whole of
// it (`SweepPad`).
RowLayout {
    id: line

    /// `NavIcon.kind`, empty where this line leads with nothing.
    property string mark: ""
    property color markTint: Theme.textSecondary
    property string text: ""
    property color tone: Theme.textSecondary
    property int weight: Font.Normal
    property real pixelSize: Theme.fontSm
    /// How far the branch this line names stands from what it reads. Both zero draws nothing — the same seat the
    /// rows leave empty when there is no count worth a column.
    property int ahead: 0
    property int behind: 0
    /// Whether this line keeps a supplement for a rest on it — **the reason it is wearing the colour it is**
    /// (デザイン規約 §hover のツールチップ: 的が印や色を着ていて、その理由が的の上に無い時). The words are the
    /// block's to raise, so that one pointer opens one thing (`NavRowFacts`); this says who the hand is on.
    property bool noted: false
    /// Stands in for the pointer where headless cannot put one, the way the row's own does
    /// (PGG_AUTO_ACT=nav-open-tag `:tip`).
    property bool tipPointed: false
    readonly property bool pointed: lineHover.hovered || line.tipPointed
    /// The hand arrived at this line, or left it.
    signal handRested(bool on)
    onPointedChanged: line.handRested(line.pointed)

    spacing: Theme.spaceXs

    // **A handler, not an area** — handlers are passive, so the one that drives the sweep over the whole block goes
    // on being answered while this reads the same pointer (rules-refs/app-ui.md 「行の hover は `HoverHandler`」).
    // Declared on the line itself: put on an item stacked over the others, it would take the hover off every line
    // below it (app-ui.md).
    HoverHandler {
        id: lineHover
        enabled: line.noted
    }

    // **The row's own seat, to the pixel** (`NameCell.seatSize` / `seatNudge` on a list with no change codes): the
    // seat is the trimmed one and the mark is drawn a step wider inside it, hard against its left edge, so the mark
    // lands in the row's own mark column and the words begin exactly where the row's name begins. Measuring this
    // seat by the mark instead put every line here 2px right of the name it belongs to (observed). **A TAGS row has
    // no seat** (`NavRowBody.seated`), and its every line leads with a cloud: the mark stands under the name's head
    // and the words one seat in.
    Item {
        Layout.preferredWidth: Theme.iconXs
        Layout.preferredHeight: Theme.iconXs
        // Built only on the lines that wear one: a mark is a canvas, and one built and hidden on every line is heap
        // the rows are measured by (rules-refs/app-ui.md, the Loader rule).
        Loader {
            id: markSeat
            active: line.mark !== ""
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: (Theme.iconSm - Theme.iconXs) / 2
            sourceComponent: NavIcon {
                kind: line.mark
                tint: line.markTint
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
    }
    CardText {
        Layout.fillWidth: true
        text: line.text
        pixelSize: line.pixelSize
        color: line.tone
        weight: line.weight
    }
    Loader {
        id: trackSeat
        active: line.ahead > 0 || line.behind > 0
        visible: trackSeat.active
        Layout.alignment: Qt.AlignVCenter
        sourceComponent: HeadTrack {
            ahead: line.ahead
            behind: line.behind
        }
    }
}
