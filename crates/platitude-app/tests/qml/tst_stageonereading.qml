import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// The page's reads move on a session's own thread, so they can change within the turn the report is read in. A
// report read across such a change is of no one state: it is thrown away unsaid, and the hold reads again until a
// report and the readings on both sides of it agree — the line says that reading (`CompletionRoute.attempt`). Here a
// read starts while the first report is being read, and lands later.
Item {
    id: root

    width: 100
    height: 100

    property int reads: 0

    StageRig {
        id: rig
        owing: ["graph"]
    }
    Automation.PageStage {
        id: op
        name: "one_reading"
        onTriggered: {
            if (!op.at("start"))
                return
            op.complete({ "name": "one_reading_ready", "when": () => ({ "ready": true }), "arriving": ["graph"],
                          "report": () => {
                              root.reads = root.reads + 1
                              if (root.reads === 1)
                                  rig.owing = ["graph", "readings"]
                              return "one_reading read=" + root.reads
                          } })
        }
    }

    TestCase {
        name: "StageOneReading"
        when: windowShown

        function test_a_report_read_across_a_change_is_thrown_away_and_read_again() {
            op.start()
            tryVerify(() => root.reads === 1)
            verify(!rig.route.read)
            verify(!Stages.reports(Harness.said).some(line => line.startsWith("one_reading read=")))
            rig.owing = ["graph"]
            tryVerify(() => rig.route.read)
            compare(root.reads, 2)
            compare(Stages.reports(Harness.said).filter(line => line.startsWith("one_reading read=")), ["one_reading read=2"])
            compare(rig.route.owedAtRead, ["graph"])
            const completed = Harness.said.filter(line => line.startsWith("complete run="))
            compare(completed.length, 1)
            compare(Stages.field(completed[0], "page"), "arriving:graph")
            compare(Stages.field(completed[0], "expected"), "graph")
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
