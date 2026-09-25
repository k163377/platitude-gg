// Whether what turns and slides in this window is held still — one answer for the whole application, because a ring
// caught mid-rotation photographs differently every run. Written from outside only; a person's window leaves it false.
//
// Not in `Metrics`: that holds constants, and this is a state somebody put the window in.
pragma Singleton

import QtQuick

QtObject {
    /// All that turns holds still. Read by every part that animates on its own clock (`SpinnerIcon`).
    property bool stilled: false
}
