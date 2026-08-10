import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

/// The current branch's ahead / behind, drawn rather than typed.
///
/// `↑` and `↓` are East Asian Ambiguous, so the CJK families this app
/// names hold them in a full-width cell: the space between arrow and
/// digit was that cell's leftover rather than a token of ours, and each
/// family drew its own arrow inside it — Windows a thin long one, Ubuntu
/// a heavy one with a small head (measured 2026-08-11). Same reason the
/// parent link's `←` is drawn
/// (デザイン規約 §寸法「印はフォントの字に任せない」).
RowLayout {
    id: track
    property int ahead: 0
    property int behind: 0
    property color tint: Theme.textSecondary

    /// The only gap that should be visible: the one between the two
    /// legs. Inside a leg the mark and its number are one thing.
    spacing: Theme.spaceXs

    /// One arrow and the number it counts, cut the same way as the graph
    /// row's tally (`GraphRowDelegate.Tally`) — no step between them and
    /// a seat drawn to the ink rather than the box.
    component Leg: RowLayout {
        id: leg
        required property int turn
        required property int count
        required property color hue
        spacing: 0
        Item {
            Layout.preferredWidth: legMark.inkWidth
            Layout.preferredHeight: Theme.iconXs
            NavIcon {
                id: legMark
                anchors.centerIn: parent
                kind: "arrow"
                rotation: leg.turn
                tint: leg.hue
                /// A step under `iconSm`: this mark stands beside a digit
                /// of its own rather than beside a word (規約 §寸法).
                width: Theme.iconXs
                height: Theme.iconXs
                /// The grid shrinks and the line has to shrink with it,
                /// or the mark carries more weight than the digit beside
                /// it (app-ui.md §語の隣に立つ印).
                stroke: Metrics.iconStroke * Theme.iconXs / Theme.iconMd
            }
        }
        Label {
            leftPadding: Theme.spaceXs / 2
            text: leg.count
            color: leg.hue
            font.pixelSize: Theme.fontSm
        }
    }

    Leg {
        turn: -90
        count: track.ahead
        hue: track.tint
        Layout.alignment: Qt.AlignVCenter
    }
    Leg {
        turn: 90
        count: track.behind
        hue: track.tint
        Layout.alignment: Qt.AlignVCenter
    }
}
