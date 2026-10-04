//! `cargo xtask replay` — verify-ui lines run again on the machine this
//! stands on, one after another, with every run's pictures and log gathered
//! under one directory. What CI's macOS job runs
//! (.github/workflows/shots.yml): the one OS no desk here has, where the
//! pictures have to travel to be looked at.
//!
//! Each line is one `verify-ui`, judged as a typed run is. Nothing here
//! writes the census or the board: the census is the desk's that recorded
//! it, and the board is read where the pictures were asked for.
//!
//! The lines build the release until one has gone green and the rest reuse
//! it, as a gate's block of verbs does (`gate::sides::verbs`). A red line
//! stops none of the others — unless what went red is the build they all
//! read.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use crate::budget::{Ask, Pool, Rank};
use crate::check::Stepped;
use crate::command::{self, Permission, Where};
use crate::gate::{Census, Tiers};
use crate::locks::Locked;

pub(crate) static REPLAY: command::Command = command::Command {
    id: "replay.lines",
    call: "replay --tier <linux|merge|all>",
    purpose: "census lines run again on this machine, every run's pictures under one directory",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&REPLAY];

/// Which census lines a run takes besides the ones it is handed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    /// The tier table's `linux` rows: judged on a size, a cut or a fold
    /// the fonts decide, so a machine with other fonts can answer
    /// differently.
    Linux,
    /// Every line a gate owes before a merge.
    Merge,
    /// Every line the full gate owes.
    All,
}

struct Options {
    tier: Option<Tier>,
    lines: Vec<String>,
    out: Option<PathBuf>,
    build: bool,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        tier: None,
        lines: Vec::new(),
        out: None,
        build: true,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--tier" => {
                opts.tier = match it.next().map(String::as_str) {
                    Some("linux") => Some(Tier::Linux),
                    Some("merge") => Some(Tier::Merge),
                    Some("all") => Some(Tier::All),
                    // Sayable, so a caller that always passes the flag
                    // (the workflow) can ask for the typed lines alone.
                    Some("none") => None,
                    other => {
                        return Err(format!(
                            "--tier takes linux, merge, all or none; got {other:?}"
                        ));
                    }
                };
            }
            "--line" => opts.lines.extend(typed_lines(
                it.next().ok_or("--line needs a verify-ui line")?,
            )),
            "--out" => {
                opts.out = Some(PathBuf::from(it.next().ok_or("--out needs a directory")?));
            }
            "--no-build" => opts.build = false,
            other => {
                return Err(format!(
                    "unknown option {other:?} (replay takes --tier, --line, --out and --no-build)"
                ));
            }
        }
    }
    Ok(opts)
}

