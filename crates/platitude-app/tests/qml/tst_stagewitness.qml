import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// History and the present in one report: the value before the input is kept as a witness of the stage it was read at,
// the press's own answer as one of the pressed stage, and the value the run claims is read when the completion is
// confirmed. The completion's line names the witnesses the report read. A witness read at the completion would say
// `was=after`; a report read when the completion was asked would say `now=pressed`.
Item {
    id: root

    width: 100
    height: 100

    property string subject: "before"

    StageRig {
        id: rig
        owing: ["details"]
    }
    Automation.PageStage {
        id: op
        name: "witness"
        order: ["pressed"]
        onTriggered: {
            if (op.at("start")) {
                op.witness("was", root.subject)
                op.input("pressed", () => {
                    root.subject = "pressed"
                    op.witness("took", root.subject)
                })
                return
            }
            op.complete({ "name": "witness_ready", "when": () => ({ "moved": root.subject !== "before" }),
                          "report": () => "witness was=" + op.witnessed("was") + " took=" + op.witnessed("took")
                                          + " now=" + root.subject })
        }
    }

    TestCase {
        name: "StageWitness"
        when: windowShown

        function test_the_witnesses_are_kept_and_the_claim_is_read_at_the_completion() {
            op.start()
            tryVerify(() => op.at("complete"))
            root.subject = "after"
            rig.owing = []
            tryVerify(() => rig.route.read)
            compare(Harness.said.filter(line => line.startsWith("witness was=")),
                    ["witness was=before took=pressed now=after"])
            const kept = Harness.said.filter(line => Stages.kind(line) === "witness")
            compare(kept.map(line => Stages.field(line, "stage") + ":" + Stages.field(line, "name")),
                    ["start:was", "pressed:took"])
            const completed = Harness.said.filter(line => line.startsWith("complete run="))
            compare(completed.length, 1)
            compare(Stages.field(completed[0], "witnesses"), "page/witness/1/start/was,page/witness/1/pressed/took")
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
