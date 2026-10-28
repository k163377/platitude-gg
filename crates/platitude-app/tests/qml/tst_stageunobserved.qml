import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A stage moved on a beat whose latest reading of the product did not hold is refused: a stage moves on what the
// product answered, never on a count of beats.
Item {
    id: root

    width: 100
    height: 100

    property int beats: 0

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "unobserved"
        onTriggered: {
            root.beats = root.beats + 1
            Automation.Awaited.all("unobserved_row", { "row": false })
            op.advance("moved")
        }
    }

    TestCase {
        name: "StageUnobserved"
        when: windowShown

        function test_a_stage_moved_with_no_reading_holding_is_refused() {
            op.start()
            tryVerify(() => rig.route.read)
            verify(root.beats >= 1)
            compare(Stages.chain(Harness.said, "page", "unobserved", 1), [])
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=moved to moved with the beat's latest observation not holding"),
                   breaches[0])
        }
    }
}
