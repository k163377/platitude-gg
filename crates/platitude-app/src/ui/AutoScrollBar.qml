import QtQuick
import QtQuick.Controls.Fusion

// A scroll bar pinned visible while its view overflows. The default
// AsNeeded policy re-derives visibility from transient view state and
// has been seen dropping the bar entirely around model swaps, so the
// policy is computed from content size instead. Attached to a
// Flickable, the bar's parent is the view itself.
ScrollBar {
    readonly property Flickable view: parent as Flickable
    policy: view && view.contentHeight + view.topMargin + view.bottomMargin
                    > view.height
            ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
}
