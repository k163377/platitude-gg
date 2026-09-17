// Whether what turns and slides in this window is held still.
//
// One answer for the whole application, because a ring is drawn in four places and a picture of one caught
// mid-rotation comes out differently every time — no two runs of the same verb would produce the same PNG. Written
// from outside; nothing in the window sets it, and every window a person opens leaves it false.
//
// A singleton of its own: `Metrics` is a verbatim mirror of the design document's
// constant tables (規約 §トークンの三層), and this is a state somebody put the window in.
pragma Singleton

import QtQuick

QtObject {
    /// All that turns holds still. Read by every part that animates on its own clock (`SpinnerIcon`).
    property bool stilled: false
}
