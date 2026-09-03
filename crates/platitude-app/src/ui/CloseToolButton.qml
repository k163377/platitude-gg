import QtQuick
import platitude.ui

// The `✕` that closes a pane, a bar or a tab: an iconLg seat around an
// iconSm mark — a step under what it closes (デザイン規約 §寸法). The
// caller writes only what varies: onClicked, and an Accessible.name or an
// opacity where its host has one to add.
HoverToolButton {
    id: closeButton

    /// How big a target this is, which is not how big the mark is drawn. The default is the seat every `✕` in the
    /// window stands in; a caller with room around it hands in a larger one, and **the wash stays `iconLg`**
    /// (規約 §当たり判定 「広げるのは判定だけ」) — a mark whose paint grew with its seat would read as a box.
    property real seat: Theme.iconLg
    /// How big the mark itself is drawn. A step under whatever it closes is the rule (§寸法), so a `✕` beside a row
    /// or a tab takes `iconSm`; the one that closes a whole screen stands beside a `fontXl` title and needs the step
    /// that goes with it — at `iconSm` the mark reads as too weak for the screen it closes (observed).
    property real markSize: Theme.iconSm
    /// What the mark is drawn in. A `✕` is normally the quiet ink; a caller whose way out is carrying a warning
    /// says so here rather than drawing its own mark (`SettingsDialog`).
    property color tone: Theme.textSecondary
    /// A press was made and turned down, and the next one goes through. The seat carries a ground of the tone's own
    /// dimmed step so the mark reads as *changed* rather than merely coloured — colour alone is one signal, and a
    /// press that did nothing needs to be answered by something (規約 §暗く落とした段).
    property bool armed: false

    /// The air this seat holds between its edge and the mark's ink, for a caller putting something right against it
    /// (デザイン規約 §余白 「印が自分で持っている余白は、隣の詰めに数える」): the neighbour writes
    /// `その行の刻み − これ` and the gap the eye measures comes out at the step. Symmetric, because the mark is
    /// centred and `close` is square in its own box. Asked of the icon rather than guessed at from a token — the
    /// diagonals are inset further than a straight-stroked mark, so no token is the right number.
    readonly property real inkAir: (closeButton.width - mark.inkWidth) / 2

    padding: 0
    implicitWidth: closeButton.seat
    implicitHeight: closeButton.seat
    // The seat grows around the wash rather than with it. `Control` shrinks the background by its insets and leaves
    // the item its own size, so the hand gets the whole seat and the paint stays where it was (after
    // `TabItemDelegate`, which does the same subtraction the other way round).
    topInset: (closeButton.seat - Theme.iconLg) / 2
    bottomInset: closeButton.topInset
    leftInset: closeButton.topInset
    rightInset: closeButton.topInset
    // **Round, and only the paint** (デザイン規約 §当たり判定): the seat above stays the square a hand is aimed at, and
    // what is drawn inside it is a disc on the mark — the ink is a `✕`, which has no corners for a box to belong to,
    // and a rounded square around it reads as a button someone drew a frame for.
    //
    // Drawn rather than a `Rectangle` with a radius, because the rim is carried out: full strength across the disc,
    // then down to the dimmed step of itself at the edge (§暗く落とした段). A hard rim on a wash this faint is a ring —
    // the one line the eye finds first in a band that has no other — and the same softness is what lets the quiet the
    // name goes into meet it without a seam (`TabTitleFade`).
    //
    // The armed ground goes under the style's own wash rather than replacing it, so a hand resting on an armed mark
    // still lights it (`HoverToolButton.washColor` is what paints on top).
    background: Item {
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        InkCanvas {
            id: washInk
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: washInk.width
            /// The two coats, in the order they are laid: the ground a refused press leaves standing, and the wash the
            /// pointer is answered with.
            readonly property color armedTone: closeButton.armed ? Theme.warningDim : "transparent"
            readonly property color washTone: closeButton.washColor
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
            id: mark
            anchors.centerIn: parent
            width: closeButton.markSize
            height: closeButton.markSize
            kind: "close"
            tint: closeButton.tone
        }
    }
}
