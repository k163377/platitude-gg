import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One side (Before / After) of the diff pane's image preview; a side with no `sizeText` collapses.
ColumnLayout {
    id: previewCell
    /// `Before` / `After`.
    required property string caption
    /// A `file:` URL (the working-tree file, or core's copy of the blob) stamped with its read, so a file that moved
    /// under the pane comes back as a changed source (`DiffModel.previewOldUrl`).
    required property string url
    required property string sizeText
    /// `DiffModel.previewVector`: SVG scales smoothly, pixel rasters stay crisp.
    required property bool isVector
    /// Automation, since the decode is asynchronous: `settled` = the decoder has answered either way, `shown` = the
    /// picture is there.
    readonly property bool settled: previewImage.status === Image.Ready || previewImage.status === Image.Error
    readonly property bool shown: previewImage.status === Image.Ready
    visible: sizeText !== ""
    spacing: Theme.spaceXs

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
            // A closed pane keeps no picture in the pixmap cache.
            cache: false
            // No `sourceSize`: it rescales rasters at decode, blurring small icons.
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
