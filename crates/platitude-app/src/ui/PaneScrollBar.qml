import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The bar a pane owns: an opaque slab flush to the pane's inner edge, an arrow at each end of the view
/// (デザイン規約 §QML 実装ルール「左メニューと右パネルのバーは張り付くバー」). Its three states are named colours, not
/// less ink (`dimsItself: false`; デザイン規約 §ペインのスクロールバー).
AutoScrollBar {
    id: paneBar

    /// The idle step — the one above the ground this bar stands on. On a ground that is `bgElevated` itself (the
    /// settings screen) the default vanishes, so that screen hands in the next step up (デザイン規約 §ペインのスクロールバー).
    property color idleColor: Theme.bgElevated

    dimsItself: false

    /// What the slab is painting, for a run to read back — the painted side, so a cut binding cannot read as green.
    readonly property alias slabColor: slab.color

    // The slab is half of a thumb twice its reach, sunk into the edge; its arrows are half of that thumb's, sunk the
    // same way, in the slab's idle and lit steps (デザイン規約 §スクロールバーの矢印). The held step is the held
    // arrow's alone — a thumb in the hand lights the slab, not the arrows (the lit state below).
    arrowSpan: 2 * Theme.navBarReach
    arrowsBuried: true
    arrowInk: paneBar.idleColor
    arrowHeldInk: Theme.borderStrong
    arrowOpacity: 1

    rightPadding: 0
    topPadding: paneBar.arrowEnd
    bottomPadding: paneBar.arrowEnd
    contentItem: Rectangle {
        id: slab
        implicitWidth: Theme.navBarReach
        implicitHeight: Theme.navBarReach
        // Round on the free side only: the other is sunk into the edge.
        radius: Theme.radiusSm
        topRightRadius: 0
        bottomRightRadius: 0
        // Up at once, down over 400ms (規約 §QML 実装ルール のバーの明るさ).
        color: paneBar.idleColor
        states: [
            State { name: "gone"; when: !paneBar.visible },
            State {
                name: "lit"
                when: paneBar.bright
                PropertyChanges {
                    slab.color: paneBar.pressed ? Theme.borderStrong : Theme.borderDefault
                    paneBar.arrowInk: Theme.borderDefault
                }
            }
        ]
        transitions: Transition {
            from: "lit"
            to: ""
            ColorAnimation { duration: 400 }
        }
    }
}
