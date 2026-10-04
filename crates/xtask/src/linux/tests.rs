//! `linux`'s own tests, in a file of their own (structure.md §分割).

use super::*;

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

#[test]
fn the_small_image_takes_everything_that_holds_no_qt() {
    assert_eq!(
        stage_for(&words("test -p platitude-core --test it")),
        "core"
    );
    assert_eq!(stage_for(&words("test -p xtask -p platitude-core")), "core");
    assert_eq!(stage_for(&words("fmt --all --check")), "core");
    assert_eq!(stage_for(&[]), "core");
}

#[test]
fn anything_that_can_reach_the_app_takes_the_other_one() {
    assert_eq!(stage_for(&words("test")), "app");
    assert_eq!(stage_for(&words("build --workspace")), "app");
    assert_eq!(stage_for(&words("clippy -p platitude-app")), "app");
    assert_eq!(stage_for(&words("verify-ui commit --preset basic")), "app");
    // qmltestrunner and the QtTest QML module both ship with Qt.
    assert_eq!(stage_for(&words("qmltest")), "app");
}

#[test]
fn an_xtask_verb_is_run_through_the_task_runner() {
    assert_eq!(
        command_line(&words("verify-ui commit --preset basic"), None),
        words("cargo xtask verify-ui commit --preset basic")
    );
    assert_eq!(
        command_line(&words("demo-repo --preset basic"), None),
        words("cargo xtask demo-repo --preset basic")
    );
}

/// /work is the host's own checkout, so a cargo in there that resolves
/// can rewrite the host's `Cargo.lock`.
#[test]
fn a_cargo_that_resolves_is_locked_wherever_it_was_typed() {
    assert_eq!(
        command_line(&words("test -p platitude-core"), None),
        words("cargo test --locked -p platitude-core")
    );
    assert_eq!(
        command_line(&words("clippy -p xtask --all-targets"), None),
        words("cargo clippy --locked -p xtask --all-targets")
    );
    assert_eq!(
        command_line(&words("tree"), None),
        words("cargo tree --locked")
    );
    // Cargo's own one-letter aliases are the same verbs.
    assert_eq!(
        command_line(&words("t -p platitude-core"), None),
        words("cargo t --locked -p platitude-core")
    );
    assert_eq!(stage_for(&words("t")), "app");
    // The gate spells its own, and one line does not carry it twice.
    assert_eq!(
        command_line(&words("test --locked -p xtask --lib"), None),
        words("cargo test --locked -p xtask --lib")
    );
}

/// The default is locked, so a verb nobody listed carries it too —
/// `fetch` writes a lock file of its own where there is none.
#[test]
fn a_verb_nobody_listed_is_locked_all_the_same() {
    assert_eq!(
        command_line(&words("fetch"), None),
        words("cargo fetch --locked")
    );
    for verb in ["vendor", "package", "rustc", "fix", "publish", "install"] {
        assert_eq!(
            command_line(&words(verb), None),
            words(&format!("cargo {verb} --locked")),
            "{verb}"
        );
    }
    // A third-party subcommand is locked as well: one that will not take
    // the flag says so, which is the loud half of being wrong.
    assert_eq!(
        command_line(&words("deny check"), None),
        words("cargo deny --locked check")
    );
    // A line that is all options has no subcommand to spell it after,
    // and resolves nothing either.
    assert_eq!(
        command_line(&words("--version"), None),
        words("cargo --version")
    );
    assert_eq!(command_line(&words("--list"), None), words("cargo --list"));
}

