//! Who a commit is by, and the boxes that say so — the identity dialog
//! and its badge, the signature, the caret in a message box, and the
//! settings card the avatar list comes down inside.

use super::Verb;

pub(super) const TABLE: &[Verb] = &[
    // The caret is the whole of these two, and a hook that never
    // reached the box leaves a picture of the resting colour --
    // which is a real state, and the other half of each pair.
    Verb {
        name: "edit-message-focus",
        when: &[],
        plain: "message_focus pane=details focused=true",
    },
    Verb {
        name: "wip-message-focus",
        when: &[],
        plain: "message_focus pane=wip focused=true",
    },
    // A dialog that stayed open because the write did not take looks
    // exactly like one nobody has answered yet, and "which half
    // landed" is not something a picture holds at all.
    Verb {
        name: "identity",
        when: &[],
        plain: "identity state=missing dialog=true nameSaved=false emailSaved=false unsaved=false",
    },
    Verb {
        name: "identity-half",
        when: &[],
        plain: "state=ready dialog=true nameSaved=true emailSaved=false unsaved=true",
    },
    // The three forced tooltips: a hook that never reached its
    // target photographs the resting state, which is a real state.
    // `tip=` is the attached ToolTip's own visible — the output
    // side, as everywhere.
    Verb {
        name: "identity-tip",
        when: &[],
        plain: "identity_tip unsaved=true badge=true tip=true",
    },
    Verb {
        name: "signature-tip",
        when: &[],
        plain: "signature_tip code=E tip=true",
    },
    // The same popup has a real loading and a causally settled form;
    // neither is selected by a millisecond window. Both are opened by
    // the field's own press, and `typing=` is the half of that press a
    // photograph cannot answer: the list must come down without taking
    // the caret out of the box it came from.
    Verb {
        name: "settings-tools-loading",
        when: &[],
        plain: "settled=false loading=true open=true typing=true",
    },
    Verb {
        name: "settings-tools",
        when: &[],
        plain: "settled=true loading=false open=true typing=true",
    },
    // The same card's avatar half. Only this one of its four wants a
    // line: the other three cannot reach the camera with the wrong
    // state, because each waits on the state itself — a filed picture
    // in the list, a lit row, a row gone from the store — and a run
    // where any of that never came photographs nothing at all. What
    // no predicate covers is the second grab: the list is a popup, it
    // lives in overlay.png, and a mirror refreshed over an empty
    // overlay passes (the blind spot `commit-menu` is here for). Three
    // popups because the card is modal — its dimmer, itself, and the
    // list that came down inside it.
    Verb {
        name: "avatar-combo",
        when: &[],
        plain: "overlay saved=true popups=3",
    },
];