/// The lines one `--line` value holds: `;` or a line end between them, so
/// a single field of a form carries several. No census line spells a `;`
/// (an argument with a space is no census line either — `verify::options`).
fn typed_lines(value: &str) -> Vec<String> {
    value
        .split([';', '\n', '\r'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// The lines of one run, in the order they run: the tier's in the
/// census's own order, then the typed ones. A line named twice runs once.
fn chosen(tier: Option<Tier>, typed: &[String], census: &Census, tiers: &Tiers) -> Vec<String> {
    let mut lines: Vec<String> = census
        .lines
        .keys()
        .filter(|line| match tier {
            None => false,
            Some(Tier::Linux) => tiers.on_linux(line, false),
            Some(Tier::Merge) => tiers.owed(line, false),
            Some(Tier::All) => tiers.owed(line, true),
        })
        .cloned()
        .collect();
    for line in typed {
        if !lines.contains(line) {
            lines.push(line.clone());
        }
    }
    lines
}

/// This tree's own place for a run nobody named a directory for.
const OWN_OUT: [&str; 2] = ["target", "replay"];

/// The extension of the lock file beside this tree's own directory.
const LOCK: &str = "lock";

/// Where a run's pictures and logs go.
struct Out {
    dir: PathBuf,
    /// Held for as long as the run stands over this tree's own directory;
    /// a directory the caller named has none.
    _sole: Option<Locked>,
}

/// Where the run's pictures and logs go.
///
/// Left unsaid it is this tree's own directory, taken down and stood up
/// again by every run — one run at a time ([`sole_over`]), so nothing else
/// writes there and one run's pictures are never read as the next one's. A
/// directory the caller names is theirs: used only while it holds nothing,
/// and never emptied.
fn out_dir(root: &Path, named: Option<&Path>) -> Result<Out, String> {
    let Some(named) = named else {
        let own = OWN_OUT
            .iter()
            .fold(root.to_path_buf(), |at, part| at.join(part));
        let sole = sole_over(&own)?;
        match std::fs::remove_dir_all(&own) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("could not clear {}: {e}", own.display())),
        }
        std::fs::create_dir_all(&own).map_err(|e| format!("{}: {e}", own.display()))?;
        return Ok(Out {
            dir: own,
            _sole: Some(sole),
        });
    };
    // Pinned where it was typed: every run starts in the tree's root
    // (`check::run_step`), where a relative path would name another place.
    let out = std::path::absolute(named).map_err(|e| format!("{}: {e}", named.display()))?;
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let holds_something = std::fs::read_dir(&out)
        .map_err(|e| format!("{}: {e}", out.display()))?
        .next()
        .is_some();
    if holds_something {
        return Err(format!(
            "{} holds something already: name a directory of this run's own (replay never \
             empties one it was handed)",
            out.display()
        ));
    }
    Ok(Out {
        dir: out,
        _sole: None,
    })
}

/// The one replay over this tree's own directory: a second one would take
/// the first's pictures down under it. The lock file stands beside the
/// directory and is never removed, as a tree's gate lock is (`lanes::Sole`).
fn sole_over(own: &Path) -> Result<Locked, String> {
    let at = own.with_extension(LOCK);
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&at)
        .map_err(|e| format!("{}: {e}", at.display()))?;
    match lock.try_lock() {
        Ok(()) => Ok(Locked::new(lock)),
        Err(TryLockError::WouldBlock) => Err(format!(
            "another replay is writing {} — wait for it, or name a directory of this run's own \
             with --out",
            own.display()
        )),
        Err(TryLockError::Error(e)) => Err(format!("{}: {e}", at.display())),
    }
}

/// How much of a line a file name carries.
const SLUG_LEN: usize = 60;

/// A line as a file name on all three systems: every character a shell or
/// a file system reads is a dash.
fn slug(line: &str) -> String {
    let mut name: String = line
        .chars()
        .map(|c| match c {
            c if c.is_ascii_alphanumeric() => c,
            '.' | '_' | '-' => c,
            _ => '-',
        })
        .collect();
    // ASCII by now, so any byte index is a character's.
    name.truncate(SLUG_LEN);
    name
}

/// One line's run as a step: this runner itself, so nothing waits on
/// cargo between two lines. `--no-board` rides in with the line
/// (`verify::suite_words`), and the census is left alone.
fn step_for(runner: &Path, line: &str, shots: &Path, build: bool) -> Vec<String> {
    let mut step = vec![runner.display().to_string(), "verify-ui".to_string()];
    step.extend(crate::verify::suite_words(line));
    step.push("--no-census".to_string());
    step.push("--shot-dir".to_string());
    step.push(shots.display().to_string());
    if !build {
        step.push("--no-build".to_string());
    }
    step
}

/// One line as it ended.
struct Row {
    line: String,
    passed: bool,
    /// Red, and what went red is the build every line reads.
    unbuilt: bool,
    secs: u64,
    /// The run's name under the output directory: its pictures' directory,
    /// and its log with `.log` after it.
    name: String,
}