/// Cargo takes its own options ahead of the subcommand, so a line that
/// reads the first word as the verb hands the container an unlocked cargo.
#[test]
fn cargos_own_options_come_before_the_subcommand() {
    assert_eq!(
        command_line(&words("--offline fetch"), None),
        words("cargo --offline fetch --locked")
    );
    assert_eq!(
        command_line(&words("-v test -p platitude-core"), None),
        words("cargo -v test --locked -p platitude-core")
    );
    // Value-bearing options in both spellings: the value is a word with
    // no dash on it, and is not the subcommand.
    assert_eq!(
        command_line(&words("--config net.retry=2 test"), None),
        words("cargo --config net.retry=2 test --locked")
    );
    assert_eq!(
        command_line(&words("--config=net.retry=2 test"), None),
        words("cargo --config=net.retry=2 test --locked")
    );
    assert_eq!(
        command_line(&words("--color always -Z unstable-options build"), None),
        words("cargo --color always -Z unstable-options build --locked")
    );
    // The attached spellings are one word each, and the word after them
    // is the subcommand.
    assert_eq!(
        command_line(&words("--color=always -Zscript build"), None),
        words("cargo --color=always -Zscript build --locked")
    );
    // The exceptions are the exceptions wherever the verb stands.
    assert_eq!(
        command_line(&words("--offline fmt --check"), None),
        words("cargo --offline fmt --check")
    );
    // Already locked, in either spelling — --frozen is --locked and
    // --offline in one word.
    assert_eq!(
        command_line(&words("--locked fetch"), None),
        words("cargo --locked fetch")
    );
    assert_eq!(
        command_line(&words("--frozen fetch"), None),
        words("cargo --frozen fetch")
    );
    // The image is chosen off the same word.
    assert_eq!(
        stage_for(&words("--offline test -p platitude-core")),
        "core"
    );
    assert_eq!(stage_for(&words("--offline test")), "app");
    assert_eq!(stage_for(&words("--config net.retry=2 build")), "app");
    // An xtask verb behind cargo's options is still ours.
    assert_eq!(
        command_line(&words("--offline verify-ui commit"), None),
        words("cargo --offline xtask verify-ui commit")
    );
    assert_eq!(stage_for(&words("--offline verify-ui commit")), "app");
}

/// Left alone on purpose: a verb that resolves nothing, and the verbs
/// whose whole purpose is to move the lock — `--locked` would refuse
/// the second kind outright.
#[test]
fn the_verbs_that_do_not_resolve_are_handed_over_as_typed() {
    assert_eq!(
        command_line(&words("fmt --all --check"), None),
        words("cargo fmt --all --check")
    );
    for line in [
        "update -p tracing",
        "generate-lockfile",
        "add thiserror",
        "remove thiserror",
        "clean",
    ] {
        assert_eq!(
            command_line(&words(line), None),
            words(&format!("cargo {line}"))
        );
    }
    // An xtask verb needs none of this: the alias carries it
    // (.cargo/config.toml).
    assert!(
        !command_line(&words("verify-ui commit"), None).contains(&"--locked".to_string()),
        "the alias already spells it"
    );
}

/// Past `--` the words are the program's.
#[test]
fn a_locked_beyond_the_separator_is_not_this_lines_own() {
    assert_eq!(
        command_line(&words("run -p xtask -- structure --locked"), None),
        words("cargo run --locked -p xtask -- structure --locked")
    );
}

/// No `cargo xtask`, no alias, no resolve (`linux::runner`).
#[test]
fn a_prepared_copy_starts_the_verb_and_no_cargo_does() {
    let copy = "/work/target/gate-runner/xtask-1758-40".to_string();
    assert_eq!(
        command_line(
            &words("verify-ui commit --preset basic"),
            Some(copy.clone())
        ),
        vec![
            copy.clone(),
            "verify-ui".into(),
            "commit".into(),
            "--preset".into(),
            "basic".into()
        ]
    );
    let line = command_line(&words("verify-ui wip --no-build"), Some(copy));
    assert!(!line.iter().any(|word| word == "cargo"), "{line:?}");
}

