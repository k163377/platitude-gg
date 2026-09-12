import QtQuick
import QtTest
import platitude.ui

// Who the standing command log belongs to, walked through the orders the news can arrive in (`CommandsOwner`).
//
// **Order is the whole of this rule and none of it is on screen.** A failure that came before the fetch's and one
// that comes after it leave the same panel standing with the same red mark in the corner, and the reader's own press
// leaves no mark at all — so no photograph tells the paths apart, and a headless run pins one path per run at a
// window's worth of cost. The rule was lifted out of the page for this: here every path is a line.
//
// The verbs still own the wiring (`fetch-recover` / `fetch-recover-held`): that a press reaches `readerTakes` and a
// landing reaches `landingTakesItDown` is a fact about the page, and this file cannot see the page.
Item {
    id: root
    width: 200
    height: 100

    CommandsOwner {
        id: owner
    }

    TestCase {
        id: paths
        name: "CommandsOwner"

        function init() {
            owner.heldByFetch = false
        }

        // ---- the paths, one at a time ---------------------------------

        function test_a_failure_that_raised_a_shut_panel_takes_it_down_again() {
            owner.fetchRaises(false)
            verify(owner.heldByFetch, "the panel this raised is the fetch's")
            verify(owner.landingTakesItDown(0, false), "the fetch that landed takes its own panel down")
            verify(!owner.heldByFetch, "and hands it back")
        }

        function test_a_panel_the_reader_had_up_is_left_where_it_stands() {
            owner.fetchRaises(true)
            verify(!owner.heldByFetch, "a panel already standing is the reader's")
            verify(!owner.landingTakesItDown(0, false), "so nothing takes it down")
        }

        function test_the_press_that_shuts_it_hands_the_panel_over() {
            owner.fetchRaises(false)
            owner.readerTakes()
            verify(!owner.landingTakesItDown(0, false), "the reader closed it; a landing has nothing to close")
        }

        function test_the_press_that_opens_it_again_is_the_readers_too() {
            owner.fetchRaises(false)
            owner.readerTakes()
            owner.readerTakes()
            verify(!owner.landingTakesItDown(0, false), "a panel the reader put back up stays up")
        }

        function test_news_that_lands_after_the_fetch_keeps_the_panel_up() {
            owner.fetchRaises(false)
            owner.newsTakes()
            verify(!owner.landingTakesItDown(0, false), "the panel is saying something else as well")
        }

        // The path the product was asked for by name: a failure, the reader shutting it, a fetch failure on top of
        // the red mark that failure left, and then the network coming back.
        function test_news_that_came_before_the_fetch_is_not_in_this_panel() {
            owner.newsTakes()
            owner.readerTakes()
            owner.fetchRaises(false)
            verify(owner.heldByFetch, "the panel standing now is the fetch's, whatever the mark still says")
            verify(owner.landingTakesItDown(0, false), "so the fetch takes it down")
        }

        function test_a_run_that_is_still_failing_takes_nothing_down() {
            owner.fetchRaises(false)
            verify(!owner.landingTakesItDown(2, false), "one fetch of a run answering is not the run ending")
            verify(owner.heldByFetch, "and the panel is still the fetch's")
            verify(owner.landingTakesItDown(0, false), "the fetch that ends the run takes it down")
        }

        function test_a_line_somebody_else_left_holds_the_panel_up() {
            owner.fetchRaises(false)
            verify(!owner.landingTakesItDown(0, true), "a line still standing is not this fetch's")
            verify(owner.heldByFetch, "the panel is the fetch's until somebody takes it")
            verify(owner.landingTakesItDown(0, false), "and goes down when the header is clear")
        }

        function test_the_panel_goes_down_once() {
            owner.fetchRaises(false)
            verify(owner.landingTakesItDown(0, false), "the recovery")
            verify(!owner.landingTakesItDown(0, false), "every drain after it asks the same question")
        }

        function test_a_second_failure_keeps_a_panel_the_fetch_already_holds() {
            owner.fetchRaises(false)
            verify(!owner.landingTakesItDown(0, true), "held, but a line is standing")
            owner.fetchRaises(true)
            verify(owner.heldByFetch, "raised again over its own panel, which is still the fetch's")
            verify(owner.landingTakesItDown(0, false), "and comes down when that clears")
        }

        function test_a_fetch_that_raised_nothing_closes_nothing() {
            verify(!owner.landingTakesItDown(0, false), "no failure, no panel, nothing to take down")
        }

        function test_a_press_before_any_fetch_leaves_the_rule_where_it_was() {
            owner.readerTakes()
            verify(!owner.landingTakesItDown(0, false), "there was no panel to hand over")
        }

        // ---- and every order of them ----------------------------------
        //
        // The cases above are the paths a reader can describe; this is the rest of them. The events are walked to a
        // depth of three and each sequence is judged against the rule said the other way round — **the panel goes
        // down if, and only if, the last thing to touch it was a failure raising one that was shut** — so an
        // implementation that drifted into some longer order has to disagree with that sentence to pass.

        readonly property var moves: ["fetch-shut", "fetch-open", "reader", "news"]

        function play(sequence) {
            owner.heldByFetch = false
            for (let i = 0; i < sequence.length; i++) {
                const move = sequence[i]
                if (move === "fetch-shut")
                    owner.fetchRaises(false)
                else if (move === "fetch-open")
                    owner.fetchRaises(true)
                else if (move === "reader")
                    owner.readerTakes()
                else
                    owner.newsTakes()
            }
        }

        /// The rule said as a sentence: a panel is the fetch's while the last move that could have taken it is a
        /// failure that found it shut. `fetch-open` is not one of those — it neither takes a held panel nor claims
        /// one that was never held.
        function shouldComeDown(sequence) {
            let held = false
            for (let i = 0; i < sequence.length; i++) {
                const move = sequence[i]
                if (move === "fetch-shut")
                    held = true
                else if (move === "reader" || move === "news")
                    held = false
            }
            return held
        }

        function test_every_order_of_three_moves_agrees_with_the_rule() {
            const moves = paths.moves
            let walked = 0
            for (let a = 0; a < moves.length; a++) {
                for (let b = 0; b < moves.length; b++) {
                    for (let c = 0; c < moves.length; c++) {
                        const sequence = [moves[a], moves[b], moves[c]]
                        play(sequence)
                        const wanted = shouldComeDown(sequence)
                        compare(owner.landingTakesItDown(0, false), wanted, sequence.join(" > "))
                        // A run of failures that has not ended, and a line that is not this fetch's, take nothing
                        // down whatever the order was.
                        play(sequence)
                        compare(owner.landingTakesItDown(1, false), false, sequence.join(" > ") + " mid-run")
                        play(sequence)
                        compare(owner.landingTakesItDown(0, true), false, sequence.join(" > ") + " under a line")
                        walked++
                    }
                }
            }
            compare(walked, 64, "every order of three moves")
        }
    }
}