/// Whether the line after `rows` builds. Every line does until one has
/// gone green: a red one may have stopped before its build (a fixture that
/// would not stand), and the next would be handed `--no-build` against no
/// release at all, or one older than the sources. `build` false is the
/// caller's word that the release stands.
fn builds_next(rows: &[Row], build: bool) -> bool {
    build && !rows.iter().any(|row| row.passed)
}

/// Whether a red line's log says the app itself did not build — the two
/// sentences `gate::runner::app_did_not_build` reads, for the same reason:
/// every line after it would run the same build into the same error.
fn build_went_red(log: &str) -> bool {
    log.contains("could not compile") || log.contains("cargo build --release failed")
}

/// What ran, as the page a workflow hands its run summary: the red lines
/// first, each with where its pictures and log are, then the lines that
/// never ran and why.
fn summary(rows: &[Row], not_run: Option<(&[String], &str)>, on: &str) -> String {
    let red = rows.iter().filter(|row| !row.passed).count();
    let mut page = format!("### replay: {} line(s) run, {red} red", rows.len());
    if let Some((left, _)) = not_run {
        page.push_str(&format!(", {} not run", left.len()));
    }
    page.push_str(&format!(
        "\n\n{on}\n\n| | line | seconds | pictures and log |\n|:--|:--|--:|:--|\n"
    ));
    let reds = rows.iter().filter(|row| !row.passed);
    for row in reds.chain(rows.iter().filter(|row| row.passed)) {
        page.push_str(&format!(
            "| {} | `{}` | {} | `{name}/`, `{name}.log` |\n",
            if row.passed { "ok" } else { "FAIL" },
            // A bar would end the cell (`publish-new-go origin|<url>`).
            row.line.replace('|', "\\|"),
            row.secs,
            name = row.name,
        ));
    }
    if let Some((left, why)) = not_run {
        page.push_str(&format!("\nNot run — {why}:\n\n"));
        for line in left {
            page.push_str(&format!("- `{line}`\n"));
        }
    }
    page
}

/// What the run stands on, shared by every line of it.
struct Ground<'a> {
    root: &'a Path,
    out: &'a Path,
    runner: &'a Path,
    pool: &'a Pool,
    seat: &'a str,
}

/// Runs one line and says how it ended. `Err` is a line that never ran:
/// no room on the machine could be had for it.
fn run_line(ground: &Ground<'_>, index: usize, line: &str, build: bool) -> Result<Row, String> {
    let name = format!("{index:03}-{}", slug(line));
    let log = ground.out.join(format!("{name}.log"));
    let step = step_for(ground.runner, line, &ground.out.join(&name), build);
    let room = ground.pool.admit_once_the_machine_is_free(&Ask {
        weight: crate::budget::weight_of(&step, !build),
        rank: Rank::Normal,
        seat: ground.seat,
        what: &format!("replay {line}"),
    })?;
    // waits(measured): the line's wall clock, said on its row and judged by nothing
    let at = std::time::Instant::now();
    let outcome = crate::check::run_step(ground.root, &step, &log, &room, &|| false);
    drop(room);
    let secs = at.elapsed().as_secs();
    let passed = matches!(outcome, Ok(Stepped::Exited(true)));
    let mut unbuilt = false;
    if passed {
        println!("ok   {line} ({secs}s) — {name}/");
    } else {
        println!("FAIL {line} ({secs}s) — {name}/, {name}.log");
        if let Err(why) = &outcome {
            println!("     {why}");
        }
        // The whole log, lossily, as `check` reads a step's: the sentence
        // that says the build went red can stand above the tail, and one
        // localized byte in a linker line would otherwise blank all of it.
        let said = String::from_utf8_lossy(&std::fs::read(&log).unwrap_or_default()).into_owned();
        unbuilt = build && build_went_red(&said);
        for said in crate::check::tail_of(&said).lines() {
            println!("     {said}");
        }
    }
    Ok(Row {
        line: line.to_string(),
        passed,
        unbuilt,
        secs,
        name,
    })
}

