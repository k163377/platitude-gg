pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The view's place in a diff about to be laid out again as one column or two (`DiffModel.relay_rows`): the rows come
// back as a whole new list, which would put the view back at the top (デザイン規約 §diff の中のステージ「diff の作り直し
// でも画面はその場」). A read of the same file moves only the rows that changed and keeps the place by itself
// (`DiffModel.lay_out_kept`, `DiffPane.rowKeep`).
Item {
    id: place

    required property var view

    property real heldY: -1
    function hold() {
        // A press inside the restore's beat finds the view at the reset-to-top: keep the place already held, and
        // let the next swap's restore finish (rules-refs/app-ui.md「`DiffScrollPlace` の位置は別のファイルへ移る時に捨てる」).
        placeTimer.stop()
        if (place.heldY < 0)
            place.heldY = place.view.contentY
    }
    function restore() {
        if (place.heldY >= 0)
            placeTimer.restart()
    }
    /// The pane moved to another file (or another side of the same one): the held place is the old diff's
    /// (規約 §diff を横へ送る「別のファイルは左端から」— its vertical half).
    function drop() {
        placeTimer.stop()
        place.heldY = -1
    }
    // On the frame the rows land `contentHeight` is still the old one, and the clamp below would take the view to the
    // top (rules-refs/app-ui.md「モデルリセット後の ListView は」).
    Timer {
        id: placeTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            // A wheel notch still in flight was aimed into the rows this is putting back (`WheelGlide.halt`).
            place.view.haltGlide()
            const want = place.heldY
            place.heldY = -1
            place.view.contentY = Math.max(0, Math.min(want, place.view.contentHeight - place.view.height))
        }
    }
    /// Automation: moves the view off the top, so a rebuild can be seen to put it back.
    function scrollTo(y) {
        place.view.contentY = y
    }
}
