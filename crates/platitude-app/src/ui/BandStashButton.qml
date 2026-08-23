import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Everything uncommitted, set aside in one entry, on the press (デザイン規約 §変更を退避する). It stands on the band
// rather than over the file list: what it sets aside is the working tree, which is there whichever pane is
// open, and the pane that could otherwise host it is one row's selection away most of the time.
ActionButton {
    id: stashButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// What the working tree lets this button do (デザイン規約 §変更を退避する). The refusals are read where they
    /// were measured (`platitude_core::stash::standing` — unborn / conflicts / clean / ready); `closed` is
    /// this band's own half: nothing open here to read a working tree off.
    readonly property string mode:
        stashButton.curPage === null || stashButton.curPage.pageTab.state !== "open"
            || !stashButton.curPage.pageWt.loaded
        ? "closed" : stashButton.curPage.pageWt.stashStanding

    kind: "stash"
    // The label is the command, like both buttons beside it (デザイン規約 §git 用語のコード表記). One wording in every
    // state: nothing here changes what the press costs, so there is no second shape to say.
    text: "stash"
    code: true
    // Nothing worn on top of that: a stash destroys nothing, so no frame, no `!` and no hold (デザイン規約
    // §変更を退避する). No ring either — the ring names a wait on the network and this write is local (§進行中・
    // 長押しの定数); while it runs the band is busy and the button is down, like the commit button beside its
    // own write.
    enabled: stashButton.mode === "ready" && stashButton.curPage.pageTab.busyCount === 0
    tip: {
        // Nothing open, or another git command already out. Neither is about stashing, and both are said on
        // this band already (デザイン規約 §無効). Said out loud because a disabled control still takes hover and
        // still opens its attached ToolTip (実測: rules-refs/app-ui.md §hover).
        if (stashButton.mode === "closed" || stashButton.curPage.pageTab.busyCount > 0)
            return ""
        // The three that *are* about the working tree each name what is missing (デザイン規約 §hover のツールチップ).
        // They speak where the fetch button's refusals stay silent, because the reason is not on screen the way
        // `REMOTES 0` is: a tree with conflicts in it looks exactly like one that could be stashed, and the
        // band's own conflict badge says the repository has them — not that they are what is holding this
        // button down.
        if (stashButton.mode === "unborn")
            return qsTr("No commits yet — git cannot stash before the first one")
        if (stashButton.mode === "conflicts")
            return qsTr("Settle the conflicts first — git will not stash an unmerged file")
        if (stashButton.mode === "clean")
            return qsTr("Nothing to stash — the working tree is clean")
        // The breadth is what the label has no room for, and with no card to read it is the only place it is
        // said (デザイン規約 §hover のツールチップ).
        return qsTr("Set these changes aside, files git is not tracking yet included")
    }
    // The name comes from the page rather than being asked for here: a summary already written for these
    // changes is the name the reader would have given them anyway (デザイン規約 §変更を退避する).
    onActivated: stashButton.curPage.pageTab.pushStash(stashButton.curPage.pageStashName)
}
