//! What a pane is judged against: the room it was given, and the grip
//! that asks for more of it.
//!
//! `solo` is here because it is the same kind of claim about the run
//! itself — the window had the machine to itself, which an ordinary
//! window photographs exactly like.

use super::Verb;

pub(super) const TABLE: &[Verb] = &[
    Verb {
        name: "solo",
        when: &[],
        plain: "solo blocked=true",
    },
    Verb {
        name: "details-fit",
        when: &[],
        plain: "details_fit fits=true",
    },
    // The author's name, capped at its own width so the signature's one
    // word stays against it. A ceiling drawn a fraction of a pixel under
    // that width elides the name, and an ellipsis is what a picture
    // answers worst — it is a few pixels wide, both OSes draw one, and
    // the box around it does not move, so the run that lost the last
    // glyph frames exactly like the run that kept it (2026-08-23
    // ユーザー報告: Ubuntu drew `Yuki Tana…` where Windows drew
    // `Yuki Tanaka`, in the same 78px of ink). At the window the verbs
    // open, the row has 200px of slack — nothing here is meant to give.
    Verb {
        name: "author-card",
        when: &[],
        plain: "author_card cut=false",
    },
    Verb {
        name: "author-card-open",
        when: &[],
        plain: "author_card cut=false",
    },
    // A pull that took more room than the pane had leaves what sits
    // under the box — the author card, the commit button — drawn over
    // the window's own footer, and that frames like a pane that fits:
    // the same blind spot details-fit answers for. Only that half is
    // judged here: whether the grip was offered at all depends on the
    // message, and the run where it stays away is half of the pair.
    Verb {
        name: "details-grow",
        when: &[],
        plain: "description_grow keeps=true",
    },
    Verb {
        name: "details-grow-squeeze",
        when: &[],
        plain: "description_grow keeps=true",
    },
    Verb {
        name: "wip-grow",
        when: &[],
        plain: "description_grow keeps=true",
    },
    Verb {
        name: "wip-grow-squeeze",
        when: &[],
        plain: "description_grow keeps=true",
    },
];
