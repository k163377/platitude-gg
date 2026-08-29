//! The left list: the name boxes it opens, the rename gesture, what a
//! fold takes away, and the tooltips that spell out an elided row.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // A name git will not take — the branch beside this one already
    // carries it. **The box stays open holding what was typed** and git's
    // own words go under it, where closing it first would have thrown the
    // typing away and left the answer nowhere but the log
    // (デザイン規約 §答えの要らない報せ, by design).
    //
    // Nothing here is a picture: a box that closed and a box that stayed
    // open are a frame's colour apart, and the words are in a tooltip.
    // `tone=warning` is the other half — the gesture is still going, which
    // is the whole reason the box is still there.
    Verb {
        name: "rename-taken",
        when: &[],
        plain: "rename_taken open=true refused=true bar=true tone=warning why=",
    },
    // A tag renamed to its own name in other letters. **git writes a ref
    // as a file**, so on a case-insensitive disk both names would be gone
    // — core refuses it outright, and the box is where that answer belongs
    // (デザイン規約 §答えの要らない報せ, by design). Two runs make the
    // pair: the one that has to be turned down, and the one beside it that
    // must not be, since a box that refuses everything frames the same.
    Verb {
        name: "rename-tag-box",
        when: &[(
            Arg::Is("v1.0"),
            "tag_name_box was=v0.3-local typed=v1.0 refused=false why=",
        )],
        plain: "tag_name_box was=v0.3-local typed=V0.3-LOCAL refused=true \
                why=Only the letter case differs — on this disk that deletes both names",
    },
    // The row taken away before git answered for it, held open for
    // the picture. `row=-1` is the sidebar without it and `chips=true`
    // the graph without its chip — **both halves at once** is the whole
    // of the rule, and a run that only ever reached the state after
    // the refs landed would say the same `row=-1` for a different
    // reason (デザイン規約 §消す操作は先に画面から消す).
    Verb {
        name: "delete-gone",
        when: &[],
        plain: "gone_row tag=v0.3-local row=-1 total=2 chips=true",
    },
    // The fourth: an elided row whose hover says the whole name.
    // One verb serves both panes (the argument picks one), so the
    // wanted line names neither — `tree=` echoes which view the
    // argument asked for (`-tree` keeps the tree, where row 0 is
    // the elided folder chain), `tip=` the shared instance's own
    // visible.
    Verb {
        name: "path-tip",
        when: &[(Arg::Ends("-tree"), "tree=true tip=true")],
        plain: "tree=false tip=true",
    },
    // And the same tooltip taken away: `copied=` is the whole of the
    // tip read back out of a selection made on it, so it is false the
    // moment the words stop being a field the reader can drag over.
    // **A picture cannot answer this** — a selected line and an
    // unselected one differ by a wash the shot's own scaling can lose,
    // and a `Text` put back in place of the field would frame
    // identically.
    Verb {
        name: "tip-copy",
        when: &[],
        plain: "tip=true copied=true",
    },
    // ...and taken from the band around the words rather than off the
    // words themselves — every place in a tip that nobody else takes
    // is a start (規約 §hover のツールチップ). The starts are the air
    // itself, sampled: a grid over the tip with the points standing on
    // the sentence dropped, which is exactly what a real press could
    // reach the pad at.
    //
    // **`all=` and not a count.** How many places a tip's air has
    // depends on the length of the path in it and on the machine that
    // drew it, so the number rides along as diagnosis and the
    // judgement is "every one of them, and there was at least one".
    // `caret=` is beside it because a selection with no keyboard on it
    // is not one `Ctrl+C` can take (規約: 選択できることと持ち帰れる
    // ことは別).
    // `hand=` is the half a sweep cannot say for itself: a run enters
    // the pad's own functions, because a pointer cannot be injected, so
    // a pad whose `MouseArea` had been taken out would answer every
    // sweep it was asked and never see a press.
    Verb {
        name: "tip-sweep",
        when: &[],
        plain: "tip_sweep all=true caret=true hand=true",
    },
    // A row's tooltip. `lit=` is the control — the tip that came up
    // under the same pointer on the way in, so a run that photographs
    // an empty overlay is showing the row's answer and not a pointer
    // that never landed. `wants=` is the row's own words and `tip=`
    // the shared instance carrying them: every row of the sidebar
    // spells its name in full, so both are judged.
    Verb {
        name: "nav-tip",
        when: &[],
        plain: "lit=true wants=true tip=true",
    },
    // The left menu's rename gesture, and the absence that is the
    // whole of its bug: a click landing in the folded list's section
    // after that section went away and came back is an ordinary
    // click, not the gesture's second one. A picture cannot carry it
    // — the run that armed nothing frames as a folded rail with a
    // section beside it, which is what `nav-peek` frames as too. So
    // the pair is read out loud: `armed=` is the row's own answer to
    // the click it was given, `collapsed=`/`box=` what came of it
    // (the box puts the whole list back over the diff the fold was
    // made for — observed).
    Verb {
        name: "nav-reclick",
        when: &[],
        plain: "away=false armed=true collapsed=true box=true focused=true",
    },
    Verb {
        name: "nav-reclick-away",
        when: &[],
        plain: "away=true armed=false collapsed=true box=false",
    },
    // The same claim by the other door — the menu's `beginRename` —
    // and the rule it answers to: a box opens where its row is, so a
    // peeked row keeps both the fold and whatever the fold was made
    // for. `diff=` is the half that only the second one has a file
    // to lose.
    Verb {
        name: "nav-peek-rename",
        when: &[],
        plain: "collapsed=true diff=false editing=branch:",
    },
    Verb {
        name: "diff-fold-by-rename",
        when: &[],
        plain: "collapsed=true diff=true editing=branch:",
    },
    // The two ways the box is walked away from with nothing typed
    // into it. Same blind spot: a pane with no box in it frames like
    // a pane that never opened one.
    // And the row the box lands on, brought back from wherever the
    // list had scrolled to. A picture cannot carry it either: a
    // section showing its last rows and one showing its first frame
    // the same, and 2,000 tags all read alike.
    Verb {
        name: "nav-rename-far",
        when: &[],
        plain: "shown=true box=true",
    },
    Verb {
        name: "nav-rename-drop",
        when: &[(
            Arg::Is("fold"),
            "nav_drop how=fold collapsed=true box=false",
        )],
        plain: "nav_drop how=away collapsed=false box=false",
    },
    // The run that walks the list on past the row a box is standing
    // on: the box has to go with it, which is the whole of what a box
    // drawn outside its list owes. A sidebar with no box in it is the
    // same picture whether the box went or never opened.
    // The two name boxes a sidebar row opens, held at the width the
    // argument dragged the pane to. Whether what is in the box is cut
    // is in the picture; the other two are not — a box drawn where
    // nothing can be typed frames exactly like one waiting for a
    // name, and a box drawn outside its list has one more way to be
    // missing than a box in a row has.
    Verb {
        name: "nav-branch-box",
        when: &[(Arg::Ends(":away"), "open=true focused=true shown=false")],
        plain: "open=true focused=true shown=true",
    },
    Verb {
        name: "nav-rename-box",
        when: &[(Arg::Ends(":away"), "open=true focused=true shown=false")],
        plain: "open=true focused=true shown=true",
    },
    // A tag that was made. The write answers before the read that puts
    // the row in TAGS, so a run judged at the write barrier photographs
    // a sidebar with nothing new in it and passes — and the count alone
    // would pass for a tag left on whatever HEAD happened to be, which
    // is why the commit it was asked for is echoed too.
    Verb {
        name: "create-tag",
        when: &[(Arg::Ends(":box"), "create_tag box=true mode=tag")],
        plain: "create_tag tag=v9.9-new row=1 total=4 at=true",
    },
    // The third mode of that same field, judged on the same line for
    // the same reasons. Which mode it came up in is the one thing
    // here the picture *does* answer: the question is written in the
    // box (`Create tag here?`), and it is all an empty box holds.
    Verb {
        name: "nav-tag-box",
        when: &[],
        plain: "open=true focused=true shown=true",
    },
    // The same walking away, over the graph: a press that landed
    // somewhere else takes an empty box with it and leaves one with
    // something typed in it standing. Both halves are read off the
    // report because the picture holds only one of the two states,
    // and the card the run is about fades — so `shown=` says which
    // end of that fade the picture was taken at.
    Verb {
        name: "find-drop",
        when: &[(Arg::Is(""), "find_drop open=false shown=false")],
        plain: "find_drop open=true shown=true",
    },
    Verb {
        name: "name-box-drop",
        when: &[(Arg::Has(":"), "name_drop box=true")],
        plain: "name_drop box=false",
    },
];
