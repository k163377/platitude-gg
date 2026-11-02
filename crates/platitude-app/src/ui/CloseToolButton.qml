import QtQuick
import platitude.ui

// The `✕` that closes a pane, a bar or a tab: an iconLg seat around an iconSm mark (デザイン規約 §寸法). The caller
// writes only what varies: onClicked, and an Accessible.name or an opacity where its host has one to add.
//
// The seat stays `iconLg`, so dropping the mark a step moves no layout. `NavIcon` shrinks the coordinates and keeps the
// stroke, so at 12px the line reads heavier: kept, so the closing marks of every bar are the same pixels
// (`NavIcon.stroke` left at its default — `ActionButton.besideWord`'s thinner line is for a mark beside a word; a mark
// standing alone keeps the same pixels). Not the window's `✕` (`WindowButton`), which keeps its step beside minimise /
// maximise in the OS chrome's seat.
//
// Whatever stands beside it takes off the air the seat holds outside the mark's ink, asked of a `NavIcon` of its own
// (`TabMetrics.markAir` = `width − inkRight`), and writes `step − air` on both sides of the mark — on one side only,
// just the mark's neighbour is spaced (デザイン規約 §余白). Not read off the button: the side that sizes the band
// (`TabStrip`'s `markRoomFull`) needs the number before any mark is built.
HoverToolButton {
    id: closeButton

    /// How big a target this is. A caller with room around it hands in a larger one, and the wash stays `iconLg`
    /// (規約 §QML 実装ルール「広げるのは判定だけ」).
    property real seat: Theme.iconLg
    /// How big the mark is drawn: a step under what it closes (§寸法), so beside a screen's `fontXl` title it is larger.
    property real markSize: Theme.iconSm
    /// The mark's ink; a caller whose way out carries a warning says so here (`SettingsHeader`).
    property color tone: Theme.textSecondary
    /// A press was turned down and the next one goes through. The seat wears `warningDim` so the mark reads as
    /// changed — colour alone is one signal (規約 §暗く落とした段).
    property bool armed: false

    padding: 0
    implicitWidth: closeButton.seat
    implicitHeight: closeButton.seat
    // The seat grows around the wash: `Control` shrinks the background by its insets, so the hand gets the whole seat
    // and the paint stays put. The sides follow `topInset` for that caller, whose seat is wider than the wash: a caller
    // that writes `topInset` to trim only the height writes `leftInset` / `rightInset: 0` too, or the same depth comes
    // off the sides and the wash is narrower than the ink (`TabItemDelegate`'s mark).
    topInset: (closeButton.seat - Theme.iconLg) / 2
    bottomInset: closeButton.topInset
    leftInset: closeButton.topInset
    rightInset: closeButton.topInset
    // A soft-rimmed disc, and only the paint (user decision — ask before changing): the hit area stays the seat's
    // rectangle and reaches past the disc. `✕` has no corners, so a rounded square under it reads as a frame drawn for
    // a box. The rim falls to `dimFade` over its outer quarter (`Metrics.washRimShare`): a hard edge on a wash this
    // faint would be the strongest line in the band. Whatever goes quiet under the mark keeps within this disc (a tab's
    // name, `TabTitleFade`). The armed ground goes under the style's wash, so a hand on an armed mark still lights it
    // (`HoverToolButton.washColor`).
    background: Item {
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        InkCanvas {
            id: washInk
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: washInk.width
            readonly property color armedTone: closeButton.armed ? Theme.warningDim : "transparent"
            readonly property color washTone: closeButton.washColor
            // Every input repaints by hand: a canvas does not redraw for a colour, and the old picture would stay.
            onWidthChanged: washInk.requestPaint()
            onArmedToneChanged: washInk.requestPaint()
            onWashToneChanged: washInk.requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                const r = width / 2
                for (const tone of [washInk.armedTone, washInk.washTone]) {
                    if (tone.a === 0)
                        continue
                    const coat = ctx.createRadialGradient(r, r, 0, r, r, r)
                    coat.addColorStop(0, tone)
                    coat.addColorStop(1 - Metrics.washRimShare, tone)
                    coat.addColorStop(1, Qt.rgba(tone.r, tone.g, tone.b, tone.a * Metrics.dimFade))
                    ctx.fillStyle = coat
                    ctx.beginPath()
                    ctx.arc(r, r, r, 0, 2 * Math.PI)
                    ctx.fill()
                }
            }
        }
    }
    contentItem: Item {
        NavIcon {
            anchors.centerIn: parent
            width: closeButton.markSize
            height: closeButton.markSize
            kind: "close"
            tint: closeButton.tone
        }
    }
}