/// A run as it ended: the lines that ran, and — where it stopped short —
/// the lines it never reached, with why.
struct Walked<'a> {
    rows: Vec<Row>,
    not_run: Option<(&'a [String], String)>,
}

/// Runs the lines in order through `run_line`, which is told whether its
/// line builds ([`builds_next`]), and stops where no later line could
/// follow: at a line whose red is the build's, which is still a line that
/// ran, or at one that never ran at all (`Err`), which is left with the
/// rest.
fn walk<'a>(
    lines: &'a [String],
    build: bool,
    run_line: &mut dyn FnMut(usize, &str, bool) -> Result<Row, String>,
) -> Walked<'a> {
    let mut rows: Vec<Row> = Vec::new();
    let mut stopped = None;
    for (index, line) in lines.iter().enumerate() {
        match run_line(index, line, builds_next(&rows, build)) {
            Ok(row) => {
                let unbuilt = row.unbuilt;
                rows.push(row);
                if unbuilt {
                    stopped = Some("the app did not build".to_string());
                    break;
                }
            }
            Err(why) => {
                stopped = Some(why);
                break;
            }
        }
    }
    let left = &lines[rows.len()..];
    Walked {
        rows,
        not_run: stopped.filter(|_| !left.is_empty()).map(|why| (left, why)),
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let root = crate::tree::workspace_root();
    // The tables are read only for a tier: a typed line runs as typed.
    let (census, tiers) = match opts.tier {
        Some(_) => (Census::load(&root)?, Tiers::load(&root)?),
        None => (Census::default(), Tiers::default()),
    };
    let lines = chosen(opts.tier, &opts.lines, &census, &tiers);
    if lines.is_empty() {
        return Err(
            "nothing to replay: --tier (linux, merge or all) and --line '<verify-ui line>' named \
             no line between them"
                .into(),
        );
    }
    let on = format!(
        "on {}, {}",
        std::env::consts::OS,
        crate::qt::this_tree()?.describe()
    );
    let out = out_dir(&root, opts.out.as_deref())?;
    let runner = std::env::current_exe().map_err(|e| format!("this runner's own path: {e}"))?;
    let pool = Pool::of(&root, crate::budget::default_jobs())?;
    let seat = crate::budget::seat_of(&root);
    let ground = Ground {
        root: &root,
        out: &out.dir,
        runner: &runner,
        pool: &pool,
        seat: &seat,
    };
    println!("replay: {} line(s) {on}", lines.len());
    println!("replay: pictures and logs under {}", out.dir.display());

    let Walked { rows, not_run } = walk(&lines, opts.build, &mut |index, line, build| {
        run_line(&ground, index, line, build)
    });
    let not_run = not_run.as_ref().map(|(left, why)| (*left, why.as_str()));

    let page_at = out.dir.join("summary.md");
    std::fs::write(&page_at, summary(&rows, not_run, &on))
        .map_err(|e| format!("{}: {e}", page_at.display()))?;
    let red: Vec<&str> = rows
        .iter()
        .filter(|row| !row.passed)
        .map(|row| row.line.as_str())
        .collect();
    let mut wrong = Vec::new();
    if !red.is_empty() {
        wrong.push(format!("{} line(s) red: {}", red.len(), red.join(" / ")));
    }
    if let Some((left, why)) = not_run {
        wrong.push(format!("{} line(s) not run: {why}", left.len()));
    }
    println!(
        "replay: {} line(s) run, {} red, {} not run — {}",
        rows.len(),
        red.len(),
        lines.len() - rows.len(),
        page_at.display()
    );
    if wrong.is_empty() {
        return Ok(());
    }
    Err(wrong.join("; "))
}

#[cfg(test)]
mod tests {
    use super::{
        Row, Tier, Walked, build_went_red, builds_next, chosen, out_dir, parse, slug, step_for,
        summary, typed_lines, walk,
    };
    use crate::gate::{Census, Tiers};
    use std::path::Path;

