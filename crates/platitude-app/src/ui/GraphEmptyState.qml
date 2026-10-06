pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// What stands in the middle of the graph when there is no history to draw: the first load's spinner, the unborn
// repository's screen, and the empty window's.
Item {
    id: emptyState

    required property var graphModel
    /// No repository behind the pane at all.
    required property bool blank
    required property var workingTree
    /// A repository is open and has no commits yet — a screen apart from `blank`.
    readonly property bool unborn: !emptyState.blank && emptyState.workingTree.unborn
    /// The line under the heading, `%1` left open so the branch name takes its own colour (`Words.nameInSentence`).
    readonly property string firstCommitLine: qsTr("The first commit will start %1")

    signal openRepositoryRequested()

    // First load, before any row exists (規約 §進行中・長押しの定数).
    SpinnerIcon {
        anchors.centerIn: parent
        width: Theme.iconLg
        height: Theme.iconLg
        spinning: emptyState.graphModel.loading && emptyState.graphModel.rowTotal === 0
    }
    // A walk that gave up is not said here but by the band's `STALE GRAPH` badge (`BandStateGroup`): a full-pane
    // sentence would paint over the rows already drawn.
    //
    // A repository with no history yet. No button: what moves it on is files in the folder, which the app cannot put
    // there.
    Column {
        anchors.centerIn: parent
        visible: emptyState.unborn
        spacing: Theme.spaceLg
        Label {
            // 規約 §タイポグラフィ「画面を占める状態メッセージの見出し」 — not the app name's step.
            text: qsTr("No commits yet")
            font.pixelSize: Theme.fontLg
            font.weight: Theme.fontWeightStrong
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeLine {
            // Names the branch: with no HEAD row, no pin can.
            text: emptyState.firstCommitLine.arg(emptyState.workingTree.branch)
            markup: Words.nameInSentence(emptyState.firstCommitLine, emptyState.workingTree.branch, Theme.textLink)
            color: Theme.textSecondary
            // Natural width while it fits, wrapped at the window's floor — unbounded it takes width from the panes
            // beside it (規約 §窓の床).
            width: Math.min(implicitWidth, emptyState.width - 2 * Theme.spaceXl)
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }
    // Empty window.
    Column {
        anchors.centerIn: parent
        visible: emptyState.blank
        spacing: Theme.spaceLg
        Label {
            text: Words.appName
            font.pixelSize: Theme.fontXl
            font.weight: Theme.fontWeightStrong
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeLine {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            // Bounded as the unborn line above is.
            width: Math.min(implicitWidth, emptyState.width - 2 * Theme.spaceXl)
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeButton {
            text: Words.openRepository
            onActivated: emptyState.openRepositoryRequested()
        }
    }
}