/// A copy that is not there is a preparation that did not happen: the
/// script says so and stops, and holds no cargo on any road, the red one
/// included.
#[test]
fn a_line_that_names_a_copy_stops_when_the_copy_is_not_there() {
    let copy = "/work/target/gate-runner/xtask-1758-40";
    let line = watched_from_inside(&[copy.to_string(), "verify-ui".into()], Some(copy));
    let script = &line[2];
    assert!(
        !script.contains("cargo --version") && !script.contains("sha256sum"),
        "nothing in there resolves, so nothing looks at what a resolve reads: {script}"
    );
    assert!(script.contains("pgg-probe ran"), "a red line is still said");
    assert!(
        script.starts_with(&format!("if [ ! -x {copy} ]; then")),
        "{script}"
    );
    assert!(script.contains("exit 127"), "{script}");
    assert!(
        script.contains("nothing here falls back to cargo"),
        "the failure names what it refuses to do: {script}"
    );
    // The guard is the copy's alone: a cargo line is the line it was.
    let cargo = watched_from_inside(&words("cargo test --locked -p platitude-core"), None);
    assert!(cargo[2].starts_with("look()"), "{}", cargo[2]);
}

/// The container is `--rm`: what it did not say while it ran is gone.
/// The look is taken before the command and printed only if it fails.
#[test]
fn the_command_runs_bracketed_by_a_look_at_what_a_resolve_reads() {
    let line = watched_from_inside(&words("cargo xtask verify-ui commit"), None);
    assert_eq!(line[..2], words("sh -c")[..]);
    // The command arrives as arguments.
    assert_eq!(line[3], IMAGE);
    assert_eq!(line[4..], words("cargo xtask verify-ui commit")[..]);
    let script = &line[2];
    for file in READ_TO_RESOLVE {
        assert!(script.contains(&format!("{WORK}/{file}")), "{file}");
    }
    // Every member's manifest, as a glob the shell in there expands —
    // a member added to the workspace is not one this forgets.
    assert!(
        script.contains("/work/crates/*/Cargo.toml"),
        "the members are globbed: {script}"
    );
    assert!(script.contains("before=$(look before)"), "{script}");
    assert!(script.contains("exit \"$code\""), "{script}");
}

/// A reap would be an engine's `rm` with no ceiling over it, and nothing
/// needs one: an interrupted container cannot reach this checkout's
/// copies (`linux::runner::SCRIPT`), and its cargo lock is waited out
/// under the step's ceiling. The gate's own container is named and
/// removed under a ceiling in `container` (`remove_container`).
#[test]
fn no_container_started_here_is_named_or_reaped() {
    assert!(
        !args_of(&carried()).iter().any(|arg| arg == "--name"),
        "a named container is one something has to come back and remove"
    );
    let source = include_str!("../linux.rs");
    for reaping in [
        "docker\", \"rm",
        "\"rm\", \"--force\"",
        "reap_an_interrupted",
    ] {
        assert!(
            !source.contains(reaping),
            "linux.rs removes a container ({reaping}) outside any ceiling"
        );
    }
}

/// `--container` is a prepared copy's road and a marked one: a line
/// with either flag and not its partner, or a container without a
/// copy, is one nobody spelled on purpose — and so is `--rebuild`
/// beside a container already running from an image.
#[test]
fn a_container_line_needs_a_copy_and_a_mark_and_nothing_to_rebuild() {
    assert!(options(&words("--container c verify-ui wip")).is_err());
    assert!(options(&words("--runner r --container c verify-ui wip")).is_err());
    assert!(options(&words("--runner r --step m verify-ui wip")).is_err());
    assert!(
        options(&words(
            "--rebuild --runner r --container c --step m verify-ui wip"
        ))
        .is_err()
    );
    let all = options(&words("--runner r --container c --step m verify-ui wip"))
        .expect("the copy, the container and the mark");
    assert_eq!(all.copy.as_deref(), Some("r"));
    assert_eq!(all.container.as_deref(), Some("c"));
    assert_eq!(all.step.as_deref(), Some("m"));
    assert_eq!(all.at, 6, "the command begins after the options");
    // Without a container the line is what it was.
    let copy = options(&words("--runner r verify-ui wip")).expect("a copy alone");
    assert!(copy.container.is_none() && copy.step.is_none());
    // `stop` is the one line that names a container and no copy: it is
    // about what is already in there.
    let stop = options(&words("--container c --step m stop")).expect("a stop needs no copy");
    assert_eq!(stop.container.as_deref(), Some("c"));
    assert_eq!(stop.step.as_deref(), Some("m"));
    assert!(
        options(&words("--container c stop")).is_err(),
        "a stop with no mark"
    );
}