    fn words(line: &[&str]) -> Vec<String> {
        line.iter().map(|word| (*word).to_string()).collect()
    }

    fn row(line: &str, passed: bool, name: &str) -> Row {
        Row {
            line: line.to_string(),
            passed,
            unbuilt: false,
            secs: 3,
            name: name.to_string(),
        }
    }

    fn census(lines: &[&str]) -> Census {
        let mut census = Census::default();
        for line in lines {
            census
                .lines
                .insert((*line).to_string(), ["Main".to_string()].into());
        }
        census
    }

    const TABLE: &str = "linux\tbadges --preset conflict\t-\tcap= reads the band's measured floor\n\
        full\tfile-menu b.txt\tstash\tpicture: the menu standing\n";

    #[test]
    fn a_tier_takes_the_lines_the_table_gives_it_and_typed_lines_follow() {
        let held = census(&["badges --preset conflict", "file-menu b.txt", "stash"]);
        let tiers = Tiers::parse(TABLE);
        let of = |tier, typed: &[&str]| chosen(tier, &words(typed), &held, &tiers);
        assert_eq!(of(Some(Tier::Linux), &[]), ["badges --preset conflict"]);
        assert_eq!(
            of(Some(Tier::Merge), &[]),
            ["badges --preset conflict", "stash"]
        );
        assert_eq!(
            of(Some(Tier::All), &[]),
            ["badges --preset conflict", "file-menu b.txt", "stash"]
        );
        // After the tier's, in the order typed; one the tier already
        // holds runs once.
        assert_eq!(
            of(
                Some(Tier::Linux),
                &["nav-tip branch:0", "badges --preset conflict"]
            ),
            ["badges --preset conflict", "nav-tip branch:0"]
        );
        // No tier: the census is not asked at all.
        assert_eq!(of(None, &["band", "band"]), ["band"]);
        assert!(of(None, &[]).is_empty());
    }

    #[test]
    fn one_value_carries_several_lines() {
        assert_eq!(
            typed_lines("band; nav-tip branch:0\n  co-authors 4 --preset co-authors ;;"),
            [
                "band",
                "nav-tip branch:0",
                "co-authors 4 --preset co-authors"
            ]
        );
        // What a workflow hands over when nobody typed a line.
        assert!(typed_lines("").is_empty());
    }

    #[test]
    fn the_command_line_names_a_tier_lines_and_a_place() {
        let opts = parse(&words(&[
            "--tier",
            "linux",
            "--line",
            "band; amend",
            "--line",
            "stash",
            "--out",
            "shots",
            "--no-build",
        ]))
        .expect("every option at once");
        assert_eq!(opts.tier, Some(Tier::Linux));
        assert_eq!(opts.lines, ["band", "amend", "stash"]);
        assert_eq!(opts.out.as_deref(), Some(Path::new("shots")));
        assert!(!opts.build);

        let none = parse(&words(&["--tier", "none", "--line", ""])).expect("nothing chosen");
        assert_eq!(none.tier, None);
        assert!(none.lines.is_empty() && none.build);

        let Err(unknown) = parse(&words(&["--tier", "fonts"])) else {
            panic!("a tier the table does not have is refused where it is typed");
        };
        assert!(unknown.contains("linux, merge, all or none"), "{unknown}");
        let Err(stray) = parse(&words(&["band"])) else {
            panic!("a line typed bare would run nothing and say nothing");
        };
        assert!(stray.contains("--line"), "{stray}");
    }

    #[test]
    fn a_line_is_a_file_name_on_every_system() {
        assert_eq!(slug("band"), "band");
        assert_eq!(
            slug("publish-new-go origin|file:///C:/x.git --preset noremote"),
            "publish-new-go-origin-file----C--x.git---preset-noremote"
        );
        assert_eq!(
            slug("nav-open head::1 --preset nested"),
            "nav-open-head--1---preset-nested"
        );
        assert_eq!(slug(&"é".repeat(200)).len(), super::SLUG_LEN);
    }

