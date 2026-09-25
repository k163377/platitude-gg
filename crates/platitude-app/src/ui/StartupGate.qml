import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// The window with nothing to show: no git to ask, or another process already has the files. An old git still answers,
// so the app runs and wears the band's `OLD GIT` badge instead (規約 §ウィンドウの縁).
Item {
    id: gate

    /// The hand under this screen's words (`SweepPad`), named so a run can enter it as `gate.pad`.
    property alias pad: gateHand

    /// The way out when another process has the files: the band holding the title bar is hidden, so the window is
    /// asked to close from here.
    signal closeRequested()

    visible: AppBackend.gitState !== "ok" || AppBackend.alreadyRunning

    // Its own ground, so a grab of this item is a picture of the screen (`AutoShotDriver`); `mainUi` is hidden while
    // this is up, so it is the colour the window shows anyway.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgBase
    }
    // Under the column, so a press reaches it only where nothing else took one and `Close` keeps its presses
    // (規約 §右のペインの字は掴める — this screen has no command log to take git's words from).
    SweepPad {
        id: gateHand
        anchors.fill: gateColumn
        content: gateColumn
    }
    Column {
        id: gateColumn
        anchors.centerIn: parent
        spacing: Theme.spaceLg
        width: Math.min(Theme.gateWidth, gate.width - 2 * Theme.spaceXxl)
        CardText {
            text: Words.appName
            pixelSize: Theme.fontXl
            weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
            width: parent.width
        }
        NoticeLine {
            visible: AppBackend.alreadyRunning
            text: qsTr("Platitude GG is already open. Its window is the one to use.")
            width: parent.width
        }
        // Which executable has the files, since two builds look alike and the taskbar does not say which it started.
        // Wrapped, not cut (規約 §右のペインの字は掴める).
        CardText {
            visible: AppBackend.alreadyRunning
            text: AppBackend.heldElsewhere
            color: Theme.textSecondary
            mono: true
            pixelSize: Theme.fontSm
            horizontalAlignment: Text.AlignHCenter
            width: parent.width
        }
        NoticeButton {
            visible: AppBackend.alreadyRunning
            text: qsTr("Close")
            onActivated: gate.closeRequested()
        }
        SpinnerIcon {
            spinning: AppBackend.gitState === "checking" && !AppBackend.alreadyRunning
            width: Theme.iconLg
            height: Theme.iconLg
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeLine {
            visible: AppBackend.gitState === "missing"
            text: qsTr("git was not found on PATH. Install git %1 or newer and restart.").arg(AppBackend.minimumGit)
            width: parent.width
        }
        NoticeLine {
            visible: AppBackend.gitState === "error"
            text: AppBackend.gitError
            color: Theme.danger
            width: parent.width
        }
    }
}
