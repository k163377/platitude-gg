pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// What stands in the middle of the graph when there is no history to draw: the first load's turning mark, whatever went
// wrong instead, and the empty window's one thing worth doing.
Item {
    id: emptyState

    required property var graphModel
    /// No repository behind the pane at all.
    required property bool blank
    /// The work tree behind the pane — read for the one state this screen has of its own.
    required property var workTree
    /// A repository is open and has no commits yet. Not `blank`'s other half: a window with nothing in it and a
    /// repository with nothing in it are two different screens.
    readonly property bool unborn: !emptyState.blank && emptyState.workTree.loaded
                                   && emptyState.workTree.headOid === ""

    signal openRepositoryRequested()

    // First load, before any row exists. The drawn ring, not Fusion's BusyIndicator — the window has one turning mark
    // and this is it (規約 §進行中・長押しの定数).
    SpinnerIcon {
        anchors.centerIn: parent
        width: Theme.iconLg
        height: Theme.iconLg
        spinning: emptyState.graphModel.loading && emptyState.graphModel.rowTotal === 0
    }
    // Why the column is empty, in the words of whoever could not fill it — git's, where git is what failed. **A walk
    // that ended without an answer has none**, and that is what the second line is for: nobody said anything, so the
    // screen says it (デザイン規約 §答えの要らない報せ「断ったのがこちら自身なら文もこちら」の同じ配り方).
    NoticeLine {
        anchors.centerIn: parent
        visible: emptyState.graphModel.failed
        text: emptyState.graphModel.error !== "" ? emptyState.graphModel.error : Words.graphStopped()
        color: Theme.danger
        width: parent.width - 2 * Theme.spaceXl
    }
    // A repository with no history yet. Told in the middle of the column that will hold it, not under the working-tree
    // row: the history has not stopped somewhere (the window cut's footer says that, where it stops) — there is none,
    // so the whole column is what the words are about. Nothing to do here that the app can do, so no button under
    // them: the one thing that moves this on is putting files in the folder, and the row above already appears when
    // they are there.
    Column {
        anchors.centerIn: parent
        visible: emptyState.unborn
        spacing: Theme.spaceLg
        Label {
            // 規約 §タイポグラフィ「画面を占める状態メッセージの見出し」. Not the app name's step — that one is the app's own
            // (空の窓 / git ゲート), and this screen is a repository's state rather than a window with nothing in it.
            text: qsTr("No commits yet")
            font.pixelSize: Theme.fontLg
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeLine {
            // The branch nothing else in this column can name: there is no HEAD row for the pin to ride, and the
            // sidebar's worktree row is the only other place it is written.
            text: qsTr("The first commit will start %1").arg(emptyState.workTree.branch)
            color: Theme.textSecondary
            // Bounded the way the line above it is (規約 §窓の床).
            width: Math.min(implicitWidth, emptyState.width - 2 * Theme.spaceXl)
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }
    // Empty window: the one thing worth doing sits in the column that will hold the history.
    Column {
        anchors.centerIn: parent
        visible: emptyState.blank
        spacing: Theme.spaceLg
        Label {
            text: Words.appName
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeLine {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            // Its natural width while the column has room for it, and wrapped inside the column when it has not: at the
            // window's floor this pane is narrower than the line asks for, and unbounded it took the difference from
            // the panes on either side (規約 §窓の床). The same shape the error line above already has.
            width: Math.min(implicitWidth, emptyState.width - 2 * Theme.spaceXl)
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeButton {
            text: Words.openRepository
            onActivated: emptyState.openRepositoryRequested()
        }
    }
}
