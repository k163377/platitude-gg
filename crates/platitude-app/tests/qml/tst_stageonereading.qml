import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// The page's reads move on a session's own thread, so they can change within the turn the report is read in: the
// completion's line says the reading that let the report be read, not a second one taken after it
// (`CompletionRoute.owedAtRead`). Here the read starts while the report is being read.
Item {
    id: root

    width: 100
    height: 100

    StageRig {
        id: rig
        owing: ["graph"]
    }
    Automation.PageStage {
        id: op
        name: "one_reading"
        onTriggered: {
            if (!op.at("start") || !Automation.Awaited.all("one_reading_ready", { "ready": true }))
                return
            op.complete(() => {
                rig.owing = ["graph", "readings"]
                return "one_reading read"
            }, ["graph"])
        }
    }

    TestCase {
        name: "StageOneReading"
        when: windowShown

        function test_the_line_says_the_reading_the_report_was_let_through_on() {
            op.start()
            tryVerify(() => rig.route.read)
            compare(rig.route.owedAtRead, ["graph"])
            const completed = Harness.said.filter(line => line.startsWith("complete run="))
            compare(completed.length, 1)
            compare(Stages.field(completed[0], "page"), "arriving:graph")
            compare(Stages.field(completed[0], "expected"), "graph")
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