#[test]
fn volume_names_survive_a_windows_path() {
    assert_eq!(
        volume(
            Path::new("C:\\Users\\x\\IdeaProjects\\platitude-gg"),
            "target"
        ),
        "pgg-linux-target-platitude-gg"
    );
}

/// A `git worktree list --porcelain` listing: the primary checkout, a
/// seat, the measurement's detached rig and a task's fresh worktree. What
/// makes a name alive is that git names the tree, not where it stands.
const LISTING: &str = "\
worktree C:/Users/x/IdeaProjects/platitude-gg
HEAD 1111111111111111111111111111111111111111
branch refs/heads/main

worktree C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/a
HEAD 2222222222222222222222222222222222222222
branch refs/heads/worktree-a

worktree C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/rig
HEAD 3333333333333333333333333333333333333333
detached

worktree C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/wizardly-ellis-7b042a
HEAD 4444444444444444444444444444444444444444
branch refs/heads/claude/competent-benz-7dd4b1
";

#[test]
fn every_tree_git_names_is_a_live_name() {
    let names = checkout_names(&crate::seats::worktree_blocks(LISTING));
    assert_eq!(
        names.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["a", "platitude-gg", "rig", "wizardly-ellis-7b042a"]
    );
}

/// Whatever kind of tree git still names, its volumes stay. The registry
/// volume is the machine's, and anything not this project's is left too.
#[test]
fn only_the_volumes_of_a_checkout_that_is_gone_are_orphans() {
    let listed = "\
pgg-linux-target-platitude-gg
pgg-linux-demo-platitude-gg
pgg-linux-target-a
pgg-linux-demo-a
pgg-linux-target-rig
pgg-linux-demo-wizardly-ellis-7b042a
pgg-linux-target-b
pgg-linux-demo-tooltip
pgg-linux-registry
some-other-project-target-a
";
    assert_eq!(
        orphan_volumes(
            listed,
            &checkout_names(&crate::seats::worktree_blocks(LISTING))
        ),
        vec!["pgg-linux-target-b", "pgg-linux-demo-tooltip"]
    );
}

/// Two generations at once: the tree that is building names one tag and
/// a seat behind it names the other, and a keep set of only the
/// builder's would take the image from a checkout still using it.
#[test]
fn a_tag_a_checkout_that_is_behind_still_names_is_kept() {
    let listed = "\
pgg-linux:app.3306fab788288e20
pgg-linux:app.d3f3924bdfb2ec3c
pgg-linux:app.0000000000000000
pgg-linux:core.f12dad52a4b651e6
pgg-linux:bare.3306fab788288e20
";
    let keep: BTreeSet<String> = [
        "pgg-linux:app.3306fab788288e20",
        "pgg-linux:app.d3f3924bdfb2ec3c",
        "pgg-linux:core.f12dad52a4b651e6",
        "pgg-linux:bare.3306fab788288e20",
    ]
    .iter()
    .map(|tag| (*tag).to_string())
    .collect();
    assert_eq!(
        stale_images(listed, "app", &keep),
        vec!["pgg-linux:app.0000000000000000"]
    );
    // A stage whose tag nobody names is a stage this leaves alone
    // entirely, and the prefix is what keeps `app` out of `bare`'s
    // answer: the two carry the same fingerprint.
    assert!(stale_images(listed, "core", &keep).is_empty());
    assert!(stale_images(listed, "bare", &keep).is_empty());
}

