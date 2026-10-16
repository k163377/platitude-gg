//! The one window the board is read in: `open` closes the board window
//! standing and opens a fresh one.
//!
//! Nothing is written down about a window between commands. A browser
//! handed a page passes it to the process already running and leaves no
//! child to watch, so a note saying "open" outlives the window it was
//! written for, and the reader who closed theirs is sent to press F5 in
//! nothing. The window is found instead, at the moment it is to be
//! closed, by the browser profile it runs on ([`profile`]).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::board::shown;
use crate::subprocess::{Answer, bounded};
use crate::wait::{Budget, Wait};

/// The board's own browser profile, beside the page and out of the
/// sweeps' reach (`runs/`, `img/`). Every process of the board's browser
/// names it on its command line and nothing else on the machine does, so
/// it is how `open` finds the window — the reader's own browser, on its
/// own profile, is never touched.
fn profile(board: &Path) -> PathBuf {
    board.join("browser")
}

/// How long the board's browser, ended, is given to be gone from the
/// listing: a new window on the same profile while it still stands would
/// be handed to it and go with it.
const ENDING: Duration = Duration::from_secs(10);

/// Between looks: each one is a listing of the machine's processes.
const PACE: Duration = Duration::from_millis(250);

/// How long one listing may take: a CIM query can wedge under a gate's
/// load while its process stays alive (`footprint`).
const ASKING: Duration = Duration::from_secs(30);

/// Puts the board in front of the reader, in a window of its own: the
/// board window already standing is closed first, so there is one. A
/// board opened in the default browser runs on no profile of the
/// board's, so no later `open` finds it to close.
pub(super) fn restart(board: &Path, page: &Path) -> Result<(), String> {
    let profile = profile(board);
    if close(board, &profile)? {
        println!("board: closed the window that was open");
    }
    let opened = spawn(page, &profile)?;
    if opened.own_profile {
        println!("board: opened {} with {}", shown(page), opened.program);
    } else {
        println!("board: opened {} in the default browser", shown(page));
    }
    Ok(())
}

/// Ends the board's browser and waits until it is gone; answers whether
/// one stood.
///
/// Ended outright, not asked to close: a browser asked answers in its own
/// time, and a wait run out on it could only fail. What it had open is not
/// offered back at the next start (`--hide-crash-restore-bubble`).
fn close(board: &Path, profile: &Path) -> Result<bool, String> {
    let standing = browsers_on(board, profile)?;
    if standing.is_empty() {
        return Ok(false);
    }
    for pid in &standing {
        end(*pid);
    }
    let mut wait = Wait::new("shots open", Budget::whole(ENDING), PACE);
    loop {
        let left = browsers_on(board, profile)?;
        if left.is_empty() {
            return Ok(true);
        }
        let left: Vec<String> = left.iter().map(u32::to_string).collect();
        wait.saw(format!("pid {}", left.join(", ")));
        wait.look_again("the board's browser to end")
            .map_err(|expired| format!("{expired} — close the board window by hand"))?;
    }
}

/// The browser processes started on `profile`: the ones that own its
/// windows. Its helpers name the profile too, and go with it.
fn browsers_on(board: &Path, profile: &Path) -> Result<Vec<u32>, String> {
    Ok(started_on(&listing(board)?, &profile.to_string_lossy()))
}

/// [`browsers_on`] over a listing of `<pid> <command line>` lines. Slashes
/// and case are not compared: the command line holds the profile as the
/// browser was handed it, and one board is reached by more than one
/// spelling of its path.
fn started_on(listing: &str, profile: &str) -> Vec<u32> {
    let flag = folded(&format!("--user-data-dir={profile}"));
    listing
        .lines()
        .filter_map(|line| {
            let (pid, command) = line.trim().split_once(char::is_whitespace)?;
            let command = folded(command);
            (names(&command, &flag) && !command.contains("--type=")).then(|| pid.parse().ok())?
        })
        .collect()
}

/// Whether `command` holds `flag` whole: a profile whose name only begins
/// like the board's is another browser's.
fn names(command: &str, flag: &str) -> bool {
    command.match_indices(flag).any(|(at, _)| {
        command[at + flag.len()..]
            .chars()
            .next()
            .is_none_or(|next| next.is_whitespace() || next == '"')
    })
}

fn folded(text: &str) -> String {
    text.replace('\\', "/").to_lowercase()
}

/// Every process of the browsers [`browsers`] starts on a profile, with
/// its command line. PowerShell is the one stock tool that answers with
/// the command line, in the console's code page unless told otherwise —
/// a path outside ASCII would not match. The matching stays in Rust where
/// quoting cannot bend it.
#[cfg(windows)]
fn listing(board: &Path) -> Result<String, String> {
    const LISTING: &str = "[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false
Get-CimInstance Win32_Process -Filter \"Name='chrome.exe' OR Name='msedge.exe'\" | \
 ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }";
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", LISTING]);
    said(board, command)
}

