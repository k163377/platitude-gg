//! The application singleton: git version gate + automation hooks.

use std::sync::Arc;

use platitude_core::version;
use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

mod avatars;
mod build_id;
mod identity;
mod lifecycle;
mod persist;
mod qobject;
mod start;

pub use build_id::build_tree;

use start::{Assignments, assignments, section_open, with_flag, with_layout, with_window};
// The directory the application's own configuration reads are made in.
// Shared with the settings screen's `GLOBAL` identity chapter, which reads
// and writes the same file (`identity` in core).
pub(crate) use start::app_workdir;

enum AppMsg {
    GitOk {
        version: String,
        /// Whether that version is at or above the supported minimum
        /// (see `AppBackend::git_unsupported`).
        supported: bool,
    },
    GitMissing {
        message: String,
    },
    GitError {
        message: String,
    },
    /// Identity git would put on a new commit here.
    Identity {
        name: String,
        email: String,
    },
    /// The identity could not be read at all — a state of its own.
    IdentityUnknown {
        message: String,
    },
    /// One candidate git answered — or did not. `path` is what was asked,
    /// so an answer that arrives after the box has moved on is dropped
    /// where it lands.
    GitPathProbed {
        path: String,
        probe: version::Probe,
        /// Whether the git asked is the one this run spawns — the binary
        /// itself, whatever the spelling (`process::same_program`). Read
        /// beside the probe: the compare reads the filesystem (the
        /// launcher looked behind, a link resolved), and a path on a
        /// server that is not answering takes as long to fail as the
        /// name does.
        names_the_run: bool,
    },
    /// The readback and verdict travel together: a drain between two
    /// messages would expose `ready` before `unsaved` and dismiss the gate.
    IdentitySaved(Result<platitude_core::identity::IdentityWrite, String>),
}

