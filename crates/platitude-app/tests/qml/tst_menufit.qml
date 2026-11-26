import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import platitude.ui.words

// A menu as wide as its widest row cuts nothing in that row (デザイン規約 §メニュー「幅は最も広い行に合わせる」), whatever
// the row draws beside its words: the row's `RowLayout` takes every item of a fixed width out of the row rounded up to
// a whole pixel, so a row that bids those widths as measured comes up the fractions short, and the words or the name
// that take the rest are cut (`AppMenuItem.besideWords`). So the row bids at least what its layout spends — read off
// the layout's own width, which a single fraction absorbed by the card's rounding still shows. Each card holds the one
// row, so the row decides the width, and drops the floor that would hand it slack. The names stay under the ceiling a
// name bids (`labelColW`): past it a row bids less than its layout on purpose.
//
// A control, to hand the cards the window's font as `Main` does: what the row draws without a face of its own (the
// note) is priced in it. Only a face whose advances come out fractional shows it — Noto Sans CJK JP's, Ubuntu's.
Control {
    id: root
    width: 1440
    height: 900
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // The graph row's `rebase --interactive` on a pushed commit.
    AppMenu {
        id: noteMenu
        widthFloor: 0
        AppMenuItem {
            code: "rebase --interactive"
            text: "from here up"
            note: Words.rewritesPushed
        }
    }
    // The left menu's `rebase` on a branch whose commits are pushed: a name in the sentence.
    AppMenu {
        id: refNoteMenu
        widthFloor: 0
        AppMenuItem {
            code: "rebase"
            refSentence: "%1 onto it"
            refName: "feature/topic-a"
            refMark: "branch"
            note: Words.rewritesPushed
        }
    }
    // A branch another worktree holds: the words, then the folder as code behind its mark.
    AppMenu {
        id: codeNameMenu
        widthFloor: 0
        AppMenuItem {
            text: "Open"
            nameMark: "tree"
            codeName: "review"
        }
    }
    // The operation panel's rows: a branch, its ahead / behind and its badge.
    AppMenu {
        id: trackMenu
        widthFloor: 0
        AppMenuItem {
            text: "feature/topic-a"
            counts: ({ "ahead": 2, "behind": 13, "level": false })
            remoteBadge: true
        }
    }
    // A worktree's row: the branch it has out in the far column.
    AppMenu {
        id: sideMenu
        widthFloor: 0
        AppMenuItem {
            text: "platitude-gg-review"
            sideName: "feature/topic-a"
        }
    }
    // A tab's row in a linked worktree: the tab's own run after the words.
    AppMenu {
        id: trailMenu
        widthFloor: 0
        AppMenuItem {
            text: "platitude-gg"
            trail: "platitude-gg-review"
        }
    }

    /// The first item under `item` the test holds true of, or null.
    function find(item, holds) {
        for (const child of item.children) {
            if (holds(child))
                return child
            const found = root.find(child, holds)
            if (found !== null)
                return found
        }
        return null
    }

    TestCase {
        name: "MenuFit"
        when: windowShown

        function test_the_widest_row_cuts_nothing_data() {
            return [
                { tag: "a chip, words and a note", menu: noteMenu },
                { tag: "a chip, a name in the sentence and a note", menu: refNoteMenu },
                { tag: "words and a code name", menu: codeNameMenu },
                { tag: "words, a track and a badge", menu: trackMenu },
                { tag: "words and the far column's branch", menu: sideMenu },
                { tag: "words and a worktree after them", menu: trailMenu }
            ]
        }
        function test_the_widest_row_cuts_nothing(data) {
            const menu = data.menu
            verify(menu.offer(), "the card has a row to offer")
            tryCompare(menu, "opened", true)
            const row = menu.itemAt(0)
            compare(menu.width, menu.widestRow + menu.leftPadding + menu.rightPadding, "the row sets the card's width")
            verify(waitForItemPolished(row.contentItem), "the row is laid out")
            const words = root.find(row, child => child.cutWords !== undefined)
            const spent = row.leftPadding + row.contentItem.implicitWidth + row.rightPadding
            const says = "row " + row.width + " bids " + row.implicitWidth + ", its layout spends " + spent
                         + ", words " + words.implicitWidth + " laid out in " + words.width
            verify(row.implicitWidth >= spent, "the row bids what its layout spends: " + says)
            verify(!words.cutWords, "the words are whole: " + says)
            verify(!row.refCut, "the name in the sentence is whole: " + says)
            verify(!row.nameCut, "the code name is whole: " + says)
            verify(!row.sideCut, "the far column's branch is whole: " + says)
            verify(!row.trailCut, "the worktree after the words is whole: " + says)
            menu.dismiss()
            tryCompare(menu, "visible", false)
        }
    }
}
