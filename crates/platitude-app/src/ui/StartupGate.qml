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

    /// The hand this screen's words are dragged over from the air around
    /// them, named so a run can enter it (`gate.pad`, the way a card is
    /// reached at `<card>.background.pad`).
    property alias pad: gateHand

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
    // The hand the words on this screen are dragged over from the air
    // around them (規約 §右のペインの字は掴める). It lies under the column
    // rather than over it, so a press reaches it only where nothing else
    // took one — the step between two lines, the room beside a short one —
    // and the `Close` button keeps every press it had.
    //
    // **This screen needs it more than any other one does**: it stands
    // before a repository is open, so the command log that holds git's
    // words everywhere else does not exist yet, and what is written here
    // is the whole of what there is to take away.
    SweepPad {
        id: gateHand
        anchors.fill: gateColumn
        content: gateColumn
    }
    Column {
        id: gateColumn
        anchors.centerIn: parent
        spacing: Theme.spaceLg
        width: Math.min(640, gate.width - 2 * Theme.spaceXxl)
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
        // Which one, since two builds are alike on screen and the
        // taskbar's launch entry does not say which it started.
        //
        // Wrapped rather than cut in the middle, the way the sentences
        // above it are: this is a path a reader has to be able to read and
        // to take away, a field has no `elide` to cut it with, and there is
        // nothing beside it on this screen to carry what a cut would drop
        // (規約 §右のペインの字は掴める).
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
        // The drawn ring, not Fusion's BusyIndicator
        // (規約 §進行中・長押しの定数).
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
