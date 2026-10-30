import QtQuick
import QtQuick.Layouts
import QtTest
import platitude
import platitude.ui
import "stages.js" as Stages
import "../../src/auto" as Automation

// A list the layout leaves with no height, raised over rows: no size is the layout's answer, and a polish pass that
// began after the reading says so — the completion stands on it, no size and all, instead of waiting for a size that
// never comes (`CompletionRoute`, `Awaited.polishes`).
Item {
    id: root

    width: 100
    height: 100

    property int reportReads: 0
    /// The list's height as the completion read it.
    property real heightAtRead: -1

    readonly property QtObject graphState: QtObject {
        property int finishCount: 1
        property bool loading: false
        property bool failed: false
        property bool stale: false
        property bool wipRow: false
        function readingsSettled() { return true }
    }

    Window {
        id: stage
        width: 400
        height: 300
        visible: true

        Item {
            id: logPage
            anchors.fill: parent
            property var pageTab: ({state: "open", settledWrites: () => []})
            property var pageCommands: ({recorded: () => true})
            property var pageWorktree: ({loaded: true, wipRowStands: false})
            property bool pageRefsLoaded: true
            property bool pageDetailsSettled: true
            property bool pageCarriedSettled: true
            property bool pageLanding: false
            property var pageGraphPane: ({placing: false})
            property var pageGraph: root.graphState
            property bool raised: false
            ListModel {
                id: logRows
                ListElement { a: 1 }
                ListElement { a: 2 }
            }
            Loader {
                id: seat
                anchors.fill: parent
                active: logPage.raised
                sourceComponent: ColumnLayout {
                    property alias list: logList
                    spacing: 0
                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                    }
                    AppListView {
                        id: logList
                        Layout.fillWidth: true
                        Layout.preferredHeight: 0
                        Layout.maximumHeight: 0
                        model: logRows
                        delegate: Item {
                            width: 100
                            height: 20
                        }
                    }
                }
            }
        }
    }

    Automation.CompletionRoute {
        id: route
        owed: () => Automation.PageSettled.missing(logPage)
        unsized: () => Automation.PageSettled.unsized(logPage)
        onReadied: {
            root.heightAtRead = seat.item.list.height
            Automation.Awaited.completed("stages", route.completing, route.owedAtRead, route.witnessesRead)
        }
    }
    Component.onCompleted: {
        Automation.Awaited.route = route
        Automation.Awaited.window = stage
    }

    Automation.PageStage {
        id: op
        name: "zero"
        onTriggered: {
            if (!op.at("start"))
                return
            op.complete({ "name": "zero_ready", "when": () => ({ "ready": true }),
                          "report": () => {
                              root.reportReads = root.reportReads + 1
                              return "zero read"
                          } })
        }
    }

    TestCase {
        name: "StageSizedZero"
        when: windowShown

        function test_a_list_laid_out_with_no_height_completes_after_a_pass() {
            logPage.raised = true
            op.start()
            tryVerify(() => route.read)
            compare(root.heightAtRead, 0)
            verify(seat.item.list.width > 0, "laid out: it has its width")
            verify(Harness.said.some(line => line.startsWith("completion waiting=") && line.includes("sized:")),
                   Harness.said.join("\n"))
            compare(root.reportReads, 1)
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
