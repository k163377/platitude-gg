import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A completion asked by a generation its operation has since started over is an old generation's: once the hold
// passes, the route refuses it without reading the report, and the run ends there (`CompletionRoute.readReport`).
Item {
    id: root

    width: 100
    height: 100

    property int reportReads: 0

    StageRig {
        id: rig
        owing: ["details"]
    }
    Automation.PageStage {
        id: op
        name: "stale"
        onTriggered: {
            if (!op.at("start") || !Automation.Awaited.all("stale_ready", { "ready": op.generation() === 1 }))
                return
            op.complete(() => {
                root.reportReads = root.reportReads + 1
                return "stale read"
            })
        }
    }

    TestCase {
        name: "StageStale"
        when: windowShown

        function test_a_completion_from_a_generation_begun_over_is_refused_unread() {
            op.start()
            tryVerify(() => op.at("complete"))
            op.start()
            compare(op.generation(), 2)
            rig.owing = []
            tryVerify(() => rig.route.read)
            compare(root.reportReads, 0)
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=completed from generation 1 after generation 2 began"), breaches[0])
        }
    }
}
