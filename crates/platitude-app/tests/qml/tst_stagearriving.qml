import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A completion that photographs a read on its way names that read, and only that read is let through: another read
// still owed holds the completion as it holds any (`StageTimer.complete`'s `arriving`, `CompletionRoute.held`).
Item {
    id: root

    width: 100
    height: 100

    property int reportReads: 0
    property int beats: 0

    StageRig {
        id: rig
        owing: ["graph", "details"]
    }
    Automation.PageStage {
        id: op
        name: "arriving"
        onTriggered: {
            root.beats = root.beats + 1
            if (!op.at("start") || !Automation.Awaited.all("arriving_ready", { "ready": true }))
                return
            op.complete(() => {
                root.reportReads = root.reportReads + 1
                return "arriving read"
            }, ["graph"])
        }
    }
    /// Beats while the operation stands completed: what "held this long" is counted on.
    Automation.PageStage {
        id: clock
        name: "clock"
        running: true
        onTriggered: root.beats = root.beats + 1
    }

    TestCase {
        name: "StageArriving"
        when: windowShown

        function test_only_the_read_a_completion_names_is_let_through() {
            op.start()
            tryVerify(() => op.at("complete"))
            const from = root.beats
            tryVerify(() => root.beats >= from + 6)
            compare(root.reportReads, 0)
            rig.owing = ["graph"]
            tryVerify(() => rig.route.read)
            compare(root.reportReads, 1)
            const completed = Harness.said.filter(line => line.startsWith("complete run="))
            compare(completed.length, 1)
            compare(Stages.field(completed[0], "page"), "arriving:graph")
            compare(Stages.field(completed[0], "expected"), "graph")
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
