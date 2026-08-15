import QtQuick
import QtQuick.Controls.Fusion

// A scroll bar pinned visible while its view overflows. The default
// AsNeeded policy re-derives visibility from transient view state and
// has been seen dropping the bar entirely around model swaps, so the
// policy is computed from content size instead. Attached to a
// Flickable, the bar's parent is the view itself.
// The comparison carries a pixel of slack. Text heights are fractional --
// a font whose line box is 23.5 makes a two-line item 47.0 inside a frame
// laid out at 47 -- and a strict `>` turns a rounding remainder no eye can
// see into a bar down the side of a view that has nothing to scroll
// (2026-08-15 ユーザー報告). A real overflow is a line of text, never a
// fraction of one, so nothing that should scroll is lost by this.
ScrollBar {
    readonly property Flickable view: parent as Flickable
    policy: view && view.contentHeight + view.topMargin + view.bottomMargin
                    > view.height + 1
            ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
}
