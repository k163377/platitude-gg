import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One side of the diff pane's image preview (Before / After). Absent sides collapse (visible tracks sizeText), so an
// added image shows a single full-width After cell and a deleted one a single Before.
//
// Scaling: 1:1 when the natural size fits, a fixed integer zoom for small images (zoomFor — independent of the window),
// and fit-to-frame shrinking as the final cap, so a tiny window keeps its zoom and
// only shrinks.
ColumnLayout {
    id: previewCell
    /// The side's own word — `Before` / `After`. The size stands beside it with a drawn dot between them, so this cell
    /// composes its own line.
    required property string caption
    /// A `file:` URL — the working-tree file itself, or the file core wrote the blob to — stamped with the read it was
    /// made at, so a file that moved under the pane comes back as a source that changed (`DiffModel.previewOldUrl`).
    required property string url
    required property string sizeText
    /// Whether the image is a vector one — the model's word (`DiffModel.previewVector`).
    /// SVG rasters scale smoothly, pixel rasters stay crisp.
    required property bool isVector
    /// Automation: the decode is asynchronous, so a read that has settled is not yet a picture on screen. `settled`
    /// is the decoder's answer either way, `shown` the picture being there.
    readonly property bool settled: previewImage.status === Image.Ready || previewImage.status === Image.Error
    readonly property bool shown: previewImage.status === Image.Ready
    visible: sizeText !== ""
    spacing: Theme.spaceXs

    // Image-preview zoom steps: a small image draws at a fixed integer scale picked from its natural size alone,
    // then fit-to-frame shrinking still wins when space runs out. Small icons land in a readable 128-256px
    // band.
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

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        Label {
            text: previewCell.caption
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
        }
        DotMark { tint: Theme.textSecondary }
        Label {
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
        // Box the image may occupy.
        readonly property real innerW: width - 2 * Theme.spaceXs
        readonly property real innerH: height - 2 * Theme.spaceXs
        // Decoded size (0 until the image is ready).
        readonly property real naturalW: previewImage.implicitWidth
        readonly property real naturalH: previewImage.implicitHeight
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
            source: previewCell.url
            asynchronous: true
            // Decoded once, held by this item alone, and let go with the URL: a closed pane keeps no picture in the
            // pixmap cache.
            cache: false
            // Decoded at natural size: `sourceSize` *rescales* rasters to the given size (a 16px icon came back
            // blurry at screen width). Integer upscales stay crisp (pixel art); shrinking and vector rasters smooth.
            smooth: previewFrame.displayScale < 1 || previewCell.isVector
            mipmap: true
            visible: status === Image.Ready
        }
        // A side with nothing to show: a file core could not write, or a format this runtime has no decoder for.
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
