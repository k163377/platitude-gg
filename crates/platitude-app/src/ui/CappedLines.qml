pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Lines that stand no taller than `cap`, the rest a scroll away — what a bar over the graph gives up when its column
// runs short (デザイン規約 §窓の床): the discard band's parts, the report's words and the question's (`RecoverBand`,
// `NoticeBar`, `AskBar`). What stands beside the lines (a press, a pill, a ✕, a form) stays where it is.
//
// The floating bar stands in the `gap` before whatever is right of the lines rather than over the line ends, and the
// lines keep their width as it comes and goes: a width that moved with it could wrap them away again (規約 §QML 実装
// ルール のバーの選び方).
Item {
    id: seat

    /// The most the lines stand at; no cap by default.
    property real cap: Infinity
    /// The room right of the lines the bar may stand in.
    property real gap: 0
    property alias spacing: lines.spacing
    default property alias lineItems: lines.data
    /// Something is cut off and a scroll away.
    readonly property bool scrolls: bar.visible

    /// Back to the first line, the bar's step in flight dropped with the lines it was sending (rules-refs/app-ui.md
    /// 「ホイールの 1 ノッチは送られる、跳ばない」) — for an owner whose lines are about something else now.
    function toTop() {
        bar.stepGlide.halt()
        view.contentY = 0
    }

    implicitWidth: lines.implicitWidth
    implicitHeight: Math.min(lines.implicitHeight, seat.cap)

    Flickable {
        id: view
        anchors.fill: parent
        anchors.rightMargin: -seat.gap
        contentWidth: seat.width
        contentHeight: lines.implicitHeight
        // The content is narrower than the view by that gap, which would let a hand drag it sideways.
        flickableDirection: Flickable.VerticalFlick
        // Takes presses and the wheel only while something is a scroll away: a hand laid under the lines' owner (the
        // question's `SweepPad`) gets the press in the lines' air through a view that is not moving anything, and a
        // drag over words that all show selects them rather than drags the view.
        interactive: bar.visible
        // Only while something is cut off: a press's wash reaches past the line it stands on, into the lines'
        // spacing, and past the first and the last there is nothing to take it off.
        clip: bar.visible
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {
            id: bar
        }
        // On the view's own frame, not in its content, which travels with the scroll.
        MiddleAutoScroll {
            parent: view
            anchors.fill: parent
            visible: bar.visible
            onDrifted: dy => view.contentY = Math.max(0, Math.min(view.contentY + dy,
                                                                  view.contentHeight - view.height))
        }
        ColumnLayout {
            id: lines
            width: seat.width
        }
    }
}
