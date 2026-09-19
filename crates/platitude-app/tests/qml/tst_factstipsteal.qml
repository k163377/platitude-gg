import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What the supplement a row opens does to the hand that asked for it (デザイン規約 §左メニューの所作): the lines a
// WORKTREES row opens keep the path for a rest over them, and a tip that took the pointer with it would put the hover
// out — the row would close, the tip would go with it, and the hand would be back where it started, over and over.
//
// **The real part, and a real pointer**: `NavRowFacts` takes plain values, so it stands here as it stands in a row,
// and `qmltestrunner` is the one place hover can be delivered for real (verify-ui スキル).
Item {
    id: root
    width: 400
    height: 200

    /// The pointer is on the lines, as they answer it themselves.
    property bool pointed: false
    /// How often that answer has changed — what a blink would run up.
    property int flips: 0

    NavRowFacts {
        id: facts
        width: 260
        height: 60
        // The lines as the table hands them over (`NavFacts.lines`) — the part draws what it is given.
        lines: [{ "mark": "branch", "markTint": Theme.accent, "text": "feature/topic-a",
                  "tone": Theme.textSecondary, "ahead": 1, "behind": 0 }]
        path: "C:/Users/somebody/IdeaProjects/project/.claude/worktrees/d"
    }

    // The lines' own hover, read the way the row reads it (`NavItemDelegate.syncHover`). A `HoverHandler` here would
    // be a second reader of the same pointer; this asks the part what it says.
    Timer {
        id: watch
        interval: 16
        repeat: true
        running: true
        onTriggered: {
            const on = facts.ToolTip.visible
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
            // The hand comes to rest on the lines and does not move again.
            mouseMove(facts, 100, 30)
            tryVerify(() => facts.ToolTip.visible, undefined, "the rest over the lines opens the path")

            const stood = root.flips
            // Long enough for a tip that took the pointer to lose it, go down, and come back — the blink this is
            // about (`Metrics.tipDelayMs` is what the second coming would wait out).
            // waits(paced): the subject is a state that must **not** change, so the wait is the window it is watched
            // over; nothing here answers sooner.
            wait(Metrics.tipDelayMs * 3)
            verify(facts.ToolTip.visible, "and it is still up with the hand where it was")
            compare(root.flips, stood, "having neither gone down nor come back in between")
        }
    }
}
