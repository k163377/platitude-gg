import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui
import platitude.ui.words

// Everything uncommitted, set aside in one entry, on the press (`git stash push -u`; 規約 §変更を退避する for the words).
//
// Always on the toolbar, right of `push` (user decision — ask before changing): what it sets aside is the uncommitted
// work, which is there whichever pane is open — on the WIP pane's band, selecting one commit of history would take the
// button off the screen. Labelled with the code chip `stash`: the toolbar is where a label is the command itself
// (規約 §git 用語のコード表記), and one plain word beside `fetch` / `push` would break the reading that a chip says "this
// is git's spelling". One action: nothing is asked, since anything that could be asked fits later and elsewhere — a
// name is given on the STASHES row (規約 §左メニューの所作 の改名), half the tree is set aside by a file row's `stash`.
// A name already written is used, though (`WipPane.stashName`). A plain button: a stash discards nothing, so it has no
// warning colour, and the write is local, so no ring (規約 §進行中・回数押しの定数); while any other git command runs it
// goes down like the rest, and stays down until what that command left has been read again (`RepoTab.settling`; user
// decision — ask before changing): git answers ahead of that reading, and a press in between would set aside a tree
// the screen is not showing yet — its own press included, so it takes one press at a time. Boxed with fetch and push
// though its word never changes (`TopBar.widestAction`).
ActionButton {
    id: stashButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// What the worktree lets this button do (`platitude_core::stash::standing`), or `closed` with nothing open: one of
    /// `closed` / `unborn` / `conflicts` / `clean` / `ready`, the button's one answer (`TopBar.stashMode`). `loaded`
    /// first: the zero counts before status lands are no answer. Headless: `stash-state <mode>` for `ready` / `clean` /
    /// `conflicts` / `unborn`.
    readonly property string mode:
        stashButton.curPage === null || stashButton.curPage.pageTab.state !== "open"
            || !stashButton.curPage.pageWorktree.loaded
        ? "closed" : stashButton.curPage.pageWorktree.stashStanding

    kind: "stash"
    text: "stash"
    code: true
    // The resting frame, like the two beside it (`BandFetchButton`); a stash destroys nothing, so it never turns.
    frameColor: Theme.borderStrong
    // The canvas inside the frame, like the two beside it (`BandFetchButton`).
    faceColor: Theme.bgSurface
    // No `!`, hold or ring (header). Down while a rebase plan is composed (its run button is the one
    // write), and while the pane reads another worktree — a press would set aside *this* window's changes
    // (`RepoPage.wipWritable`). Down too while a stage or unstage press holds it back, as the commit button is
    // (`Words.stagingWait`): what it sets aside is the index git holds, not yet the one on screen.
    enabled: stashButton.mode === "ready" && stashButton.curPage.pageTab.busyCount === 0
             && !stashButton.curPage.pageTab.settling
             && !stashButton.curPage.pageTab.stagingUnseen
             && !stashButton.curPage.planShown && stashButton.curPage.wipWritable
    tip: {
        // Silent where the band already says why, emptied by hand: a disabled control still opens its ToolTip
        // (rules-refs/app-ui.md「押せないボタンを黙らせる」). No tab: nothing to set aside.
        if (stashButton.mode === "closed")
            return ""
        // Not said anywhere else: the rows look moved already (`Words.stagingWait`, git running or not).
        if (stashButton.curPage.pageTab.stagingUnseen)
            return Words.stagingWait
        // Another git command out — running, where the band says it, or answered and still being read back, the
        // same wait's tail.
        if (stashButton.curPage.pageTab.busyCount > 0 || stashButton.curPage.pageTab.settling)
            return ""
        // The freeze names itself: a tree that could be stashed looks no different while a plan stands over it.
        if (stashButton.curPage.planShown)
            return qsTr("A rebase plan is being composed — until it closes, the only write is its run button")
        // The files on screen are the ones a reader would expect a press here to reach.
        if (!stashButton.curPage.wipWritable)
            return qsTr("Reading another worktree — this sets aside this window's changes")
        // git's refusals, said before the press in the menus' reasons — a file row's `stash` says the same
        // (`WorktreeModel.stashWhy`). These speak though disabled: a disabled button keeps quiet only where its reason
        // shows elsewhere (規約 §無効), and here it does not — a tree in conflict looks exactly like one that could be
        // stashed, and the band's conflict badge says only that the repository is in conflict.
        if (stashButton.curPage.pageWorktree.stashWhy !== "")
            return Words.menuWhy(stashButton.curPage.pageWorktree.stashWhy)
        if (stashButton.mode === "clean")
            return qsTr("Nothing to stash — the working tree is clean")
        // The breadth (untracked files included, `RepoTab::stash_push`) is said only here: the label stays the bare
        // command (規約 §hover のツールチップ).
        return qsTr("Set these changes aside, files git is not tracking yet included")
    }
    // Named from the page's summary where one is written (`WipPane.stashName`).
    onActivated: stashButton.curPage.pageTab.pushStash(stashButton.curPage.pageStashName)
}
