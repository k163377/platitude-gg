//! The window and its strip of tabs: what it fills, the floor it stops
//! at, which tab was carried where, and the band that gives way when it
//! is crowded.

use super::Verb;

pub(super) const TABLE: &[Verb] = &[
    // `edge=` rides along in that report but is not judged: whether an
    // edge would land off the screen is a question about a real
    // monitor, and the offscreen platform has none to answer with.
    Verb {
        name: "window-fill",
        when: &[],
        plain: "window_fill fills=true",
    },
    // A window held at its floor and one let past it frame alike — the
    // picture is of the panes either way, and the one that went past
    // simply has a pane outside the frame, where a screenshot cannot
    // follow. What the floor came to is a number or it is nothing.
    Verb {
        name: "window-floor",
        when: &[],
        plain: "window_floor fits=true",
    },
    // Which tab went is the whole question, and every demo working
    // tree is called `repo`, so the strip photographs the same either
    // way. `gone=` is the pressed tab's own path, asked of the strip
    // after it caught up with the model — a run that closed the
    // neighbour, or one that reported before a tab had ever opened,
    // is caught by nothing else. How many are left is not judged: the
    // verb takes as many repositories as it is given.
    Verb {
        name: "middle-close",
        when: &[],
        plain: "middle_close gone=true",
    },
    // What a tab switch carries and what it drops, in one line
    // because neither half means anything alone. `sessions=1` with
    // two tabs open is the release itself — the tab left behind is
    // holding no repository — and it is the one thing here no picture
    // can say. `folded=` / `log=` are the layout the reader left the
    // last tab in, found on the next one; `empty=` is that tab's own
    // commit editor, which the words did *not* follow into; `back=`
    // is those words still standing where they were typed, after
    // everything else on that page was thrown away and read again;
    // `wip=` is the pane holding them being the one on screen, which
    // is the half of "kept" that a report about text alone misses.
    Verb {
        name: "tab-carry",
        when: &[],
        plain: "sessions=1 folded=true log=true empty=true back=true wip=true",
    },
    // Which tab was carried and where it came to rest. A strip whose
    // order merely changed passes with any two tabs swapped, and one
    // where the carry never took hold photographs the order it
    // started in — which is a strip that looks like every other one.
    // `moved=` is the carried tab's own path, found at the place it
    // was asked for.
    Verb {
        name: "tab-drag",
        when: &[],
        plain: "tab_drag moved=true",
    },
    // The other half of the same gesture, and the one the settled
    // strip cannot hold: a strip whose tab was never drawn away from
    // its own row photographs exactly like one whose rows only ever
    // jumped. `lifted=` is the offset the transform is carrying, read
    // off the tab rather than off what was asked of it.
    Verb {
        name: "tab-hold",
        when: &[],
        plain: "tab_hold lifted=true",
    },
    // And the strip travelling under a tab held past its end. The
    // picture is a scrolled strip either way — the one that travelled
    // and the one that was already there frame alike — so what is
    // judged is that the tab reached the far end of an order it could
    // not see when the hand took hold.
    Verb {
        name: "tab-edge",
        when: &[],
        plain: "tab_edge landed=true",
    },
    // The ☰'s card, standing. `yield=true` is the half no picture holds:
    // the band's empty run is the platform's caption, and while the card
    // is up it has to stop being that or every press landing there is
    // swallowed and the card never goes down. A run where the run was
    // never handed back frames the same open card.
    Verb {
        name: "app-menu",
        when: &[],
        plain: "app_menu open=true yield=true",
    },
    // And the mark pressed a second time. The card that reopened on that
    // press and the card that was never closed are the same photograph —
    // which is how this went unnoticed — so what is judged is the card
    // being down with the run back in the platform's hands.
    Verb {
        name: "app-menu-reclick",
        when: &[],
        plain: "app_menu open=false yield=false",
    },
    // The three states themselves. A run where one of them never stood
    // photographs a band that was never crowded, and that picture
    // cannot be told from a band that gave the crowd room — so the
    // crowd has to be said out loud.
    //
    // Neither the fit nor which of the group's three shapes landed is
    // judged: this verb takes a width, and the widths that show the
    // last shape are below the floor a hand can drag the window to
    // (`fits=false` is what was asked for there). The floor itself is
    // `window-floor`'s question.
    Verb {
        name: "badges",
        when: &[],
        plain: "op=true conflicts=true identity=true",
    },
    // The card, opened. `rows=` is the half the picture cannot carry
    // on its own: a card with one row and a card with three frame the
    // same way once it is cropped to the band, and which rows arrived
    // is the whole question the group raises when it gives way.
    Verb {
        name: "badges-hover",
        when: &[],
        plain: "card=true rows=op,conflicts,identity",
    },
    // The fourth badge. A run whose shim never reached PATH reads the
    // git this machine has, wears no badge, and photographs an
    // ordinary window — which is exactly what an ordinary window looks
    // like. `badge=` is the band's own reading, so the whole path from
    // `git --version` to the row is what passes or fails here.
    Verb {
        name: "old-git",
        when: &[],
        plain: "old-git badge=true",
    },
    // And the card it opens. `rows=` is not judged: the other three
    // rows come and go with the machine (a container with no identity
    // configured stands one of them), and only this row is the verb's.
    Verb {
        name: "old-git-card",
        when: &[],
        plain: "old-git badge=true card=true",
    },
    // The band folded with nothing red standing in it. Two halves, and
    // the picture holds neither on its own: a mark that never came up
    // frames as a band with room to spare, and the colour of three
    // dots is not something a cropped screenshot settles an argument
    // about. `tint=` is named rather than spelled in hex — what is
    // being judged is which rule painted it (規約 §状態: 最も重い状態が
    // 決める), and a red mark over a lone warning is the way that rule
    // fails silently.
    Verb {
        name: "old-git-fold",
        when: &[],
        plain: "mark=true tint=warning",
    },
];
