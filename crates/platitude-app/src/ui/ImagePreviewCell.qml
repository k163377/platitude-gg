import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Window
import platitude.ui

// One side (Before / After) of the diff pane's image preview; a side with no `sizeText` collapses. What a side is
// shown from and what it costs: `platitude_core::preview` and the app's `preview_images`. Headless:
// `preview*` and `preview-close` on `--preset pictures`, which wait on `settled` too — `DiffPane.diffSettled()` alone
// shoots before the decode, an empty frame under the caption.
ColumnLayout {
    id: previewCell
    /// `Before` / `After`.
    required property string caption
    /// The working-tree file's `file:` URL stamped with its read, or the `image://` a blob is held under —
    /// new for every read — so a side that moved under the pane comes back as a changed source
    /// (`DiffModel.previewOldUrl`).
    required property string url
    required property string sizeText
    /// `DiffModel.previewVector`: SVG is drawn at the size it is shown at, pixel rasters stay crisp.
    required property bool isVector
    /// A blob's side, which an image provider draws. Qt hands a provider the screen's ratio only with a `sourceSize`
    /// (`qquickimagebase.cpp`, `loadPixmap`), which a first read cannot name — its size is what that read tells — so
    /// the side asks for the scale in its URL, at both reads (`preview_images::drawn_at`); the provider reads no size.
    readonly property bool provided: previewCell.url.startsWith("image:")
    /// The zoom a drawing is drawn at — the one its own size picks (`zoomFor`) — so an enlarged drawing is drawn at
    /// the size it is shown at, not stretched; a frame too small for it shrinks that drawing and never draws it again.
    /// Nothing tells a drawing's own size before its first answer, drawn at that size: the zoom is learnt from it and
    /// the drawing asked for again (`learnZoom`) — a blob's second read, for a drawing under 128px only. 1 for pixels,
    /// which are shown as decoded, and before that answer.
    readonly property int drawnZoom: previewCell.isVector && previewCell.zoomLearntFor === previewCell.url
                                     ? previewCell.zoomLearnt : 1
    /// The read whose drawing's zoom is learnt, the zoom, and the drawing's own size it was learnt from — kept
    /// apart from the picture, whose size moves with the drawing asked for.
    property string zoomLearntFor: ""
    property int zoomLearnt: 1
    property size ownLearnt: Qt.size(0, 0)
    /// How many of the picture's pixels a provided drawing is drawn at to one of its own: the screen's ratio times its
    /// zoom, since Qt reads a picture asked for with no `sourceSize` at a ratio of 1. A file's drawing Qt draws at the
    /// screen's ratio and divides it back out itself (`QQuickImageProviderWithOptions::loadSize`), its zoom asked as a
    /// size (`sourceSize`). 1 for those and for pixels.
    readonly property real drawnAt: previewCell.isVector && previewCell.provided
                                    ? Screen.devicePixelRatio * previewCell.drawnZoom : 1
    /// The picture is the last one this side asks for: pixels, a drawing at its zoom, or one that needs none.
    readonly property bool drawn: previewImage.status === Image.Ready
                                  && (!previewCell.isVector || previewCell.drawnZoom > 1
                                      || previewCell.zoomFor(Math.max(previewFrame.naturalW, previewFrame.naturalH))
                                         === 1)
    /// Automation, since the decode is asynchronous: `settled` = the decoder has answered either way, `shown` = the
    /// picture is there.
    readonly property bool settled: previewCell.drawn || previewImage.status === Image.Error
    readonly property bool shown: previewCell.drawn

    // A whole-step zoom from the natural size alone (not the window), landing small icons in a 128-256px band.
    function zoomFor(maxSide) {
        if (maxSide <= 0)
            return 1
        if (maxSide < 32)
            return 8
        if (maxSide < 64)
            return 4
        if (maxSide < 128)
            return 2
        return 1
    }
    // A drawing's first answer, drawn at its own size, tells the zoom to draw it at (`drawnZoom`). Its size is read
    // here because Qt sets the implicit size ahead of the status.
    function learnZoom() {
        if (previewImage.status !== Image.Ready || !previewCell.isVector || previewCell.drawnZoom !== 1)
            return
        const zoom = previewCell.zoomFor(Math.max(previewFrame.naturalW, previewFrame.naturalH))
        if (zoom === 1)
            return
        previewCell.ownLearnt = Qt.size(Math.round(previewFrame.naturalW), Math.round(previewFrame.naturalH))
        previewCell.zoomLearnt = zoom
        // Last: the drawing is asked for again once, with the size and zoom in place.
        previewCell.zoomLearntFor = previewCell.url
    }

    visible: sizeText !== ""
    spacing: Theme.spaceXs

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        PlainTextLabel {
            id: captionWord
            text: previewCell.caption
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
        }
        // The `·` between the words, drawn in its place on the face's own `+`'s middle (デザイン規約 §アイコンの席 の
        // 字の代替).
        DotMark {
            tint: Theme.textSecondary
            Layout.preferredHeight: captionWord.implicitHeight
            middle: captionWord.baselineOffset - WordFace.signMiddle(captionWord.font.pixelSize)
        }
        PlainTextLabel {
            Layout.fillWidth: true
            text: previewCell.sizeText
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
            elide: Text.ElideRight
        }
    }
    Rectangle {
        id: previewFrame
        Layout.fillWidth: true
        Layout.fillHeight: true
        color: "transparent"
        border.color: Theme.borderSubtle
        border.width: Theme.borderWidth
        clip: true
        readonly property real innerW: width - 2 * Theme.spaceXs
        readonly property real innerH: height - 2 * Theme.spaceXs
        // Decoded size (0 until the image is ready), in the picture's own pixels; a drawing asked for again at its
        // zoom keeps the size learnt, so the picture coming in at that zoom does not move it.
        readonly property real naturalW: previewCell.drawnZoom > 1 ? previewCell.ownLearnt.width
                                         : previewImage.implicitWidth / previewCell.drawnAt
        readonly property real naturalH: previewCell.drawnZoom > 1 ? previewCell.ownLearnt.height
                                         : previewImage.implicitHeight / previewCell.drawnAt
        readonly property real fitScale: naturalW > 0 && naturalH > 0 && innerW > 0 && innerH > 0
            ? Math.min(innerW / naturalW, innerH / naturalH) : 1
        // Shrink freely; enlarge only in whole steps, within the frame and up to the size-picked zoom.
        readonly property real displayScale: fitScale < 1
            ? fitScale
            : Math.max(1, Math.min(previewCell.zoomFor(Math.max(naturalW, naturalH)), Math.floor(fitScale)))
        Image {
            id: previewImage
            anchors.centerIn: parent
            width: previewFrame.naturalW * previewFrame.displayScale
            height: previewFrame.naturalH * previewFrame.displayScale
            fillMode: Image.PreserveAspectFit
            source: previewCell.provided && previewCell.drawnAt !== 1 ? previewCell.url + "@" + previewCell.drawnAt
                                                                      : previewCell.url
            asynchronous: true
            // A closed pane keeps no picture in the pixmap cache. So every draw of a blob side is a `git cat-file`
            // of its own (`preview::PictureReader`).
            cache: false
            // A file's drawing at its zoom, which Qt draws at the screen's ratio too. Never a raster's: it rescales
            // the stored pixels at decode, blurring small icons. Never a blob's: that side asks for its scale in its
            // URL at both reads, and the provider reads no size (`provided`).
            sourceSize: !previewCell.provided && previewCell.drawnZoom > 1
                        ? Qt.size(previewCell.ownLearnt.width * previewCell.drawnZoom,
                                  previewCell.ownLearnt.height * previewCell.drawnZoom)
                        : undefined
            smooth: previewFrame.displayScale < 1 || previewCell.isVector
            mipmap: true
            // Not the first answer of a drawing that is asked for again: it would show stretched for a moment.
            visible: previewCell.drawn
            onStatusChanged: previewCell.learnZoom()
        }
        // A side with nothing to show: a blob core could not read, or a format this runtime has no decoder for.
        Label {
            anchors.centerIn: parent
            width: Math.min(implicitWidth, parent.width - 2 * Theme.spaceSm)
            visible: previewCell.url === "" || previewImage.status === Image.Error
            text: qsTr("Preview unavailable")
            elide: Text.ElideRight
            color: Theme.textMuted
        }
    }
}