/// One `-o` per field: the two `ps` this runs under read a comma-joined
/// list with empty headings differently.
#[cfg(not(windows))]
fn listing(board: &Path) -> Result<String, String> {
    let mut command = Command::new("ps");
    command.args(["-A", "-o", "pid=", "-o", "args="]);
    said(board, command)
}

/// What `listing` wrote, under [`ASKING`]. The file is this process's
/// own, so two `open`s at once do not read each other's.
fn said(board: &Path, listing: Command) -> Result<String, String> {
    let file = board.join(format!("listing.{}.txt", std::process::id()));
    let answer = bounded("a listing of processes", listing, ASKING, Some(&file));
    // Read already, or never written.
    let _ = std::fs::remove_file(&file);
    match answer {
        Answer::Ended { status, stdout } if status.success() => Ok(stdout),
        Answer::Ended { status, .. } => Err(format!(
            "could not list the board's browser: the listing exited {status}"
        )),
        Answer::OutOfTime { after, .. } => Err(format!(
            "could not list the board's browser: no answer within {:.0}s",
            after.as_secs_f32()
        )),
        Answer::Unstarted(why) => Err(why),
    }
}

/// Ends the browser outright — on Windows with the tree under it; on unix
/// its helpers follow it out. The number was read off the listing a
/// moment ago together with the board's profile, so it is not one handed
/// on to a stranger.
fn end(pid: u32) {
    let mut command = if cfg!(windows) {
        let mut c = Command::new("taskkill");
        c.args(["/F", "/T", "/PID", &pid.to_string()]);
        c
    } else {
        let mut c = Command::new("kill");
        c.args(["-9", &pid.to_string()]);
        c
    };
    // The status is not read: a browser gone since the listing makes it
    // fail, and the wait after this is what says whether it went.
    let _ = crate::subprocess::run_captured(&mut command);
}

/// One way to open the board.
struct Browser {
    program: String,
    args: Vec<String>,
    /// Started on the board's profile, so the next `open` finds it.
    own_profile: bool,
}

/// Opens the board in a window of its own (a browser in `--app` mode, on
/// the board's profile), answering what opened it. System image viewers
/// are no candidate: they smooth what they magnify.
fn spawn(page: &Path, profile: &Path) -> Result<Browser, String> {
    let url = format!("file:///{}", page.display().to_string().replace('\\', "/"));
    browsers(&url, profile)
        .into_iter()
        .find(|browser| start(&browser.program, &browser.args))
        .ok_or_else(|| {
            format!(
                "found no browser to open {}: open it by hand",
                page.display()
            )
        })
}

/// Starts `program` handing it nothing of this process: the browser stays
/// for as long as the window does, and a pipe it inherited from the
/// caller would stay open with it — the caller reading it would wait on
/// the window.
///
/// Windows hands a child every handle of the parent's marked inheritable,
/// whatever its stdio is set to, and the stdout `cargo` hands this process
/// is one. `Start-Process` given only a program and its arguments goes
/// through ShellExecuteEx, which hands on none; `-NoNewWindow`, a redirect
/// or a credential would make it a CreateProcess that inherits again.
#[cfg(windows)]
fn start(program: &str, args: &[String]) -> bool {
    crate::subprocess::run_captured(Command::new("powershell").args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &shell_start(program, args),
    ]))
    .is_ok_and(|out| out.status.success())
}

