//! The configuration a run needs standing before the window opens.
//!
//! Five verbs are about a window the app itself could never write: one
//! under every floor the layout has, a pane wider than the row inside
//! it, a band too narrow to hold a path. The app writes only shapes it
//! could take, so nothing inside it can produce these — the way in is
//! the only place that can, and it does it here rather than by hand.

use std::path::Path;

/// Writes what the run has to start from into its own config directory.
/// Says what it seeded, because a picture of a seeded window and one of a
/// window that came up that way read the same.
///
/// Reads both halves of what a run is: a window shape is the verb's
/// business, and an arrangement the opening would undo is the fixture's,
/// so `presets` is asked as well as `verb`.
pub(super) fn config(
    config_dir: &Path,
    verb: &str,
    arg: &str,
    presets: &[String],
) -> Result<(), String> {
    // A git the store names and the machine does not have. **Only a file
    // can put the window here**: the screen stores a path it has just
    // asked for a version, so nothing typed into it ends up in this
    // state — what is photographed is a run that opened on a git that
    // has since gone (`Hub::resolve_git` falls back to `PATH`, and the
    // chapter is the only place that says so).
    if verb == "settings-git-path" && arg == "stored-missing" {
        let settings = config_dir.join("settings.toml");
        // Forward slashes: this is TOML, where a Windows path's
        // separators would be escapes.
        let gone = config_dir.join("no-such-git.exe").display().to_string();
        let gone = gone.replace('\\', "/");
        std::fs::write(
            &settings,
            format!("version = 1\n\n[defaults]\ngit_path = \"{gone}\"\n"),
        )
        .map_err(|e| format!("could not write {}: {e}", settings.display()))?;
        println!("seeded settings: a git that is not there ({gone})");
    }

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

    // The one arrangement the window would destroy on its way in, and it
    // belongs to the fixture rather than to whoever opens it: `outrun`
    // **is** "the remote moved and this end has not looked", and opening
    // a tab fetches once (デザイン規約 §リモートから取り込む). Turning the timer
    // off turns that one off with it (`session::fetch_on_open` asks the
    // interval for its permission), which is also what leaves the toolbar
    // offering a plain `push` — an end that had fetched would know it was
    // diverged and offer the overwrite instead.
    //
    // `unpublished` is the same requirement from the other side: its
    // `outsider` is a name put there by another clone and **never
    // fetched here**, which is the whole of the third shape a first push
    // can meet — neither answer can be given from this end, so the bar
    // wears the frame and the `!` (`RemoteBranchState::Unknown` comes of
    // `is_in_head_history` failing on a commit this repository does not
    // hold). An opening that fetches puts that commit in the repository,
    // and the comparison then answers `refused` like any other diverged
    // name — on every run, there being no race here to win (measured: 10
    // of 10 of `publish-taken outsider --preset unpublished` reported
    // `far=refused code=push -f theirs=1`).
    //
    // **Keyed on the preset, not on a verb.** A run that fetches on the
    // way in is not photographing `outrun` at all, it is photographing
    // `demo::remote::diverged` — which is literally `behind`, a fetch,
    // and the same commit on top. Named by verb, this reached the one
    // that presses `push` and left every other run to picture the
    // neighbouring preset under this preset's name (observed: `nav-tip
    // branch:0 --preset outrun` framed an ahead 1 / behind 1 row and a
    // `push -f`).
    //
    // `perf` is here for a reason of its own: a measurement is not to
    // reach the network at all.
    if verb == "perf" || presets.iter().any(|p| p == "outrun" || p == "unpublished") {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    /// A directory of this run's own to seed into: two of these tests
    /// write the same file name, and a shared path would let one read
    /// what the other wrote.
    fn config_dir(stem: &str) -> PathBuf {
        crate::verify::claim_dir(&std::env::temp_dir().join("pgg-verify"), stem)
            .expect("a config directory nobody else has")
    }

    /// **The one seed that is the fixture's rather than the verb's.** A
    /// run that opens `outrun` with the timer on fetches on its way in
    /// and photographs `diverged` instead, so the settings have to follow
    /// the preset through whatever verb asks for it — a verb nobody has
    /// written yet included, since such a run PASSes on a picture of the
    /// wrong fixture and says nothing about it.
    #[test]
    fn the_preset_that_must_not_fetch_is_seeded_under_any_verb() {
        let dir = config_dir("seed-outrun");
        super::config(&dir, "nav-tip", "", &["outrun".to_string()]).expect("the seed is written");
        let settings = std::fs::read_to_string(dir.join("settings.toml"))
            .expect("a preset that must not fetch is given settings of its own");
        assert!(
            settings.contains("auto_fetch_minutes = 0"),
            "the seeded settings turn the fetching off, not something else: {settings}"
        );
    }

    /// The second fixture with a name on the far side it must not have
    /// read: `unpublished`'s `outsider` is the only shape a first push
    /// has that neither answer fits, and an opening that fetches hands
    /// the comparison the commit it was supposed to be missing — the run
    /// then photographs a diverged name (measured: `refused`, ten times
    /// out of ten) under this preset's name and PASSes doing it.
    #[test]
    fn the_preset_whose_third_shape_a_fetch_would_answer_is_seeded_too() {
        let dir = config_dir("seed-unpublished");
        super::config(&dir, "publish-taken", "", &["unpublished".to_string()])
            .expect("the seed is written");
        let settings = std::fs::read_to_string(dir.join("settings.toml"))
            .expect("a preset that must not fetch is given settings of its own");
        assert!(
            settings.contains("auto_fetch_minutes = 0"),
            "the seeded settings turn the fetching off, not something else: {settings}"
        );
    }

    /// And no other fixture is quieted. A preset with nothing to lose to
    /// an opening's fetch runs on the settings the application ships,
    /// which is the half that says this is a fixture's requirement rather
    /// than a blanket for every headless run.
    #[test]
    fn a_preset_with_nothing_to_lose_is_left_on_the_shipped_settings() {
        let dir = config_dir("seed-basic");
        super::config(&dir, "nav-tip", "", &["basic".to_string()]).expect("nothing to seed");
        assert!(
            !dir.join("settings.toml").exists(),
            "a preset that does not care about the opening's fetch was given settings anyway"
        );
    }
}
