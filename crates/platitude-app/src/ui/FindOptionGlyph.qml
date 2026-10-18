import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One mark inside the find bar's box (デザイン規約 §diff の中を探す): an option that is on or off (`case` / `word` /
// `regex`, the glyphs editors use for them) or the new-line press. Every mark has the same seat — the box's inner
// height, square — whatever its ink is wide, so the four read as one row of keys. An option that is on wears a plain
// frame a step inside its seat, and its ink goes to `textPrimary`: no colour, as the band's toggles have none.
HoverToolButton {
    id: glyph

    /// `newline` / `case` / `word` / `regex`.
    required property string mark
    /// The option is on.
    property bool on: false
    /// The seat's side: the box's one-line height, handed in by the bar so the frame sits the same above and below
    /// the letters on every face (a face's line runs past `iconLg` on some machines).
    property real side: Theme.iconLg - 2 * Theme.borderWidth

    readonly property color ink: glyph.on ? Theme.textPrimary : Theme.textSecondary

    implicitWidth: glyph.side
    implicitHeight: glyph.side
    padding: 0
    // The frame and the wash a step inside the seat (`iconMd` square), so an `on` frame has air to the box's frame
    // and to its neighbours.
    topInset: Theme.borderWidth
    bottomInset: Theme.borderWidth
    leftInset: Theme.borderWidth
    rightInset: Theme.borderWidth
    background: Rectangle {
        radius: Theme.radiusSm
        color: glyph.washColor
        border.width: glyph.on ? Theme.borderWidth : 0
        border.color: Theme.textPrimary
    }
    contentItem: Item {
        // The two lettered marks (editors' own glyphs for these options, not words): a grade under the box's text,
        // so `Aa` keeps inside the same seat as `W`.
        Label {
            visible: glyph.mark === "case" || glyph.mark === "word"
            anchors.fill: parent
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            text: glyph.mark === "case" ? "Aa" : "W"
            color: glyph.ink
            font.pixelSize: Theme.fontSm
        }
        // The drawn marks, level with the letters and no wider than them: a dot and a star for a pattern, a corner
        // arrow for a new line.
        Canvas {
            visible: glyph.mark === "regex" || glyph.mark === "newline"
            anchors.centerIn: parent
            width: Theme.iconMd
            height: Theme.iconMd
            readonly property color tone: glyph.ink
            onToneChanged: requestPaint()
            onPaint: {
                const c = getContext("2d")
                c.clearRect(0, 0, width, height)
                c.strokeStyle = tone
                c.fillStyle = tone
                c.lineWidth = 1.5
                c.lineJoin = "round"
                c.lineCap = "round"
                c.beginPath()
                if (glyph.mark === "regex") {
                    c.arc(4.5, 11, 1.25, 0, 2 * Math.PI)
                    c.fill()
                    c.beginPath()
                    for (let i = 0; i < 3; i++) {
                        const a = Math.PI / 2 + i * Math.PI / 3
                        c.moveTo(10 - 2.6 * Math.cos(a), 8.5 - 2.6 * Math.sin(a))
                        c.lineTo(10 + 2.6 * Math.cos(a), 8.5 + 2.6 * Math.sin(a))
                    }
                } else {
                    c.moveTo(11, 5.5); c.lineTo(11, 11); c.lineTo(5, 11)
                    c.moveTo(7.5, 8.5); c.lineTo(5, 11); c.lineTo(7.5, 13.5)
                }
                c.stroke()
            }
        }
    }
}
