import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The bar a pane owns: an opaque slab flush to the pane's inner edge, an arrow at each end of the view. Worn by the
/// left menu and the right panel (user decision — ask before changing; which bar an edge wears: 規約 §QML 実装ルール
/// のバーの選び方). Its three states are named colours, not less ink (`dimsItself: false`; デザイン規約 §ペインの
/// スクロールバー). It parts from the floating bar in three ways:
///
/// Flush with the edge: touching the edge is itself what says nothing passes under here. `navBarReach` thick; the rows
/// leave it `navBarGutter` = thumb 5 + ground 3. The two corners on the edge are square, only the inner two round.
///
/// Opaque: a gutter clears the ink, not the ground, so the thumb crosses the selected row's blue — at the style's 0.75
/// the colour read there would shift, one thumb naming a different colour from place to place (デザイン規約 §色).
///
/// Its ends are the arrows' sections: the style's one step of padding at each end of the track gives way to them, and
/// the thumb runs between. The free side keeps its step: grip room, the box wider than the ink (7 against 5).
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
        // Up at once when put to use, over 200ms under a hand alone, down over 400ms — the timing of `AutoScrollBar`'s
        // own states (規約 §QML 実装ルール のバーの明るさ).
        color: paneBar.idleColor
        states: [
            State { name: "gone"; when: !paneBar.visible },
            State {
                name: "lit"
                when: paneBar.used
                PropertyChanges {
                    slab.color: paneBar.pressed ? Theme.borderStrong : Theme.borderDefault
                    paneBar.arrowInk: Theme.borderDefault
                }
            },
            State {
                name: "underHand"
                when: paneBar.underHand
                PropertyChanges {
                    slab.color: Theme.borderDefault
                    paneBar.arrowInk: Theme.borderDefault
                }
            }
        ]
        transitions: [
            Transition {
                from: "lit,underHand"
                to: ""
                ColorAnimation { duration: 400 }
            },
            Transition {
                from: ",gone"
                to: "underHand"
                ColorAnimation { duration: 200 }
            }
        ]
    }
}
