import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A list the page shows with rows and no size holds the completion for a polish pass of the window begun after the
// reading — no size is the layout's answer or its polish not run yet, and only such a pass tells — however long the
// hold beats meanwhile, and however many frames are said swapped (`CompletionRoute.markSizing`, `Awaited.polishes`).
Item {
    id: root

    width: 100
    height: 100

    property int reportReads: 0
    property int beats: 0

    StageRig {
        id: rig
        unsized: ["CommandsPane/AppListView"]
    }
    Automation.PageStage {
        id: op
        name: "unsized"
        onTriggered: {
            root.beats = root.beats + 1
            if (!op.at("start"))
                return
            op.complete({ "name": "unsized_ready", "when": () => ({ "ready": true }),
                          "report": () => {
                              root.reportReads = root.reportReads + 1
                              return "unsized read"
                          } })
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
        name: "StageUnsized"
        when: windowShown

        function test_a_list_with_rows_and_no_size_holds_the_completion_for_a_polish_pass() {
            Automation.Awaited.window = null
            op.start()
            tryVerify(() => op.at("complete"))
            const from = root.beats
            tryVerify(() => root.beats >= from + 6)
            compare(root.reportReads, 0)
            verify(!rig.route.read)
            verify(Harness.said.some(line => line.startsWith("completion waiting=")
                                    && line.includes("sized:CommandsPane/AppListView")),
                   Harness.said.join("\n"))
            // Frames said swapped are no polish pass.
            Automation.Awaited.frames = Automation.Awaited.frames + 3
            const swapped = root.beats
            tryVerify(() => root.beats >= swapped + 4)
            verify(!rig.route.read)
            Automation.Awaited.polishes = Automation.Awaited.polishes + 1
            tryVerify(() => rig.route.read)
            compare(root.reportReads, 1)
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
