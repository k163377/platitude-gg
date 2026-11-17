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
    // (`hub::life::resolve_git` falls back to `PATH`; the chapter is the only
    // place that says so).
    if is_stored_missing(verb, arg) {
        let settings = config_dir.join("settings.toml");
        let gone = missing_git(config_dir);
        // Doubled, since in a TOML string a lone `\` is an escape.
        let escaped = gone.replace('\\', r"\\");
        std::fs::write(
            &settings,
            format!("version = 1\n\n[defaults]\ngit_path = \"{escaped}\"\n"),
        )
        .map_err(|e| format!("could not write {}: {e}", settings.display()))?;
        println!("seeded settings: a git that is not there ({gone})");
    }

    state(config_dir, verb)?;

    // Fetch-on-open off, for the fixtures an opening fetch would destroy
    // (デザイン規約 §リモートから取り込む; `session::fetch_on_open` asks the
    // interval). `outrun` is "the remote moved and this end has not
    // looked": fetched, it is `demo::remote::diverged` and the toolbar
    // offers `push -f`. `unpublished`'s `outsider` is a name never fetched
    // here (`RemoteBranchState::Unknown`, from `commit::is_in_history`
    // failing); fetched, the comparison answers `refused`.
    // `publish-unreachable`'s origin is nowhere: an opening fetch would
    // fail first and stand its own failure where the publish's ask is to
    // be read.
    //
    // Keyed on the preset, not the verb: any verb run on these would
    // picture the neighbouring fixture under this one's name. `perf` wants
    // it because a measurement is of this machine alone.
    if verb == "perf"
        || presets
            .iter()
            .any(|p| p == "outrun" || p == "unpublished" || p == "publish-unreachable")
    {
        let settings = config_dir.join("settings.toml");
        std::fs::write(
            &settings,
            "version = 1\n\n[defaults]\nauto_fetch_minutes = 0\n",
        )
        .map_err(|e| format!("could not write {}: {e}", settings.display()))?;
        println!("seeded settings: auto fetch off (so the window opens without looking)");
    }

    Ok(())
}

/// The window the run opens in, and the layout a verb needs standing.
///
/// Every run that reads its store gets a window size: one never saved
/// takes a share of the screen (`WindowShape.firstLength`), and the screen
/// here is the offscreen platform's 800x800 (or `--screen`'s), not the
/// 1440x900 the pictures are drawn for. A state already in the directory
/// (an explicit `--config-dir`) is kept, given the size only where it has
/// no `[window]`, unless the verb seeds a shape of its own. `solo` and
/// `gate-sweep` read no store (`child::stages_a_held_store`): theirs is the
/// window that opens unsized.
fn state(config_dir: &Path, verb: &str) -> Result<(), String> {
    // A window under every floor — what a file written before there was a
    // floor looks like, which the way in has to lift (`window-floor`). The
    // stepping verbs ride it for the height: the graph and the diff they
    // walk (`spread`, `dirty`'s `untracked.txt`) fit the run's 1440x900
    // window, and a pane with nothing to scroll answers every step the way
    // a broken one would.
    let under_floor = verb == "window-floor"
        || verb == "graph-step-edge"
        || verb == "graph-step-far"
        || verb == "diff-step"
        || verb == "diff-step-edge";
    // `details-fit`: at the default 400 the author row is already clamped
    // to its block, so a row that never filled answers like one that did.
    // Seeded wide, a missing fill shows. `diff-band-sweep`: the diff band
    // cuts its path only when too narrow for it, and 1100 of the window's
    // 1440 to the details pane leaves the band shorter than every preset's
    // path.
    let details = match verb {
        "details-fit" => Some((800, "wider than the row asks for")),
        "diff-band-sweep" => Some((1100, "so the band has to cut")),
        _ => None,
    };
    let (width, height) = if under_floor { (320, 240) } else { (1440, 900) };
    let window = format!("[window]\nwidth = {width}\nheight = {height}\nmaximized = false\n");
    let state = config_dir.join("state.toml");
    if !under_floor && details.is_none() {
        let Ok(kept) = std::fs::read_to_string(&state) else {
            std::fs::write(&state, format!("version = 1\n\n{window}"))
                .map_err(|e| format!("could not write {}: {e}", state.display()))?;
            println!("seeded window: 1440x900");
            return Ok(());
        };
        if kept.lines().any(|line| line.trim() == "[window]") {
            return Ok(());
        }
        let joined = if kept.ends_with('\n') { "\n" } else { "\n\n" };
        std::fs::write(&state, format!("{kept}{joined}{window}"))
            .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded window: 1440x900 (beside the state already there)");
        return Ok(());
    }
    let mut text = format!("version = 1\n\n{window}");
    if let Some((details, _)) = details {
        text.push_str(&format!("\n[layout]\ndetails_width = {details}\n"));
    }
    std::fs::write(&state, text)
        .map_err(|e| format!("could not write {}: {e}", state.display()))?;
    match details {
        Some((details, why)) => println!("seeded layout: a {details}px details pane ({why})"),
        None => println!("seeded window: {width}x{height} (under every floor)"),
    }
    Ok(())
}

