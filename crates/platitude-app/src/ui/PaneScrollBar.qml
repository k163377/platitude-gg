import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The bar a pane owns: an opaque slab flush to the pane's inner edge and to both ends of the view
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

    rightPadding: 0
    topPadding: 0
    bottomPadding: 0
    contentItem: Rectangle {
        id: slab
        implicitWidth: Theme.navBarReach
        implicitHeight: Theme.navBarReach
        // Round on the free side only: the ends meet the frames square.
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
