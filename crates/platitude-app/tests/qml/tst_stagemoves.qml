import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// The value the report claims moves while the completion is held: the line says the value at the reading the
// completion was confirmed on, not the value when the verb asked to complete. A route that read the report when it
// was asked would say `before`.
Item {
    id: root

    width: 100
    height: 100

    property string subject: "before"
    property int reportReads: 0

    StageRig {
        id: rig
        owing: ["details"]
    }
    Automation.PageStage {
        id: op
        name: "moves"
        onTriggered: {
            if (!op.at("start"))
                return
            op.complete({ "name": "moves_ready", "when": () => ({ "ready": true }),
                          "report": () => {
                              root.reportReads = root.reportReads + 1
                              return "moves subject=" + root.subject
                          } })
        }
    }

    TestCase {
        name: "StageMoves"
        when: windowShown

        function test_the_report_says_the_state_the_completion_was_confirmed_on() {
            op.start()
            tryVerify(() => op.at("complete"))
            compare(root.reportReads, 0)
            root.subject = "after"
            rig.owing = []
            tryVerify(() => rig.route.read)
            compare(Harness.said.filter(line => line.startsWith("moves subject=")), ["moves subject=after"])
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
