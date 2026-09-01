pragma ComponentBehavior: Bound

import QtQuick

/// The one door from the window's own QML into the verification harness.
///
/// The harness is a QML module of its own (`platitude.auto`, `src/auto/`) that a shipped build does not carry: it is
/// embedded under the `automation` Cargo feature, and `cargo build --release` is the build without it. **So nothing in
/// `platitude.ui` may name a type from it** — a static type reference is a load failure in the build that has none, and
/// the window would not come up at all. A seat loads its part by URL instead, which in that build is a part that is
/// simply never built.
///
/// What the host hands over goes in [`seats`] and is set as the part is built, once. That is the whole of the
/// contract: the objects a verb acts on are the ids of one window or one page and do not change under it. A value that
/// does change (the tab in front) is written by the host onto [`driver`] with a `Binding`.
Item {
    id: seat

    /// The part's file name inside the harness module.
    required property string part
    /// Whether this run asked for the part at all. False leaves the seat empty even in a build that carries the
    /// harness, so an ordinary run pays for nothing but this Item.
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
    /// Every reader goes through here instead, and the seat's own completion is just the reader of last resort — the
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
            // Only reachable in a build that was told to drive itself and has no harness to drive with, which is
            // exactly what has to be said out loud: every other run never gets here.
            console.warn("harness part " + seat.part + " did not load: " + component.errorString())
            return null
        }
        return component.createObject(seat, seat.seats)
    }
}
