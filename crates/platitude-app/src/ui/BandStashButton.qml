import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Everything uncommitted, set aside in one entry, on the press (デザイン規約 §変更を退避する).
ActionButton {
    id: stashButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// What the working tree lets this button do (`platitude_core::stash::standing`), or `closed` with nothing open.
    readonly property string mode:
        stashButton.curPage === null || stashButton.curPage.pageTab.state !== "open"
            || !stashButton.curPage.pageWt.loaded
        ? "closed" : stashButton.curPage.pageWt.stashStanding

    kind: "stash"
    text: "stash"
    code: true
    // The resting frame, like the two beside it (`BandFetchButton`); a stash destroys nothing, so it never turns.
    frameColor: Theme.borderStrong
    // The canvas inside the frame, like the two beside it (`BandFetchButton`).
    faceColor: Theme.bgSurface
    // No `!`, hold or ring (デザイン規約 §変更を退避する). Down while a rebase plan is composed (its run button is the one
    // write), and while the pane reads another working copy — a press would set aside *this* window's changes
    // (P3-確認事項 §別 worktree の未コミット行).
    enabled: stashButton.mode === "ready" && stashButton.curPage.pageTab.busyCount === 0
             && !stashButton.curPage.planShown && stashButton.curPage.wipWritable
    tip: {
        // Silent where the band already says why (デザイン規約 §変更を退避する), emptied by hand: a disabled control
        // still opens its ToolTip (rules-refs/app-ui.md「押せないボタンを黙らせる」).
        if (stashButton.mode === "closed" || stashButton.curPage.pageTab.busyCount > 0)
            return ""
        // The freeze names itself: a tree that could be stashed looks no different while a plan stands over it.
        if (stashButton.curPage.planShown)
            return qsTr("A rebase plan is being composed — until it closes, the only write is its run button")
        // The files on screen are the ones a reader would expect a press here to reach.
        if (!stashButton.curPage.wipWritable)
            return qsTr("Reading another working copy — this sets aside this window's changes")
        // These speak though disabled: the reason is not on screen (デザイン規約 §変更を退避する).
        if (stashButton.mode === "unborn")
            return qsTr("No commits yet — git cannot stash before the first one")
        if (stashButton.mode === "conflicts")
            return qsTr("Settle the conflicts first — git will not stash an unmerged file")
        if (stashButton.mode === "clean")
            return qsTr("Nothing to stash — the working tree is clean")
        // The breadth is said only here (デザイン規約 §変更を退避する).
        return qsTr("Set these changes aside, files git is not tracking yet included")
    }
    // Named from the page's summary where one is written (デザイン規約 §変更を退避する).
    onActivated: stashButton.curPage.pageTab.pushStash(stashButton.curPage.pageStashName)
}
