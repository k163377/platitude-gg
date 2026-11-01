import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// What this binary is and the git it drives (デザイン規約 §アプリ名), in the right pane's corner: a release names its
// tag; any other build the tag it will become, the tree that built it and the commit. One field
// (規約 §右のペインの字は掴める): neither the git's version nor the build's name is written anywhere else in the window.
//
// The line (user decision — ask before changing): the build's name joined by `-`, the git version after a `/`.
// A release (HEAD is the version's tag itself) `PGG-v0.1.0/git 2.51.0`; any other build `v0.1.0-b-c7b549aa/git 2.51.0`
// — the tag to be cut next, the tree's letter (a worktree's build only), the hash's first 8; a build that could not
// read its commit (a source archive) `v0.1.0-?/git 2.51.0` (`v0.1.0-b-?/…` in a worktree), never a release. `PGG` is
// this corner's short name and nowhere else's (user decision): every other place says `Platitude GG`. Release and
// commit come from the system git at build time (`build.rs`; a Linux container is handed them by xtask).
//
// It keeps its colour when the git is old: the band's `OLD GIT` badge is the one place that says so, and a corner
// saying it too could no longer mean "the version" as the badge means "the warning" (規約 §git が無い時・古い時).
// Never on the commit pane: its foot is the commit button's, so no seat is left there — the version is read on the
// details side (`WipPane.bottomRoom`). Headless: `corner`.
RowLayout {
    id: gitCorner

    /// The height the pane under it leaves bare; the page picks which pane answers.
    required property real roomLeft
    /// Whether the seat is offered at all (規約 §コミットメッセージの 2 つの枠「ペインの底は常に埋まる」). A property, not
    /// `visible` on the instance — that would replace the binding below, room test and all.
    property bool offered: true
    /// One step, not two: a list's rows already carry their own spacing (user decision — ask before changing).
    readonly property real roomNeeded: gitCorner.implicitHeight + Theme.spaceXs
    /// The pane's width that shows the whole line, a step clear of either edge: the pane's floor is held to it
    /// (`PageLayout.rightMinWidth`), so no window shape cuts it. Hiding keeps the implicit width, as the height.
    readonly property real floorWidth: gitCorner.implicitWidth + Theme.spaceXs * 2
    /// `PGG-v0.1.0`, or `v0.1.0-b-c7b549aa` — the tree only where the build has one, and `?` for a commit the build
    /// could not read.
    readonly property string buildName: AppBackend.buildRelease
                                        ? qsTr("PGG-%1").arg(AppBackend.buildTag)
                                        : [AppBackend.buildTag, AppBackend.buildTree,
                                           AppBackend.buildCommit || qsTr("?")].filter(part => part !== "").join("-")

    // Hidden once the list reaches the corner. Hiding keeps the implicit height, so `roomNeeded` cannot oscillate.
    visible: gitCorner.offered && AppBackend.gitVersion !== ""
             && gitCorner.roomLeft >= gitCorner.roomNeeded

    LineText {
        text: qsTr("%1/git %2").arg(gitCorner.buildName).arg(AppBackend.gitVersion)
        color: Theme.textMuted
        pixelSize: Theme.fontSm
    }
}