    /// A line that builds, and one told not to; neither writes the census
    /// or the board.
    #[test]
    fn a_line_runs_as_one_verify_ui_of_this_runner() {
        let runner = Path::new("target/debug/xtask");
        let shots = Path::new("out/000-band");
        let building = step_for(runner, "co-authors 4 --preset co-authors", shots, true);
        assert_eq!(
            building,
            words(&[
                "target/debug/xtask",
                "verify-ui",
                "co-authors",
                "4",
                "--preset",
                "co-authors",
                "--no-board",
                "--no-census",
                "--shot-dir",
                "out/000-band",
            ])
        );
        let reusing = step_for(runner, "band", shots, false);
        assert_eq!(reusing.last().map(String::as_str), Some("--no-build"));
        // The weight the ledger counts each as (`budget::weight_of`).
        assert_eq!(
            crate::budget::weight_of(&building, false),
            crate::budget::COMPILE
        );
        assert_eq!(
            crate::budget::weight_of(&reusing, true),
            crate::budget::LIGHT
        );
    }

    #[test]
    fn a_named_directory_is_used_only_empty_and_never_emptied() {
        let yard = crate::yard::Yard::new("replay-out");
        let named = yard.join("shots");
        assert_eq!(
            out_dir(&yard, Some(&named))
                .expect("a directory that is not there yet")
                .dir,
            named
        );
        assert_eq!(
            out_dir(&yard, Some(&named))
                .expect("the same one, still empty")
                .dir,
            named
        );
        std::fs::write(named.join("app.png"), b"").expect("a picture");
        let Err(refusal) = out_dir(&yard, Some(&named)) else {
            panic!("a directory holding somebody's pictures was taken");
        };
        assert!(refusal.contains("holds something already"), "{refusal}");
        assert!(
            named.join("app.png").exists(),
            "and it was left as it stood"
        );
    }

    #[test]
    fn the_trees_own_directory_is_stood_up_again_by_one_run_at_a_time() {
        let yard = crate::yard::Yard::new("replay-own");
        let first = out_dir(&yard, None).expect("the tree's own");
        assert_eq!(first.dir, yard.join("target").join("replay"));
        std::fs::write(first.dir.join("000-band.log"), b"the first run's").expect("a log");

        let Err(refusal) = out_dir(&yard, None) else {
            panic!("a second run took the first's directory down under it");
        };
        assert!(refusal.contains("another replay is writing"), "{refusal}");
        assert!(
            first.dir.join("000-band.log").exists(),
            "and the first run's log stands"
        );

        drop(first);
        let next = out_dir(&yard, None).expect("the tree's own, once the first run is done");
        assert!(
            !next.dir.join("000-band.log").exists(),
            "the last run's is gone"
        );
    }

    /// A first line that went red before its build left no release, so the
    /// second builds too; `--no-build` is the caller's word that one stands.
    #[test]
    fn a_line_builds_until_one_has_gone_green() {
        let red = || row("band", false, "000-band");
        let green = || row("stash", true, "001-stash");
        assert!(builds_next(&[], true));
        assert!(builds_next(&[red()], true));
        assert!(builds_next(&[red(), red()], true));
        assert!(!builds_next(&[red(), green()], true));
        assert!(!builds_next(&[green(), red()], true));
        assert!(!builds_next(&[], false));
        assert!(!builds_next(&[red()], false));
    }

    /// How one line of a scripted walk ends: `(passed, unbuilt)`, or why it
    /// never ran.
    type Ends<'a> = Result<(bool, bool), &'a str>;

    /// Walks `lines` with each one ending as scripted, and says beside the
    /// walk what each line that was reached was told about building.
    fn walked<'a>(lines: &'a [String], ends: &[Ends<'_>]) -> (Walked<'a>, Vec<bool>) {
        let mut told = Vec::new();
        let walked = walk(lines, true, &mut |index, line, build| {
            told.push(build);
            let (passed, unbuilt) = ends[index]?;
            Ok(Row {
                unbuilt,
                ..row(line, passed, "a-run")
            })
        });
        (walked, told)
    }

