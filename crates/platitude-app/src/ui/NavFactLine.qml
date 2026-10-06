import QtQuick
import QtQuick.Layouts
import platitude.ui

// One line of what a sidebar row opens under itself: a mark in the row's mark column, the words beginning where its
// name does, and — for a branch — how far it stands from what it is measured against (デザイン規約 §左メニューの所作).
// The seat is held open with or without a mark, so every line's words begin in one column (as in `NameCell`).
//
// The words are a field, not a label (規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」); the
// hand that sweeps them is the pad under the block (`SweepPad`).
//
// A line whose words stand on a commit of their own is the way to it (`goes`). The block takes the press and lays the
// band (`NavRowFacts.aimRow`); this part only draws what it is told.
RowLayout {
    id: line

    /// `NavIcon.kind`, empty where this line leads with nothing.
    property string mark: ""
    property color markTint: Theme.textSecondary
    property string text: ""
    property color tone: Theme.textSecondary
    property int weight: Font.Normal
    property real pixelSize: Theme.fontSm
    /// How far the branch this line names stands from what it reads; both zero draws nothing.
    property int ahead: 0
    property int behind: 0
    /// Whether a rest on this line opens a supplement — why it wears its colour (デザイン規約 §hover のツールチップ).
    /// The block raises the words, so one pointer opens one thing (`NavRowFacts`); this only says the hand is here.
    property bool noted: false
    /// Stands in for the pointer headless cannot put (PGG_AUTO_ACT=nav-open-tag `:tip`).
    property bool tipPointed: false
    readonly property bool pointed: lineHover.hovered || line.tipPointed
    /// The hand arrived at this line, or left it.
    signal handRested(bool on)
    onPointedChanged: line.handRested(line.pointed)
    /// Whether the line is a way to the commit its name stands on, and whether the hand is on it
    /// (`NavRowFacts.aimRow`). Said by the pointer, not at rest: under the hand the words wear their own colour's
    /// underline, over the block's band.
    property bool goes: false
    property bool aimed: false
    /// How far the words run across the field: the field fills the line, and a short name is a sliver of it.
    readonly property real wordsWidth: Math.min(words.implicitWidth, words.width)
    /// The middle of the words in another item's coordinates, where a run puts the press it cannot make with a hand
    /// (PGG_AUTO_ACT=nav-follow).
    function wordsMiddle(item) {
        return line.mapToItem(item, words.x + line.wordsWidth / 2, words.y + words.height / 2)
    }

    spacing: Theme.spaceXs

    // A handler, not an area, so the block's sweep keeps its pointer
    // (rules-refs/app-ui.md「行の hover は `HoverHandler`」). On the line itself: on an item stacked over the others,
    // it would take the hover off every line below.
    HoverHandler {
        id: lineHover
        enabled: line.noted
    }

    // The row's own seat, to the pixel (`NameCell.seatSize` / `seatNudge`), so the words begin
    // where the row's name does; sized by the mark instead, they land right of it. A TAGS row has no seat
    // (`NavRowBody.seated`): its lines' cloud stands under the name's head and the words one seat in.
    Item {
        Layout.preferredWidth: Theme.iconXs
        Layout.preferredHeight: Theme.iconXs
        // Built only on the lines that wear one (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
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
    // The words and the ahead / behind on one baseline: the words are a field, which files the face's leading under its
    // line, so centred beside the numbers they stand half of it high (3px on macOS).
    CardText {
        id: words
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignBaseline
        text: line.text
        pixelSize: line.pixelSize
        color: line.tone
        weight: line.weight
        underline: line.aimed
    }
    Loader {
        id: trackSeat
        active: line.ahead > 0 || line.behind > 0
        visible: trackSeat.active
        baselineOffset: trackSeat.item ? trackSeat.item.baselineOffset : 0
        Layout.alignment: Qt.AlignBaseline
        sourceComponent: HeadTrack {
            ahead: line.ahead
            behind: line.behind
        }
    }
}