pub struct AppBackend {
    git_state: String,
    git_version: String,
    /// The git that answered is older than [`platitude_core::version::MINIMUM_GIT`].
    /// Everything still runs — this is the band's fourth badge: most of
    /// what the app asks for works on an older git, and the ones that do
    /// not fail with git's own words where they are asked for.
    git_unsupported: bool,
    /// The supported minimum, printed by the missing-git screen and by the
    /// badge's card. Read from core so neither can drift from the version
    /// the check actually compares against.
    minimum_git: String,
    git_error: String,
    /// "unknown" until the check runs, then "checking" / "missing" /
    /// "ready" / "error". Only "missing" opens the setup screen.
    identity_state: String,
    identity_name: String,
    identity_email: String,
    identity_error: String,
    identity_busy: bool,
    /// Which half of the last save git now reports as what was asked for.
    /// The two are how a half-written identity shows on screen as the
    /// half it is, and they stay false until a save has finished —
    /// nothing has been asked for before that.
    identity_name_saved: bool,
    identity_email_saved: bool,
    /// A save finished without both halves landing. Separate from the two
    /// above because it also carries "a save has been tried", which is
    /// what keeps the marks and the toolbar badge out of a fresh window.
    identity_unsaved: bool,
    /// The window keeps the platform's own title bar above an ordinary
    /// tab row (`Main.captionMerged`). **Read once, as the window is
    /// created** — a window that came up without a way to close it
    /// cannot be taken back — so it sits here, where a run can only
    /// read it.
    system_title_bar: bool,
    /// This build carries the verification harness, so the seats that load
    /// it by URL have something to load (`HarnessSeat.wanted`). The one
    /// thing the product knows about the harness at all; everything a run
    /// was told to do is on `Harness`, which a shipped build has no type
    /// for (`crate::harness::singleton`).
    harness_present: bool,
    /// Auto-fetch interval in minutes; 0 is off. Application-wide, because
    /// the answer is about how often this computer should talk to remotes.
    auto_fetch_minutes: i32,
    /// Ceiling the settings input enforces.
    auto_fetch_max: i32,
    /// How many git processes run at once, application-wide for the
    /// reason the interval is: the answer is about this machine
    /// (`platitude_core::process::Slots`). The ceiling and what an
    /// empty box falls back to are core's, held here for the input.
    git_concurrency: i32,
    git_concurrency_max: i32,
    git_concurrency_default: i32,
    /// How often the other working copies of a repository are read for
    /// uncommitted work, in seconds; 0 is off. The floor and ceiling are
    /// core's (`session::copies_interval_secs`). The page's tick reads
    /// the same number in milliseconds (`copies_interval_ms`), computed
    /// here so the timer binds to a property of its own.
    copies_interval_secs: i32,
    copies_interval_min: i32,
    copies_interval_max: i32,
    copies_interval_ms: i32,
    /// Commits a graph opens with; 0 is the whole history, which the
    /// screen asks for with a box of its own. Application-wide
    /// for the same reason the interval is: the answer is about how much
    /// history this person reads at once.
    initial_commits: i32,
    /// Floor the settings input enforces. There is no ceiling of ours —
    /// the property's own type is the only one (`session::log_limit`).
    initial_commits_min: i32,
    /// What an empty input falls back to, and what the input shows behind
    /// a reader who has not typed one. Held here, because the number is
    /// core's (`session::DEFAULT_LOG_LIMIT`).
    initial_commits_default: i32,
    /// The git this computer runs, as the settings file holds it: a path,
    /// or empty for whichever one `PATH` resolves. Application-wide for
    /// the same reason the interval is — which git is installed is an
    /// answer about the machine itself.
    git_path: String,
    /// What the git at [`Self::git_path`] said when it was last asked its
    /// version: "checking" while the ask is out, then "ok" / "old" /
    /// "missing" / "failed". The screen holds the four sentences; this
    /// says which of them ([`version::Probe`], whose words are git's).
    git_path_state: String,
    /// The version string that git printed, empty where it printed none.
    git_path_version: String,
    /// git's or the OS's own words for a probe that failed. Passed
    /// through: the reason a binary would not start is not ours to write.
    git_path_error: String,
    /// Where an empty box points: the git `PATH` resolves
    /// (`Hub::path_program`). Held so the offer can be worked out from
    /// **which binary** each answer names, whatever the spelling — a box
    /// emptied, and a box holding the very path this run spawns, are both
    /// the git already running.
    git_path_on_path: String,
    /// The box is holding a git this run is not on, and that git
    /// answered — the one state the chapter grows a button and a warning
    /// for.
    ///
    /// **One spelling of the rule, read by everything.** The button's
    /// shape, the warning beside it and the press's own guard are the
    /// same three terms; written twice, a screen could offer a restart
    /// the press then refuses, or refuse one the screen was offering.
    git_path_offers_restart: bool,
    /// The last answer's word on whether the box names the git already
    /// running (`AppMsg::GitPathProbed::names_the_run`). Only read behind
    /// a state the answer set: a box rewritten since is "checking" again.
    git_path_names_the_run: bool,
    /// A git that answers has been chosen and it is not the one this run
    /// is on, so the window is to close and come back. Read by the window,
    /// which is where the one road out lives (`Main.qml`).
    restart_wanted: bool,
    /// Where the git this run spawns actually is. **What the box shows
    /// behind an empty value**: a path is somewhere a reader can go and
    /// look at. Read once — the binary is settled for the length of the
    /// run (`Hub::git_program`).
    git_path_in_use: String,
    /// Assigned pictures, one record each: address, name, URL
    /// (`Assignment`). The settings list is the only reader, and it is a
    /// handful of rows.
    avatars: Assignments,
    /// How the last assignment was refused, as the card writes its line
    /// from it: which refusal it was, the numbers that sentence takes,
    /// and the operating system's own words where the failure is one it
    /// made. All three empty means it worked
    /// (`avatar::AvatarRefusal`, `Words.avatarFailure`).
    avatar_error_kind: String,
    avatar_error_facts: Vec<String>,
    avatar_error_said: String,
    /// The patterns the picker offers, built from the kinds the store
    /// accepts so the dialog and the store cannot drift apart. Patterns
    /// only: the word in front of them is the dialog's, and words live in
    /// `qsTr()` on the QML side.
    avatar_patterns: String,
    /// The worktree this binary was built in, empty for the primary
    /// checkout — see [`build_tree`].
    build_tree: String,
    /// Another process is already using the files this one would have
    /// used, so this window is only here to say so and be dismissed.
    already_running: bool,
    /// The directory that other process is holding. Two builds are told
    /// apart by nothing else on screen, and the taskbar's launch entry
    /// gives no clue which one it started.
    held_elsewhere: String,
    /// Whether the command log is up. The one part of the layout that is
    /// held for the length of the run alone: a tab handing over to the
    /// next one reads it back from here the way it reads the pane widths
    /// (`PageLayout`), but a new window comes up with the log down — it
    /// is where a command's answer is read, and the window keeps its own
    /// shape.
    commands_shown: bool,
    check_feed: Arc<Feed<AppMsg>>,
}