    fn lines_of(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.line.as_str()).collect()
    }

    #[test]
    fn a_red_line_stops_none_of_the_others() {
        let lines = words(&["band", "stash", "amend"]);
        let (ended, told) = walked(
            &lines,
            &[Ok((false, false)), Ok((true, false)), Ok((false, false))],
        );
        assert_eq!(lines_of(&ended.rows), ["band", "stash", "amend"]);
        assert!(ended.not_run.is_none());
        // The second builds again; the third reads what the second built.
        assert_eq!(told, [true, true, false]);
    }

    /// A line whose red is the build's still ran, and the ones after it
    /// did not; a line that never ran is left with the ones after it.
    #[test]
    fn a_run_stops_where_no_later_line_could_follow() {
        let lines = words(&["band", "stash", "amend"]);

        let (ended, told) = walked(
            &lines,
            &[Ok((false, true)), Ok((true, false)), Ok((true, false))],
        );
        assert_eq!(lines_of(&ended.rows), ["band"]);
        let (left, why) = ended.not_run.expect("two lines the build left unrun");
        assert_eq!(left, &lines[1..]);
        assert_eq!(why, "the app did not build");
        assert_eq!(told, [true]);

        let (ended, _) = walked(
            &lines,
            &[Ok((true, false)), Err("no room"), Ok((true, false))],
        );
        assert_eq!(lines_of(&ended.rows), ["band"]);
        let (left, why) = ended
            .not_run
            .expect("the line that never ran, and the one after");
        assert_eq!(left, &lines[1..]);
        assert_eq!(why, "no room");

        // On the last line there is nothing left to name.
        let (ended, _) = walked(
            &lines,
            &[Ok((true, false)), Ok((true, false)), Ok((false, true))],
        );
        assert_eq!(lines_of(&ended.rows), ["band", "stash", "amend"]);
        assert!(ended.not_run.is_none());
    }

    #[test]
    fn a_red_build_is_read_off_a_building_lines_log() {
        assert!(build_went_red(
            "error: could not compile `platitude-app` (bin \"platitude-gg\") due to 1 previous error\n"
        ));
        assert!(build_went_red("error: cargo build --release failed\n"));
        assert!(!build_went_red("FAIL: band in 2.1s (exit 1)\n"));
    }

    #[test]
    fn the_page_puts_the_red_lines_first_and_keeps_a_bar_inside_its_cell() {
        let rows = [
            row("band", true, "000-band"),
            row(
                "publish-new-go origin|x",
                false,
                "001-publish-new-go-origin-x",
            ),
        ];
        let page = summary(&rows, None, "on macos, Qt 6.12.0");
        assert!(
            page.starts_with("### replay: 2 line(s) run, 1 red\n\non macos, Qt 6.12.0\n"),
            "{page}"
        );
        let fail = page
            .find("| FAIL | `publish-new-go origin\\|x` | 3 |")
            .expect("the red row");
        let ok = page
            .find("| ok | `band` | 3 | `000-band/`, `000-band.log` |")
            .expect("the green row");
        assert!(fail < ok, "{page}");
        assert!(!page.contains("Not run"), "{page}");
    }

    #[test]
    fn the_page_names_the_lines_a_run_stopped_short_of() {
        let left = words(&["stash", "nav-tip branch:0"]);
        let page = summary(
            &[row("band", false, "000-band")],
            Some((&left, "the app did not build")),
            "on macos, Qt 6.12.0",
        );
        assert!(
            page.starts_with("### replay: 1 line(s) run, 1 red, 2 not run\n"),
            "{page}"
        );
        assert!(
            page.ends_with(
                "\nNot run — the app did not build:\n\n- `stash`\n- `nav-tip branch:0`\n"
            ),
            "{page}"
        );
    }
}
