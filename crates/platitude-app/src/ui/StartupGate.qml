import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// ---- the two ways the window has nothing to show ---------------------
// No git to ask, or another process already has the files. A git
// older than the supported minimum is not one of them: it answers, so
// the app runs and wears the band's `OLD GIT` badge instead
// (規約 §ウィンドウの縁).
Item {
    id: gate

    /// The way out of the second one. The band is the title bar and it is
    /// inside the part that stays hidden, so the window is asked to close
    /// itself from here.
    signal closeRequested()

    visible: AppBackend.gitState !== "ok" || AppBackend.alreadyRunning

    // Its own ground rather than the window's, so that a grab of this
    // item is a picture of the screen (`AutoShotDriver`). Nothing under it
    // is drawn while it is up — `mainUi` is hidden — so the colour is
    // the one the window would have shown anyway.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgBase
    }
    Column {
        anchors.centerIn: parent
        spacing: Theme.spaceLg
        width: Math.min(640, gate.width - 2 * Theme.spaceXxl)
        Label {
            text: qsTr("Platitude GG")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            visible: AppBackend.alreadyRunning
            text: qsTr("Platitude GG is already open. Its window is the one to use.")
            wrapMode: Text.Wrap
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
        }
        // Which one, since two builds are alike on screen and the
        // taskbar's launch entry does not say which it started.
        Label {
            visible: AppBackend.alreadyRunning
            text: AppBackend.heldElsewhere
            color: Theme.textSecondary
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
            elide: Text.ElideMiddle
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
        }
        // The way out: the band is the title bar and it is inside the
        // part that stays hidden. A plain frame (規約 §アクセント).
        ActionButton {
            visible: AppBackend.alreadyRunning
            implicitHeight: Theme.controlHeight
            text: qsTr("Close")
            frameColor: Theme.borderDefault
            activeFocusOnTab: true
            anchors.horizontalCenter: parent.horizontalCenter
            onActivated: gate.closeRequested()
        }
        // The drawn ring, not Fusion's BusyIndicator
        // (規約 §進行中・長押しの定数).
        SpinnerIcon {
            spinning: AppBackend.gitState === "checking"
                      && !AppBackend.alreadyRunning
            width: Theme.iconLg
            height: Theme.iconLg
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            visible: AppBackend.gitState === "missing"
            text: qsTr("git was not found on PATH. Install git %1 or newer and restart.")
                      .arg(AppBackend.minimumGit)
            wrapMode: Text.Wrap
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
        }
        Label {
            visible: AppBackend.gitState === "error"
            text: AppBackend.gitError
            color: Theme.danger
            wrapMode: Text.Wrap
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
        }
    }
}
