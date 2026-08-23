import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One side of the diff pane's image preview (Before / After). Absent sides collapse (visible tracks sizeText), so an
// added image shows a single full-width After cell and a deleted one a single Before.
//
// Scaling: 1:1 when the natural size fits, a fixed integer zoom for small images (zoomFor — independent of the window),
// and fit-to-frame shrinking as the final cap, so a tiny window never overflows and never picks a different zoom, it
// only shrinks.
ColumnLayout {
    id: previewCell
    /// The side's own word — `Before` / `After`. The size stands beside it with a drawn dot between them, so this cell
    /// composes its own line rather than being handed one already spelled.
    required property string caption
    required property string url
    required property string sizeText
    /// Whether the image is a vector one — the model's word (`DiffModel.previewVector`), not something read back off
    /// the URL. SVG rasters scale smoothly, pixel rasters must not.
    required property bool isVector
    visible: sizeText !== ""
    spacing: Theme.spaceXs

    // Image-preview zoom steps: a small image draws at a fixed integer scale picked from its natural size (never from
    // the window), then fit-to-frame shrinking still wins when space runs out. Small icons land in a readable 128-256px
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
        // Shrink freely; enlarge only in whole steps, never past the frame and never more than the size-picked zoom.
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
            cache: false
            // No sourceSize: it does not cap decoding, it *rescales* rasters to the given size (a 16px icon came back
            // blurry at screen width). Decode memory is already bounded by the 16 MiB byte cap in
            // platitude-core::preview. Integer upscales stay crisp (pixel art); shrinking and vector rasters smooth.
            smooth: previewFrame.displayScale < 1 || previewCell.isVector
            mipmap: true
            visible: status === Image.Ready
        }
        Label {
            anchors.centerIn: parent
            width: Math.min(implicitWidth, parent.width - 2 * Theme.spaceSm)
            visible: previewCell.url === "" || previewImage.status === Image.Error
            text: previewCell.url === "" ? qsTr("Too large to preview") : qsTr("Preview unavailable")
            elide: Text.ElideRight
            color: Theme.textMuted
        }
    }
}
