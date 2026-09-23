import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The operation panel's name as a box of two lines: the name with its counts on the first, the upstream on the second.
// **The box is as wide as the longer line**, never the two laid end to end — which held a whole upstream's width of air
// past both of them — and the counts end the longer line: carried out to the end of an upstream that runs further,
// or a word's step after a name that does. What the row asks the picker to give comes off the longer line first. The
// arithmetic is the picker's own, so it is asked here with no window and no fonts in question beyond the platform's.
Item {
    id: root
    width: 1200
    height: 200

    component Branch: OpsPicker {
        kind: "branch"
        kindSize: Theme.iconMd
        pixelSize: Theme.fontLg
        height: Theme.opsBarHeight
        width: drawnWidth
    }

    // A short name over a longer upstream — the shape of `main` over `origin/main`.
    Branch {
        id: shortName
        name: "main"
        note: "origin/main-that-runs-further"
        ahead: 2
    }
    // A long name over a shorter upstream — one that still runs well past the name cut to its floor, in any face.
    Branch {
        id: longName
        y: 60
        name: "release/2026-08-candidate-with-a-very-long-branch-name"
        note: "origin/release/2026-08-candidate"
        ahead: 1
        behind: 1
    }
    // No upstream at all: one line, no counts.
    Branch {
        id: bare
        y: 120
        name: "topic"
    }

    // **A name is the way into its card and the way out of it** — the shape `TopBar` gives both of its names: the card
    // hung off the name, and closed on a press outside that name only. With the default policy the press on the name
    // shuts the card and the click that follows the same press opens it again, so the second press never closes it.
    component Door: OpsPicker {
        id: door
        property alias card: doorCard
        height: Theme.opsBarHeight
        width: drawnWidth
        opened: doorCard.opened
        onClicked: {
            if (doorCard.opened) {
                doorCard.close()
                return
            }
            doorCard.offerHere()
        }
        AppMenu {
            id: doorCard
            parent: door
            y: door.height
            AppMenuItem {
                text: "row"
            }
        }
    }
    Door {
        id: hung
        x: 700
        name: "hung"
        card.closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
    }
    Door {
        id: plain
        x: 900
        name: "plain"
    }

    TestCase {
        name: "OpsPicker"
        when: windowShown

        /// Every input the cases move put back where it started, the cards included: a case that fails part-way would
        /// otherwise hand the next one the state it stopped in.
        function init() {
            shortName.given = 0
            longName.given = 0
            shortName.endAir = Qt.binding(() => shortName.gap)
            shortName.opened = false
            hung.card.close()
            plain.card.close()
            tryCompare(hung.card, "visible", false)
            tryCompare(plain.card, "visible", false)
        }

        function test_the_box_is_as_wide_as_the_longer_line() {
            verify(shortName.lineTwoWhole > shortName.lineOneWhole, "the upstream is the longer line")
            compare(shortName.wholeWidth, shortName.bareBox + shortName.lineTwoWhole)
            verify(longName.lineOneWhole > longName.lineTwoWhole, "the name and its counts are the longer line")
            compare(longName.wholeWidth, longName.bareBox + longName.lineOneWhole)
            compare(bare.wholeWidth, bare.bareBox + bare.lineOneWhole)
            compare(bare.lineTwoWhole, 0)
        }

        function test_the_counts_end_an_upstream_that_runs_further() {
            tryCompare(shortName, "trackPlace", "end")
        }

        function test_the_counts_follow_a_longer_name_at_their_own_step() {
            tryCompare(longName, "trackPlace", "after")
            compare(longName.trackGap, Theme.spaceSm)
            compare(bare.trackPlace, "none")
        }

        /// A line that is not the longer one frees nothing by being cut, so what is asked for comes off the longer
        /// line alone until the two meet — and off both from there.
        function test_what_is_given_comes_off_the_longer_line_first() {
            const apart = longName.lineOneWhole - longName.lineTwoWhole
            longName.given = apart - 1
            compare(longName.nameGiven, apart - 1)
            compare(longName.noteGiven, 0)
            compare(longName.drawnWidth, longName.wholeWidth - (apart - 1))
            longName.given = apart + 4
            compare(longName.nameGiven, apart + 4)
            compare(longName.noteGiven, 4)
            // The counts are never what gives: they are still a word's step after what is left of the name.
            tryCompare(longName, "trackPlace", "after")
        }

        /// Nothing is ever cut past its floor, whatever the row asks — and a line is cut no further than the other
        /// line's floor needs: past that, what it gave up would free nothing.
        function test_nothing_gives_past_its_floor() {
            verify(longName.lineOneFloor > longName.lineTwoFloor, "the name's line has the wider floor")
            verify(longName.lineTwoWhole > longName.lineOneFloor, "the upstream runs past the name's floor")
            longName.given = 100000
            compare(longName.drawnWidth, longName.foldWidth)
            compare(longName.nameGiven, longName.nameSlack)
            compare(longName.noteGiven, longName.lineTwoWhole - longName.lineOneFloor)
            verify(longName.noteGiven < longName.noteSlack)
        }

        /// The last name in the row keeps as much air past its words as before its chevron, where the row asks it to.
        function test_the_air_past_the_words_can_be_the_air_before_the_mark() {
            shortName.endAir = Qt.binding(() => shortName.leadAir)
            compare(shortName.rightPadding, Math.ceil(shortName.leadAir))
        }

        /// The second press on a name takes its card down — and a press anywhere else still does.
        function test_a_second_press_on_the_name_takes_its_card_down() {
            mouseClick(hung)
            tryCompare(hung.card, "opened", true)
            verify(hung.standing, "the name stays lit while its card stands")
            mouseClick(hung)
            tryCompare(hung.card, "visible", false)
            verify(!hung.standing)
            mouseClick(hung)
            tryCompare(hung.card, "opened", true)
            mouseClick(root, 5, 190)
            tryCompare(hung.card, "visible", false)
        }

        /// …which is the policy's doing: under the default one the press on the name is "outside", closes the card,
        /// and the click the same press ends in opens it again. Pinned so a Qt that changed it is read here first.
        function test_the_default_policy_reopens_on_the_second_press() {
            mouseClick(plain)
            tryCompare(plain.card, "opened", true)
            mouseClick(plain)
            // Both halves of the press are delivered before this returns: the card shut on the press, and the click
            // it ended in opened it again.
            verify(plain.card.visible, "the default policy put the card straight back")
            plain.card.close()
            tryCompare(plain.card, "visible", false)
        }

        /// The chevron turns while its card stands, and nothing in the row moves for it: the steps are measured on the
        /// mark at rest.
        function test_opening_turns_the_mark_and_moves_nothing() {
            const whole = shortName.wholeWidth
            const lead = shortName.leftPadding
            shortName.opened = true
            compare(shortName.foldTurn, 90)
            compare(shortName.wholeWidth, whole)
            compare(shortName.leftPadding, lead)
            shortName.opened = false
        }
    }
}
