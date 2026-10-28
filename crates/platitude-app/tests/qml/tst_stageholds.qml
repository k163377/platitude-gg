import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// The page's own holds on a completion that cannot be read — the page that claimed the run gone with its driver — end
// the run at once as a breach that says so, rather than standing to the ceiling (`CompletionRoute.held`).
Item {
    id: root

    width: 100
    height: 100

    property int reportReads: 0

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "holds"
        onTriggered: {
            if (!op.at("start") || !Automation.Awaited.all("holds_ready", { "ready": true }))
                return
            op.complete(() => {
                root.reportReads = root.reportReads + 1
                return "holds read"
            })
        }
    }

    TestCase {
        name: "StageHolds"
        when: windowShown

        function test_holds_that_throw_end_the_run_as_a_breach() {
            Automation.Awaited.pageHolds = () => { throw new Error("the page is gone") }
            op.start()
            tryVerify(() => rig.route.read)
            compare(root.reportReads, 0)
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].includes("why=the page's holds on the completion could not be read"), breaches[0])
        }
    }
}