fn is_stored_missing(verb: &str, arg: &str) -> bool {
    verb == "settings-git-path" && arg == "stored-missing"
}

/// The path `stored-missing` seeds: nothing there, spelled with the OS's
/// own separators as a hand writes the file. The screen shows them as `/`
/// (`native=false`) and the file is to keep them ([`stored_path_kept`]).
fn missing_git(config_dir: &Path) -> String {
    config_dir.join("no-such-git.exe").display().to_string()
}

/// Whether the path `stored-missing` seeded is still in the run's
/// settings file as it was written, read once the app has ended; `None`
/// for every other run. The run makes the way out's write before it
/// reports, so a box that wrote its screen spelling back shows here — the
/// report itself only holds what the screen holds. Either TOML string
/// form counts, in case a save rewrote the file with the value intact.
pub(super) fn stored_path_kept(verb: &str, arg: &str, config_dir: &Path) -> Option<bool> {
    if !is_stored_missing(verb, arg) {
        return None;
    }
    let gone = missing_git(config_dir);
    let escaped = gone.replace('\\', r"\\");
    let text = std::fs::read_to_string(config_dir.join("settings.toml")).unwrap_or_default();
    Some(text.contains(&format!("\"{escaped}\"")) || text.contains(&format!("'{gone}'")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    /// A directory of this test's own: the tests write the same file name.
    fn config_dir(stem: &str) -> PathBuf {
        crate::verify::claim_dir(&crate::temp::root().join("pgg-verify"), stem)
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

    /// The witness reads the value, not the file's bytes: a save that kept
    /// it in TOML's other string form passes, one that respelled it fails.
    #[test]
    fn the_stored_path_witness_reads_the_seeded_spelling_back() {
        let dir = config_dir("seed-stored-missing");
        super::config(&dir, "settings-git-path", "stored-missing", &[])
            .expect("the seed is written");
        assert_eq!(
            super::stored_path_kept("settings-git-path", "stored-missing", &dir),
            Some(true),
            "the seed as written"
        );
        assert_eq!(super::stored_path_kept("settings-git-path", "", &dir), None);
        assert_eq!(
            super::stored_path_kept("settings-escape", "stored-missing", &dir),
            None
        );

        let gone = super::missing_git(&dir);
        let settings = dir.join("settings.toml");
        let rewrite = |value: &str| {
            std::fs::write(
                &settings,
                format!("version = 1\n\n[defaults]\ngit_path = {value}\n"),
            )
            .expect("the file is rewritten");
            super::stored_path_kept("settings-git-path", "stored-missing", &dir)
        };
        assert_eq!(
            rewrite(&format!("'{gone}'")),
            Some(true),
            "a literal string keeps the value"
        );
        assert_eq!(rewrite("\"\""), Some(false), "an emptied box written back");
        if cfg!(windows) {
            let slashed = gone.replace('\\', "/");
            assert_eq!(
                rewrite(&format!("\"{slashed}\"")),
                Some(false),
                "the screen's spelling written back"
            );
        }
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

    /// Left unsized, every picture would come out a share of the offscreen
    /// screen; a verb's own layout keeps the size beside it.
    #[test]
    fn every_run_opens_at_the_size_its_pictures_are_drawn_for() {
        let dir = config_dir("seed-window");
        super::config(&dir, "nav-tip", "", &[]).expect("the seed is written");
        let state = std::fs::read_to_string(dir.join("state.toml")).expect("a state of its own");
        assert!(state.contains("width = 1440\nheight = 900\n"), "{state}");

        let laid = config_dir("seed-window-layout");
        super::config(&laid, "details-fit", "", &[]).expect("the seed is written");
        let state = std::fs::read_to_string(laid.join("state.toml")).expect("a state of its own");
        assert!(state.contains("width = 1440\nheight = 900\n"), "{state}");
        assert!(state.contains("details_width = 800\n"), "{state}");
    }

    /// A directory named twice comes back to what the first run left.
    #[test]
    fn a_state_already_there_is_kept() {
        let dir = config_dir("seed-window-kept");
        let kept = "version = 1\n\n[window]\nwidth = 1000\nheight = 700\n";
        std::fs::write(dir.join("state.toml"), kept).expect("the earlier run's state");
        super::config(&dir, "nav-tip", "", &[]).expect("nothing to seed");
        let state = std::fs::read_to_string(dir.join("state.toml")).expect("still there");
        assert_eq!(state, kept);
    }

    /// A hand-written state that says nothing of the window opens at the
    /// run's size too, with everything it does say kept.
    #[test]
    fn a_state_without_a_window_is_given_one() {
        let dir = config_dir("seed-window-added");
        let kept = "version = 1\n\n[tabs]\npaths = [\"a\"]";
        std::fs::write(dir.join("state.toml"), kept).expect("a hand-written state");
        super::config(&dir, "nav-tip", "", &[]).expect("the window is added");
        let state = std::fs::read_to_string(dir.join("state.toml")).expect("still there");
        assert!(state.starts_with(kept), "{state}");
        assert!(
            state.contains("\n\n[window]\nwidth = 1440\nheight = 900\n"),
            "{state}"
        );
    }
}
