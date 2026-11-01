pragma ComponentBehavior: Bound

import QtQuick

/// The one door from the window's own QML into the verification harness (`platitude.auto`), reached by URL: a shipped
/// build lacks the module, and a static type reference there would fail to load the window.
///
/// [`seats`] is handed to the part once, as it is built; a value that changes (the tab in front) is written by the
/// host onto [`driver`] with a `Binding`.
///
/// The only kind of seat: the window's (`Main.qml` → `WindowHarness`, holding `WindowAutoActDriver` / `AutoShotDriver`
/// / `WindowShotMirrors` / `WindowPerfDriver`) and each page's (`RepoPage.qml` → `PageHarness`, holding
/// `AutoActDriver` / `PagePerfDriver` / `PageAutoStart`). Something more a part acts on is added to its host's `seats`
/// — a page's menus whole, the rows picked out inside `PageHarness`. Only the window's seat stands behind everything
/// (`z: -10000`), since the shot stand-ins must; no other harness part draws.
Item {
    id: seat

    /// The part's file name inside the harness module.
    required property string part
    /// Whether this build carries the harness (`AppBackend.harnessPresent`) — all the product knows about it; what a
    /// run was told is the part's to read.
    required property bool wanted
    /// What the part is handed as it is built — its `required property` list, by name.
    property var seats: ({})
    /// The part, or `null` (ordinary runs, shipped builds). A property, so a `Binding` on it follows the part in; read
    /// it through [`ask`] during completion.
    property var driver: null

    /// The part, built now if it has not been yet. `Component.onCompleted` runs parent before child, so a host asking
    /// from its own completion would find the seat empty; the seat's own completion builds it anyway, since the shot
    /// stand-ins must stand even if no verb asks.
    function ask() {
        if (seat.driver === null && seat.wanted)
            seat.driver = seat.build()
        return seat.driver
    }

    Component.onCompleted: seat.ask()

    function build() {
        const component = Qt.createComponent("qrc:/qt/qml/platitude/auto/" + seat.part,
                                             Component.PreferSynchronous, seat)
        if (component.status !== Component.Ready) {
            // Unreachable: [`wanted`] and the module come from the same Cargo feature. A `wanted` read off the
            // environment would bring a shipped window here on a stray `PGG_*`.
            console.warn("harness part " + seat.part + " did not load: " + component.errorString())
            return null
        }
        return component.createObject(seat, seat.seats)
    }
}
