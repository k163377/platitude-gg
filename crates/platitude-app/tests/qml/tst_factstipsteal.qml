import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What the supplement a row opens does to the hand that asked for it (デザイン規約 §左メニューの所作): the lines a
// WORKTREES row opens keep the path for a rest over them, and a tip that took the pointer would put the hover out —
// the row would close, the tip with it, and the hand would be back where it started, over and over.
// The real part under a real pointer (`qmltestrunner` delivers hover — verify-ui スキル).
Item {
    id: root
    width: 400
    height: 200

    property bool pointed: false
    /// How often `pointed` has changed — what a blink would run up.
    property int flips: 0

    // The window's tooltip, as `Main` keeps it.
    HandWatch { id: hand }
    TipSeat {
        host: root
        hand: hand
    }

    NavRowFacts {
        id: facts
        width: 260
        height: 60
        // The lines as `NavFacts.lines` hands them over.
        lines: [{ "mark": "branch", "markTint": Theme.accent, "text": "feature/topic-a",
                  "tone": Theme.textSecondary, "ahead": 1, "behind": 0 }]
        path: "C:/Users/somebody/IdeaProjects/project/.claude/worktrees/d"
    }

    // A sampler of the path tip standing (`NavRowFacts.tipShown`, its `TipAsk.shown`), counting its flips; a
    // `HoverHandler` here would be a second reader of the same pointer.
    Timer {
        id: watch
        interval: 16
        repeat: true
        running: true
        onTriggered: {
            const on = facts.tipShown
            if (on !== root.pointed) {
                root.pointed = on
                root.flips++
            }
        }
    }

    TestCase {
        name: "FactsTipSteal"
        when: windowShown

        function test_the_supplement_stands_still_under_a_hand_that_stays() {
            mouseMove(facts, 100, 30)
            tryVerify(() => facts.tipShown, undefined, "the rest over the lines opens the path")
            // The watch is a sampler: until it has met the tip standing, the tip's own arrival would count below as
            // the blink.
            tryVerify(() => root.pointed, undefined, "and the watch has met it standing")

            const stood = root.flips
            // Long enough for a tip that took the pointer to go down and come back (the return waits out
            // `Metrics.tipDelayMs`).
            // waits(paced): the subject is a state that must **not** change, so the wait is the window it is watched
            // over; nothing here answers sooner.
            wait(Metrics.tipDelayMs * 3)
            verify(facts.tipShown, "and it is still up with the hand where it was")
            compare(root.flips, stood, "having neither gone down nor come back in between")
        }

        /// Leaving the lines is the first step of the walk to the box, and the box follows its target's words while
        /// it stands — words bound to the ask would be wiped on the way out, and the box the beat keeps up would be
        /// empty.
        function test_the_box_keeps_its_words_when_the_hand_sets_off_for_it() {
            mouseMove(facts, 100, 30)
            tryVerify(() => facts.tipShown, undefined, "the rest over the lines opens the path")
            const box = Tips.box
            tryCompare(box, "text", facts.path, undefined, "and the box is wearing it")

            // The box is a popup over the panel, so the lines stop being asked.
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => facts.says === "", undefined, "the ask falls with the pointer")
            compare(box.text, facts.path, "the box still has the words the hand is walking towards")
        }
    }
}
