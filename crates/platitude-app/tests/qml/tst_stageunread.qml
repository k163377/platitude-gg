import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A completion handed a report already read — a line, not a function the route reads after its hold — is refused:
// what it says is of a state before the hold.
Item {
    width: 100
    height: 100

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "unread"
        onTriggered: {
            if (!Automation.Awaited.all("unread_ready", { "ready": true }))
                return
            op.complete("unread said=early")
        }
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
            verify(breaches[0].endsWith("why=completed with a report already read"), breaches[0])
        }
    }
}
