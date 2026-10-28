import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A completion handed a report already read — a line, not a function the route reads in the turn its reading holds —
// is refused: what it says is of a state before the hold. A function that answers a value read before is the same
// thing, and the static check names it where it is written (`cargo xtask waits`'s `stale`): at run time the two read
// alike.
Item {
    width: 100
    height: 100

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "unread"
        onTriggered: op.complete({ "name": "unread_ready", "when": () => ({ "ready": true }),
                                   "report": "unread said=early" })
    }

    TestCase {
        name: "StageUnread"
        when: windowShown

        function test_a_report_already_read_is_refused() {
            op.start()
            tryVerify(() => rig.route.read)
            verify(!Harness.said.includes("unread said=early"))
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=completed without a `when` and a `report` to read at the completion"),
                   breaches[0])
        }
    }
}
