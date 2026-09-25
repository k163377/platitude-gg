//! The configuration a run needs standing before the window opens: states
//! the app never writes itself (it writes only shapes it could take), so
//! only a file written here produces them.

use std::path::Path;

/// Writes what the run starts from into its own config directory, and
/// says so — a seeded window and one that came up that way photograph the
/// same. Window shapes key on `verb`; an arrangement the opening would
/// undo keys on `presets`.
pub(super) fn config(
    config_dir: &Path,
    verb: &str,
    arg: &str,
    presets: &[String],
) -> Result<(), String> {
    // A git the store names and the machine lacks. Only a file gets here:
    // the screen asks a path for its version before storing it
    // (`Hub::resolve_git` falls back to `PATH`; the chapter is the only
    // place that says so).
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

    // A window under every floor — what a file written before there was a
    // floor looks like, which the way in has to lift (`window-floor`). The
    // stepping verbs ride it for the height: no demo graph or diff is
    // taller than a default window, and a pane with nothing to scroll
    // answers every step the way a broken one would.
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

    // Fetch-on-open off, for the fixtures an opening fetch would destroy
    // (デザイン規約 §リモートから取り込む; `session::fetch_on_open` asks the
    // interval). `outrun` is "the remote moved and this end has not
    // looked": fetched, it is `demo::remote::diverged` and the toolbar
    // offers `push -f`. `unpublished`'s `outsider` is a name never fetched
    // here (`RemoteBranchState::Unknown`, from `is_in_head_history`
    // failing); fetched, the comparison answers `refused`.
    //
    // Keyed on the preset, not the verb: any verb run on these would
    // picture the neighbouring fixture under this one's name. `perf` wants
    // it because a measurement is of this machine alone.
    if verb == "perf" || presets.iter().any(|p| p == "outrun" || p == "unpublished") {
        let settings = config_dir.join("settings.toml");
        std::fs::write(
            &settings,
            "version = 1\n\n[defaults]\nauto_fetch_minutes = 0\n",
        )
        .map_err(|e| format!("could not write {}: {e}", settings.display()))?;
        println!("seeded settings: auto fetch off (so the window opens without looking)");
    }

    // `details-fit`: at the default 400 the author row is already clamped
    // to its block, so a row that never filled answers like one that did.
    // Seeded wide, a missing fill shows.
    if verb == "details-fit" {
        let state = config_dir.join("state.toml");
        std::fs::write(&state, "version = 1\n\n[layout]\ndetails_width = 800\n")
            .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded layout: an 800px details pane (wider than the row asks for)");
    }
    // The diff band cuts its path only when too narrow for it: 1100 of the
    // window's 1440 to the details pane leaves the band shorter than
    // every preset's path.
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

    /// A directory of this test's own: the tests write the same file name.
    fn config_dir(stem: &str) -> PathBuf {
        crate::verify::claim_dir(&std::env::temp_dir().join("pgg-verify"), stem)
            .expect("a config directory nobody else has")
    }

    /// The seed follows the preset through any verb, one not yet written
    /// included: a run that fetched would PASS on a picture of `diverged`
    /// without a word.
    #[test]
    fn the_preset_that_must_not_fetch_is_seeded_under_any_verb() {
        let dir = config_dir("seed-outrun");
        super::config(&dir, "nav-tip", "", &["outrun".to_string()]).expect("the seed is written");
        let settings = std::fs::read_to_string(dir.join("settings.toml"))
            .expect("a preset that opens quiet is given settings of its own");
        assert!(
            settings.contains("auto_fetch_minutes = 0"),
            "the seeded settings turn the fetching off: {settings}"
        );
    }

    #[test]
    fn the_preset_whose_third_shape_a_fetch_would_answer_is_seeded_too() {
        let dir = config_dir("seed-unpublished");
        super::config(&dir, "publish-taken", "", &["unpublished".to_string()])
            .expect("the seed is written");
        let settings = std::fs::read_to_string(dir.join("settings.toml"))
            .expect("a preset that opens quiet is given settings of its own");
        assert!(
            settings.contains("auto_fetch_minutes = 0"),
            "the seeded settings turn the fetching off: {settings}"
        );
    }

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
