import QtQuick
import platitude
import "../../src/auto" as Automation

// The harness's completion route over reads a test holds back (`owing`), set as the machinery's route, with the
// completion said once the route has read the report — what `WindowHarness` does over the page's reads. A file of
// `tst_stage*.qml` has one run: the machinery is a singleton of the file's engine, and a run completes or breaks once.
Item {
    id: rig

    property var owing: []
    /// The lists with rows and no size the page shows (`PageSettled.unsized`).
    property var unsized: []
    readonly property alias route: route

    Automation.CompletionRoute {
        id: route
        owed: () => rig.owing
        unsized: () => rig.unsized
        onReadied: Automation.Awaited.completed("stages", route.completing, route.owedAtRead, route.witnessesRead)
    }

    Component.onCompleted: Automation.Awaited.route = route
}
