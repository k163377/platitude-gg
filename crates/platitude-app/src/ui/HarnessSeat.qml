pragma ComponentBehavior: Bound

import QtQuick

/// The one door from the window's own QML into the verification harness.
///
/// The harness is a QML module of its own (`platitude.auto`, `src/auto/`) that a shipped build does not carry: it is
/// embedded under the `automation` Cargo feature, and `cargo build --release` is the build without it. **So
/// `platitude.auto`'s types are reached by URL** — a static type reference is a load failure in the build that has
/// none, and the window would not come up at all. A seat loads its part by URL, which in that build is a part that is
/// simply never built.
///
/// What the host hands over goes in [`seats`] and is set as the part is built, once. That is the whole of the
/// contract: the objects a verb acts on are the ids of one window or one page and stay put under it. A value that
/// does change (the tab in front) is written by the host onto [`driver`] with a `Binding`.
Item {
    id: seat

    /// The part's file name inside the harness module.
    required property string part
    /// Whether this build carries the harness at all (`AppBackend.harnessPresent`) — **the whole of what the product
    /// knows about it**. What a run was told to do is the part's own to read, and a shipped build leaves the seat
    /// empty and pays for nothing but this Item.
    required property bool wanted
    /// What the part is handed as it is built — its `required property` list, by name.
    property var seats: ({})
    /// The part, or `null`: every ordinary run, and every shipped build. A property, so a `Binding` on it follows the
    /// part in. **Ask for it with [`ask`] from anything that runs during completion.**
    property var driver: null

    /// The part, built now if it has not been yet.
    ///
    /// **`Component.onCompleted` runs parent before child** (measured: a page's own handler logged ahead of this
    /// seat's), so a host that reaches for the harness from its completion handler would find the seat still empty.
    /// Every reader goes through here, and the seat's own completion is just the reader of last resort — the
    /// stand-ins a shot is taken from have to stand whether or not a verb ever asks for anything.
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
            // Unreachable, and said out loud: [`wanted`] and the module this loads from come
            // from the one Cargo feature, so a build that asks has one to load and a build that has none never
            // asks. What used to get here was a shipped window with a stray `PGG_*` still in its environment.
            console.warn("harness part " + seat.part + " did not load: " + component.errorString())
            return null
        }
        return component.createObject(seat, seat.seats)
    }
}