/// A tree with a pin file and a tree from before it name their images by
/// two formulas, in two shapes: a dashed tag a seat that is behind still
/// answers to stays, and one no tree's runner arrives at goes like any
/// other.
#[test]
fn a_tree_from_before_the_pin_file_keeps_the_dashed_tag_its_runner_names() {
    let yard = crate::yard::Yard::new("linux-tags");
    let stand = |name: &str, files: &[(&str, &str)]| {
        let tree = yard.join(name);
        for (path, text) in files {
            let at = tree.join(path);
            std::fs::create_dir_all(at.parent().expect("a file under the tree")).expect("dir");
            std::fs::write(at, text).expect("file");
        }
        tree
    };
    let shared = [
        ("ci/linux/Dockerfile", "FROM ubuntu:24.04 AS core\n"),
        ("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
    ];
    let workflow = crate::qt::former_pin();
    let ahead = stand(
        "ahead",
        &[
            shared[0],
            shared[1],
            (crate::qt::PIN, "6.12.0\n"),
            // A workflow that pins nothing, whatever else it says.
            (workflow.as_str(), "jobs:\n"),
        ],
    );
    let behind = stand(
        "behind",
        &[
            shared[0],
            shared[1],
            (workflow.as_str(), "env:\n  QT_VERSION: \"6.12.0\"\n"),
        ],
    );

    // Each by the formula its own runner has, as the keep set asks them.
    let ours = tag_named_by(&ahead, "app").expect("the tree with a pin file");
    let theirs = tag_named_by(&behind, "app").expect("the tree from before it");
    // FNV-1a over the files' bytes, worked out apart from this crate: the
    // former tag has to be the one a runner from before spells, to the
    // digit, or its images are taken from under it.
    assert_eq!(ours, "pgg-linux:app.76d8208b83d28ac8");
    assert_eq!(theirs, "pgg-linux:app-5797d9df901044ed");
    assert_eq!(
        tag_named_by(&behind, "core").expect("core"),
        "pgg-linux:core-ae6f466b6a1deea3"
    );
    // The keep set asks every tree, each its own way: both tags stand in
    // it, and a tree that cannot say is named rather than skipped.
    let (kept, silent) = tags_named_by_all(&[&ahead, &behind]);
    assert!(silent.is_empty(), "{silent:?}");
    for tag in [&ours, &theirs] {
        assert!(kept.contains(tag), "{tag} is not kept: {kept:?}");
    }
    let nowhere = yard.join("nowhere");
    let (_, silent) = tags_named_by_all(&[&ahead, &nowhere]);
    assert_eq!(silent.len(), 1, "{silent:?}");
    assert_eq!(silent[0].0, nowhere.as_path());
    // Not a line of the workflow: an edit there names the same image.
    std::fs::write(ahead.join(&workflow), "jobs:\n  test:\n").expect("an edited workflow");
    assert_eq!(image_tag(&ahead, "app").expect("still"), ours);
    // The pin is: a new Qt is a new image.
    std::fs::write(ahead.join(crate::qt::PIN), "6.13.0\n").expect("a newer pin");
    assert_ne!(image_tag(&ahead, "app").expect("moved"), ours);
    // Core is built with no Qt, in either formula.
    assert_eq!(
        image_tag(&ahead, "core").expect("core").replace('.', "-"),
        former_image_tag(&behind, "core").expect("core")
    );

    let listed = format!("{ours}\n{theirs}\npgg-linux:app-0000000000000000\n");
    let keep: BTreeSet<String> = [ours.clone(), theirs.clone()].into();
    assert_eq!(
        stale_images(&listed, "app", &keep),
        vec!["pgg-linux:app-0000000000000000"]
    );
    // Once no tree is behind, nothing names the dashed tag.
    let keep: BTreeSet<String> = [ours.clone()].into();
    assert_eq!(
        stale_images(&listed, "app", &keep),
        vec![theirs.as_str(), "pgg-linux:app-0000000000000000"]
    );
}

/// A stage added to the Dockerfile and forgotten here is one whose images
/// no checkout is ever seen to need.
#[test]
fn the_stages_are_the_ones_the_dockerfile_builds() {
    let path = crate::tree::workspace_root()
        .join("ci")
        .join("linux")
        .join("Dockerfile");
    let text = std::fs::read_to_string(&path).expect("ci/linux/Dockerfile");
    // The name `--target` takes, which is the word after `AS` and not
    // simply the last one: a `FROM <image>` with no name at all would
    // otherwise be read as a stage called after the image.
    let built: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("FROM "))
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            words.find(|word| word.eq_ignore_ascii_case("AS"))?;
            words.next()
        })
        .collect();
    assert_eq!(built, STAGES);
}

