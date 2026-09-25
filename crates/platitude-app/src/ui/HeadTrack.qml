import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

/// A branch's ahead / behind against its upstream, arrows drawn rather than set as `↑` / `↓` (East Asian Ambiguous,
/// full-width in the CJK families — デザイン規約 §寸法「印は描いて出す」). Only a nonzero leg is drawn, as git spells
/// `[ahead 2]`; whoever seats this leaves it empty when both are zero (デザイン規約 §左メニューの所作).
RowLayout {
    id: track
    property int ahead: 0
    property int behind: 0
    property color tint: Theme.textSecondary

    /// One colour for the pair: `warning` once anything is behind — something to act on before a push
    /// (デザイン規約 §状態) — and plain otherwise.
    readonly property color hue: track.behind > 0 ? Theme.warning : track.tint

    /// The gap between the legs; inside a leg, mark and number are one thing.
    spacing: Theme.spaceXs

    /// One arrow and its count, cut like the graph row's tally (`WipTallyRow.Tally`): no step between them, and a
    /// seat drawn to the ink.
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
                /// A step under `iconSm`: a mark paired with a digit (規約 §寸法).
                width: Theme.iconXs
                height: Theme.iconXs
                /// Line scaled with the grid (rules-refs/app-ui.md「語の隣に立つ印は `iconSm`」).
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
        hue: track.hue
        visible: track.ahead > 0
        Layout.alignment: Qt.AlignVCenter
    }
    Leg {
        turn: 90
        count: track.behind
        hue: track.hue
        visible: track.behind > 0
        Layout.alignment: Qt.AlignVCenter
    }
}
