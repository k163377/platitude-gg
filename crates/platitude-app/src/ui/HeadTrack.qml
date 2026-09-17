import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

/// A branch's ahead / behind against its upstream, drawn, arrows included.
///
/// `↑` and `↓` are East Asian Ambiguous, so the CJK families this app names hold them in a full-width cell: the space
/// between arrow and digit was that cell's leftover, and each family drew its own arrow
/// inside it — Windows a thin long one, Ubuntu a heavy one with a small head (measured). Same reason the
/// parent link's `←` is drawn (デザイン規約 §寸法「印は描いて出す」).
///
/// **Only a leg that counts is drawn** — git spells its own answer the same way (`[ahead 2]`, never
/// `[ahead 2, behind 0]`), and whoever seats this leaves the seat empty when neither leg has anything to say
/// (デザイン規約 §左メニューの所作). What is left is never a zero, so the numbers here are always the point.
RowLayout {
    id: track
    property int ahead: 0
    property int behind: 0
    property color tint: Theme.textSecondary

    /// **The pair is one expression, so one colour covers it.** Commits arrived on the far side are what the reader
    /// has to do something about before this branch can be sent, which is the standing a state colour is for
    /// (デザイン規約 §状態 —— 分岐しているは常設でよい); the toolbar paints the same fact the same way when it turns
    /// `push` into `push -f`. Commits of one's own with nothing on top of them are the ordinary way to stand, and
    /// wear no state at all.
    readonly property color hue: track.behind > 0 ? Theme.warning : track.tint

    /// The only gap that should be visible: the one between the two legs. Inside a leg the mark and its number are one
    /// thing.
    spacing: Theme.spaceXs

    /// One arrow and the number it counts, cut the same way as the graph row's tally (`GraphRowDelegate.Tally`) — no
    /// step between them and a seat drawn to the ink.
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
                /// A step under `iconSm`: this mark stands beside a digit of its own (規約
                /// §寸法).
                width: Theme.iconXs
                height: Theme.iconXs
                /// The grid shrinks and the line has to shrink with it, or the mark carries more weight than the digit
                /// beside it (app-ui.md §語の隣に立つ印).
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