/// The registry volume is shared by every container on the machine, so
/// cargo's locks on it (taken in CARGO_HOME) must be too: the image links
/// both into the registry the volume is mounted over. Otherwise two
/// containers unpack the same new crate at once, the second failing on
/// its `.cargo-ok`.
#[test]
fn the_package_cache_locks_live_in_the_shared_registry() {
    let path = crate::tree::workspace_root()
        .join("ci")
        .join("linux")
        .join("Dockerfile");
    let text = std::fs::read_to_string(&path).expect("ci/linux/Dockerfile");
    let home = text
        .split("CARGO_HOME=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("the image sets CARGO_HOME");
    assert_eq!(REGISTRY_MOUNT, format!("{home}/registry"));
    for lock in [".package-cache", ".package-cache-mutate"] {
        assert!(
            text.contains(&format!("ln -s registry/{lock} {home}/{lock}")),
            "the image links {lock} into the registry"
        );
    }
}

#[test]
fn mount_paths_are_forward_slashed() {
    assert_eq!(
        mount_path(Path::new("C:\\Users\\x\\platitude-gg")),
        "C:/Users/x/platitude-gg"
    );
}

#[test]
fn the_qt_version_comes_from_the_trees_pin() {
    let root = crate::tree::workspace_root();
    let version = qt_version(&root).expect("the pin file at the tree's root");
    assert!(
        version.split('.').all(|part| part.parse::<u32>().is_ok()),
        "{version:?} does not look like a version"
    );
}

/// Every container is under the launcher's announcement and ticket, and
/// takes neither of its own — why both, at `carried`.
#[test]
fn a_container_is_started_carrying_both_marks() {
    assert_eq!(
        args_of(&carried()),
        vec![
            "run".to_string(),
            "--rm".to_string(),
            "--env".to_string(),
            format!("{}=1", crate::still::UNDER),
            "--env".to_string(),
            format!("{}=1", crate::budget::HELD),
        ]
    );
}

/// Only a listing the engine answered sends a line on to build. An
/// engine that would not say — whatever it said instead — stops the line
/// with the engine's account in it.
#[test]
fn an_engine_that_would_not_say_what_it_holds_is_never_asked_to_build() {
    let tag = "pgg-linux:core-0123456789abcdef";
    assert_eq!(built_off(tag, engine::Presence::There), Ok(true));
    assert_eq!(
        built_off(tag, engine::Presence::Gone),
        Ok(false),
        "the one answer that builds"
    );
    let account = "`wslc images` exited Some(1) after 0.0s and said: ERROR_INVALID_STATE";
    let stopped = built_off(tag, engine::Presence::Unknown(account.to_string()))
        .expect_err("an engine that would not answer was read as holding no image");
    assert!(stopped.contains(tag), "{stopped}");
    assert!(stopped.contains(account), "{stopped}");
    assert!(stopped.contains("built nothing"), "{stopped}");
}

/// The arguments as the process would see them.
pub(super) fn args_of(cmd: &Command) -> Vec<String> {
    cmd.get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}
