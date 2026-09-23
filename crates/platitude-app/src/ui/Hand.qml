// Whether a hand is over this window at all.
//
// One answer for the whole application, and the same reason `Motion` has one: a picture has to show what was done to
// the window and nothing else. A headless run has no hand — it cannot inject a pointer, so what it means to light it
// lights through a stand-in (`HoverToolButton.pointedAt`) — and yet the platform it runs on keeps a cursor at the
// screen's own origin, which the window comes up on. Whatever the window's top left corner holds is handed that
// pointer and washes for the whole run, in every picture, with nothing having pointed at it.
//
// Written from outside; nothing in the window sets it, and every window a person opens leaves it false.
pragma Singleton

import QtQuick

QtObject {
    /// No hand is anywhere over this window, so a pointer lights nothing. Read wherever a real pointer would
    /// (`HoverToolButton.lit`).
    property bool away: false
}