/// On unix the child's stdio replaces the parent's in place, and nothing
/// else this process holds is left open across the start.
#[cfg(not(windows))]
fn start(program: &str, args: &[String]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

/// The PowerShell line that starts `program` with `args`. Each argument
/// holding a space is quoted whole, as the browser splits its command
/// line; the line is then one single-quoted PowerShell string. No
/// arguments, no `-ArgumentList`: it refuses an empty one.
#[cfg(any(windows, test))]
fn shell_start(program: &str, args: &[String]) -> String {
    let literal = |text: &str| format!("'{}'", text.replace('\'', "''"));
    let mut line = format!("Start-Process -FilePath {}", literal(program));
    if !args.is_empty() {
        let quoted: Vec<String> = args
            .iter()
            .map(|arg| {
                if arg.contains(char::is_whitespace) {
                    format!("\"{arg}\"")
                } else {
                    arg.clone()
                }
            })
            .collect();
        line.push_str(&format!(" -ArgumentList {}", literal(&quoted.join(" "))));
    }
    line
}

/// Candidates in order of preference, per OS: the browsers that run on a
/// profile of the board's own, then the default browser. The first-run,
/// default-browser and restore screens are turned off: the board's
/// profile is new to the browser once, and its browser is ended outright
/// ([`close`]).
fn browsers(url: &str, profile: &Path) -> Vec<Browser> {
    let on_profile = vec![
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
        "--hide-crash-restore-bubble".to_string(),
        format!("--app={url}"),
    ];
    let own = |program: &str, args: Vec<String>| Browser {
        program: program.to_string(),
        args,
        own_profile: true,
    };
    let default = |program: &str, args: Vec<String>| Browser {
        program: program.to_string(),
        args,
        own_profile: false,
    };
    if cfg!(windows) {
        vec![
            own(
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                on_profile.clone(),
            ),
            own(
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                on_profile,
            ),
            // ShellExecuteEx opens a URL in the default browser.
            default(url, Vec::new()),
        ]
    } else if cfg!(target_os = "macos") {
        let mut chrome = vec![
            "-na".to_string(),
            "Google Chrome".to_string(),
            "--args".to_string(),
        ];
        chrome.extend(on_profile);
        vec![own("open", chrome), default("open", vec![url.to_string()])]
    } else {
        vec![
            own("google-chrome", on_profile.clone()),
            own("chromium", on_profile),
            default("xdg-open", vec![url.to_string()]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{browsers, shell_start, started_on};

    #[test]
    fn the_shell_is_handed_each_argument_whole() {
        let line = shell_start(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            &[
                r"--user-data-dir=C:\Users\A B\it's\browser".to_string(),
                "--no-first-run".to_string(),
            ],
        );
        assert_eq!(
            line,
            "Start-Process -FilePath 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe' \
             -ArgumentList '\"--user-data-dir=C:\\Users\\A B\\it''s\\browser\" --no-first-run'"
        );
        assert_eq!(
            shell_start("file:///C:/b/index.html", &[]),
            "Start-Process -FilePath 'file:///C:/b/index.html'"
        );
    }

    #[test]
    fn every_candidate_is_handed_the_board() {
        let candidates = browsers("file:///b/index.html", Path::new("/b/browser"));
        assert!(candidates.iter().any(|browser| browser.own_profile));
        assert!(
            candidates
                .last()
                .is_some_and(|browser| !browser.own_profile),
            "the default browser is the last resort"
        );
        for browser in candidates {
            let mut said = browser.args.clone();
            said.push(browser.program.clone());
            assert!(
                said.iter().any(|a| a.contains("file:///b/index.html")),
                "a candidate was handed no board: {said:?}"
            );
            assert_eq!(
                browser.own_profile,
                browser
                    .args
                    .iter()
                    .any(|a| a == "--user-data-dir=/b/browser"),
                "a candidate says it runs on the board's profile and does not, or the reverse: {said:?}"
            );
        }
    }

    /// The browser on the board's profile, and nothing else: not its
    /// helpers, not the reader's own browser, not a profile whose name
    /// only begins like the board's.
    #[test]
    fn the_board_window_is_the_browser_on_the_boards_profile() {
        let listing = "\
            100 \"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\" \
                --user-data-dir=C:\\x\\platitude-gg\\.shots\\browser --no-first-run \
                --app=file:///C:/x/platitude-gg/.shots/index.html\r\n\
            200 \"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\" --type=renderer \
                --user-data-dir=C:\\x\\platitude-gg\\.shots\\browser\r\n\
            300 \"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\"\r\n\
            400 \"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\" \
                --user-data-dir=C:\\x\\platitude-gg\\.shots\\browser-old\r\n\
            not a line\r\n";
        assert_eq!(
            started_on(listing, "C:/x/platitude-gg/.shots/browser"),
            vec![100]
        );
    }

    /// A profile path with a space in it reaches the command line quoted
    /// whole, flag and all.
    #[test]
    fn a_quoted_profile_is_still_the_boards() {
        let listing = "7 chrome.exe \"--user-data-dir=C:\\Users\\A B\\.shots\\browser\" --app=x\n";
        assert_eq!(
            started_on(listing, "C:\\Users\\A B\\.shots\\browser"),
            vec![7]
        );
    }

    /// The primary checkout reads its board's path off the working
    /// directory's spelling and a seat off its `.git` file, so one board
    /// comes in more than one case.
    #[test]
    fn a_profile_spelt_in_another_case_is_still_the_boards() {
        let listing = "9 chrome.exe --user-data-dir=C:\\Users\\Me\\.shots\\browser --app=x\n";
        assert_eq!(started_on(listing, "c:/users/me/.shots/browser"), vec![9]);
    }
}
