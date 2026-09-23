import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A card of headings alone begins their words on one x (デザイン規約 §操作パネル): one heading wears the WORKTREES
// mark and the other wears none, and the one without holds the mark's step — the shape `TopBar`'s stand card has.
// A card that does not ask for it keeps each heading at its own step, which is every other menu in the app.
Item {
    id: root
    width: 400
    height: 300

    AppMenu {
        id: aligned
        alignsHeadings: true
        AppMenu {
            title: "REPOSITORY"
            AppMenuItem {
                text: "one"
            }
        }
        AppMenu {
            title: "WORKTREE"
            titleKind: "tree"
            AppMenuItem {
                text: "two"
            }
        }
    }
    AppMenu {
        id: loose
        AppMenu {
            title: "REPOSITORY"
            AppMenuItem {
                text: "one"
            }
        }
        AppMenu {
            title: "WORKTREE"
            titleKind: "tree"
            AppMenuItem {
                text: "two"
            }
        }
    }

    TestCase {
        name: "MenuHeadings"
        when: windowShown

        function test_the_heading_without_a_mark_holds_the_marks_step() {
            const plain = aligned.itemAt(0)
            const marked = aligned.itemAt(1)
            verify(marked.titled, "WORKTREE wears the section's mark")
            verify(!plain.titled, "and REPOSITORY wears none")
            verify(aligned.headingSeat > 0, "the card measured a step to hold")
            compare(plain.leftPadding, marked.leftPadding)
        }

        function test_a_card_that_does_not_ask_keeps_each_headings_own_step() {
            compare(loose.headingSeat, 0)
            verify(loose.itemAt(0).leftPadding < loose.itemAt(1).leftPadding,
                   "the unmarked heading starts at the row's padding, the marked one past its mark")
        }
    }
}
