import QtQuick
import platitude
import platitude.ui

// What this binary is and the git it drives (デザイン規約 §アプリ名), in the right pane's corner: a release names its
// tag; any other build the tag it will become, the tree that built it and the commit. Values in fields
// (規約 §右のペインの字は掴める): neither the git's version nor the build's name is written anywhere else in the window.
// A narrow pane has the line give way, and a pane too narrow for its least has the corner stand down (`BuildLine`).
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
Item {
    id: gitCorner

    /// The height the pane under it leaves bare; the page picks which pane answers.
    required property real roomLeft
    /// The pane's width. The line gives way to it a step clear of either edge (`BuildLine`), and under the line's
    /// least the corner stands down rather than run out past the pane's left edge: a narrow pane keeps its room for
    /// its own words.
    required property real roomAcross
    /// Whether the seat is offered at all (規約 §コミットメッセージの 2 つの枠「ペインの底は常に埋まる」). A property, not
    /// `visible` on the instance — that would replace the binding below, room test and all.
    property bool offered: true
    /// One step, not two: a list's rows already carry their own spacing (user decision — ask before changing).
    readonly property real roomNeeded: gitCorner.implicitHeight + Theme.spaceXs
    /// What has given way in the line (`BuildLine.gives`), for runs.
    readonly property alias gives: line.gives
    /// `PGG-v0.1.0`, or `v0.1.0-b-c7b549aa` — the tree only where the build has one, and `?` for a commit the build
    /// could not read.
    readonly property string buildName: AppBackend.buildRelease
                                        ? qsTr("PGG-%1").arg(AppBackend.buildTag)
                                        : [AppBackend.buildTag, AppBackend.buildTree,
                                           AppBackend.buildCommit || qsTr("?")].filter(part => part !== "").join("-")

    // Hidden once the list reaches the corner, or the pane is narrower than the line's least. Hiding keeps the
    // implicit height, so `roomNeeded` cannot oscillate; the line's least does not move with the room.
    visible: gitCorner.offered && AppBackend.gitVersion !== ""
             && gitCorner.roomLeft >= gitCorner.roomNeeded && line.fits
    implicitWidth: line.implicitWidth
    implicitHeight: line.implicitHeight

    BuildLine {
        id: line
        anchors.fill: parent
        name: gitCorner.buildName
        hash: AppBackend.buildRelease ? "" : AppBackend.buildCommit
        version: AppBackend.gitVersion
        room: gitCorner.roomAcross - Theme.spaceXs * 2
    }
}
