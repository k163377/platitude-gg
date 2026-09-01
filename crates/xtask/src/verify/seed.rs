//! The configuration a verb needs standing before the window opens.
//!
//! Five verbs are about a window the app itself could never write: one
//! under every floor the layout has, a pane wider than the row inside
//! it, a band too narrow to hold a path. The app writes only shapes it
//! could take, so nothing inside it can produce these — the way in is
//! the only place that can, and it does it here rather than by hand.

use std::path::Path;

/// Writes what `verb` has to start from into the run's own config
/// directory. Says what it seeded, because a picture of a seeded window
/// and one of a window that came up that way read the same.
pub(super) fn config(config_dir: &Path, verb: &str) -> Result<(), String> {
    // The verbs that need the configuration to say something before the
    // run starts: a window smaller than any floor the layout has. It is
    // what a file written before there was a floor looks like, and the
    // way in is the only place that can put it right. Written here rather
    // than by hand in the app, because the app never writes a shape it
    // could not take — so nothing inside it could produce this state.
    // `window-floor` is about the lift itself; the stepping verbs ride
    // the same seed for the height, since no demo repository has more
    // commits than the default window shows at once, and a graph with
    // nothing below the fold has no viewport rule to answer. The diff's
    // arrows want it for the same reason from the other side: no demo
    // file's diff is taller than a default window either, and a pane with
    // nothing to scroll answers every step the way a broken one would.
    if verb == "window-floor"
        || verb == "graph-step-edge"
        || verb == "graph-step-far"
        || verb == "diff-step"
        || verb == "diff-step-edge"
    {
        let state = config_dir.join("state.toml");
        std::fs::write(
            &state,
            "version = 1\n\n[window]\nwidth = 320\nheight = 240\nmaximized = false\n",
        )
        .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded window: 320x240 (under every floor)");
    }

    // The one verb whose arrangement the window would destroy on its way
    // in: a push is only out of date while this end has not looked at the
    // remote, and opening a tab fetches once (デザイン規約 §リモートから取り込む).
    // Turning the timer off turns that one off with it, which is also
    // what leaves the toolbar offering a plain `push` — an end that had
    // fetched would know it was diverged and offer the overwrite instead.
    if verb == "push-outdated" || verb == "perf" {
        let settings = config_dir.join("settings.toml");
        std::fs::write(
            &settings,
            "version = 1\n\n[defaults]\nauto_fetch_minutes = 0\n",
        )
        .map_err(|e| format!("could not write {}: {e}", settings.display()))?;
        println!("seeded settings: auto fetch off (so the window opens without looking)");
    }

    // The other side of the same idea: `details-fit` is about the pane's
    // right column, and half of what it claims only bites where the
    // author row is narrower than the block it stands in. At the default
    // 400 the row is already wider than the block, so it is clamped to
    // the block whether or not it was told to fill — and a row that
    // never filled answers exactly like one that did. Seeded wide, the
    // same run puts the hash plate 540px short of the pane's edge the
    // moment the fill is gone.
    if verb == "details-fit" {
        let state = config_dir.join("state.toml");
        std::fs::write(&state, "version = 1\n\n[layout]\ndetails_width = 800\n")
            .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded layout: an 800px details pane (wider than the row asks for)");
    }
    // The band above the diff cuts its path at the head, and a band with
    // room for the whole path never cuts at all — so the run that has to
    // say the cut works has to be given a narrow band. Seeded from the
    // other side: the details pane is what the diff is left over from,
    // and 1100 of the window's 1440 leaves the band a couple of hundred
    // pixels, which every path in the presets is longer than.
    if verb == "diff-band-sweep" {
        let state = config_dir.join("state.toml");
        std::fs::write(&state, "version = 1\n\n[layout]\ndetails_width = 1100\n")
            .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded layout: an 1100px details pane (so the band has to cut)");
    }

    Ok(())
}
