import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A press that answers false put nothing in, and kept nothing either: what it witnessed is dropped with it, so the
// press tried again on the next beat keeps its own value under the same name — once, in the run's history too.
Item {
    id: root

    width: 100
    height: 100

    property int tries: 0
    property string subject: "before"

    StageRig {
        id: rig
        owing: ["details"]
    }
    Automation.PageStage {
        id: op
        name: "retry"
        order: ["pressed"]
        onTriggered: {
            if (op.at("start")) {
                op.input("pressed", () => {
                    root.tries++
                    op.witness("tried", root.tries)
                    // The first press finds nothing to press yet.
                    if (root.tries < 2)
                        return false
                    root.subject = "pressed"
                    return true
                })
                return
            }
            op.complete({ "name": "retry_ready", "when": () => ({ "moved": root.subject !== "before" }),
                          "report": () => "retry tried=" + op.witnessed("tried") })
        }
    }

    TestCase {
        name: "StageWitnessRetry"
        when: windowShown

        function test_a_press_that_put_nothing_in_kept_no_witness() {
            op.start()
            tryVerify(() => op.at("complete"))
            rig.owing = []
            tryVerify(() => rig.route.read)
            compare(root.tries, 2)
            compare(Stages.breaches(Harness.said), [])
            compare(Harness.said.filter(line => line.startsWith("retry tried=")), ["retry tried=2"])
            const kept = Harness.said.filter(line => Stages.kind(line) === "witness")
            compare(kept.map(line => Stages.field(line, "stage") + ":" + Stages.field(line, "name")),
                    ["pressed:tried"])
        }
    }
}
