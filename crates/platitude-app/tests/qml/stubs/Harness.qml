pragma Singleton

import QtQuick

// Stands in for the app's `Harness` under `qmltestrunner`, which registers no Rust type (`cargo xtask qmltest` stages
// it into the `platitude` module): only what the harness's own machinery reads (`auto/Awaited.qml`,
// `auto/CompletionRoute.qml`) — the run's name, no held act — and the lines it says, kept in order for a test to read
// back. A test that leans on anything else here is testing this stand-in, not the app.
QtObject {
    property string shotDir: "pictures/stages-run"
    property bool faultHoldAct: false
    /// Every line said, in the order said.
    property var said: []

    function report(line) {
        said = said.concat([line])
    }
}
