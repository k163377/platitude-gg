import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import "painted.js" as Painted

// A card of headings alone begins their words on one x (デザイン規約 §操作パネル): the heading without the WORKTREES
// mark holds the mark's step — the shape `TopBar`'s stand card has. Every other menu keeps each heading's own step.
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

        /// A heading's mark seat holds the ink of every mark a card's heading wears but GitHub's cloud, which runs past
        /// it, and is the widest of them, the tag, to a whole pixel (`AppMenuItem.headSeat`): the words start a step
        /// past any of them. Read off the heading's own mark — the card is not open, so nothing on it is visible.
        function test_a_headings_seat_holds_every_heading_mark_but_the_cloud() {
            const marked = loose.itemAt(1)
            const mark = Painted.itemUnder(marked, child => child.inkAcrossOf !== undefined && child.kind === "tree")
            verify(mark !== null, "the heading's own mark")
            for (const kind of ["tree", "branch", "tag", "github"])
                verify(mark.inkAcrossOf(kind) <= marked.headSeat,
                       kind + "'s ink " + mark.inkAcrossOf(kind) + " in the seat " + marked.headSeat)
            verify(marked.headSeat - mark.inkAcrossOf("tag") < 1, "a whole pixel at most past the tag's ink")
            verify(mark.inkAcrossOf("remote") > marked.headSeat, "the cloud runs past the seat")
        }
    }
}
