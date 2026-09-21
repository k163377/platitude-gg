//! The landing permit: main moves on the user's own message, once.
//!
//! The ask is read where the harness shows the user's own words
//! (UserPromptSubmit) and kept per session beside the primary checkout's
//! `.git`. What it holds is one fact about the latest message:
//!
//! * it asked for main (反映) and nothing has landed on it — open. The
//!   landing verb goes through on it as many times as it takes: a landing
//!   that stopped before main moved (a dirty seat, a red gate, a rebase
//!   that halted) fulfilled nothing, and the ask stands until one does;
//! * it asked, and a landing moved main on it — spent. The land verb
//!   writes this at the fast-forward (`landed`);
//! * it did not ask — closed. A correction, an answer to a question, an
//!   approval of something else: none carries an earlier ask forward. A
//!   message that asks again opens it again.
//!
//! Only the user's own message moves it. The context a summary carries
//! back and the harness's tagged events arrive as prompts too, and are
//! nobody's ask and nobody's answer: they leave the permit as it stands.
//!
//! The word that asks is the user's own, 反映, net of the forms that
//! describe or forbid. land / merge / マージ are not read:
//! in a git client they name features far more often than the landing
//! (the merge editor, マージン, tipLanded). What the word catches and
//! misses is measured in internal-docs/反映前テストの機械化.md §land の許可.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::payload::{deny, string_field};

/// The user's word for putting a branch on main.
const ASK: &str = "反映";

/// The forms that hold the ask whole but describe a state or forbid the
/// act. Japanese has no word boundary to test, so each is counted and
/// taken off: a message asks when it holds more asks than these.
const CONTAINERS: [&str; 8] = [
    "反映され",
    "反映済",
    "反映前",
    "未反映",
    "反映漏れ",
    "反映するな",
    "反映しな",
    "反映せず",
];

/// What the harness hands in as a prompt when an earlier context was
/// summarized: it quotes whatever the user said before, and none of it
/// is said now.
const CARRIED_CONTEXT: &str = "This session is being continued from a previous conversation";

/// How much of the message the refusal quotes back, in characters.
const EXCERPT: usize = 60;

/// Field separator in the permit file; every value written has tabs and
/// newlines taken out, as the chip ledger does.
const SEP: char = '\t';

/// Where the permit stands.
#[derive(Debug, PartialEq, Clone, Copy)]
enum Standing {
    /// The latest message asked, and no landing has moved main on it.
    Open,
    /// The latest message asked, and a landing moved main on it.
    Spent,
    /// The latest message did not ask.
    Closed,
}

/// The permit as the file remembers it: where it stands, when the
/// message it answers for arrived, and enough of that message to quote.
struct Permit {
    standing: Standing,
    asked_at: u64,
    excerpt: String,
}

/// UserPromptSubmit: the user's message opens or closes the permit; a
/// prompt that is not the user's leaves it as it stands. An open ask
/// supplies the completion check before the session starts landing.
pub(super) fn prompt_submit(input: &str, prompt: &str) {
    if !is_the_users_own(prompt) {
        return;
    }
    let Some(path) = permit_path(input) else {
        return;
    };
    let permit = Permit {
        standing: if asks_for_main(prompt) {
            Standing::Open
        } else {
            Standing::Closed
        },
        asked_at: now(),
        excerpt: excerpt(prompt),
    };
    store(&path, &permit);
    if permit.standing == Standing::Open {
        println!(
            "Before landing, complete and commit the requested work under CLAUDE.md §Git 運用. \
             The permit authorizes one landing; completion is the session's to check. \
             Land runs the final gate."
        );
    }
}

/// PreToolUse: whether `what` — the landing verb — is refused for want
/// of a permit. Prints the refusal when it is. Letting it through spends
/// nothing: the landing that moves main is what spends the permit.
pub(super) fn landing_denied(input: &str, what: &str) -> bool {
    let permit = permit_path(input).and_then(|path| load(&path));
    if permit.as_ref().map(|p| p.standing) == Some(Standing::Open) {
        return false;
    }
    deny(&refusal(what, permit.as_ref()));
    true
}

/// The land verb, at the fast-forward: main moved on this session's
/// permit, and the permit is spent by it. `dir` is any directory of the
/// repository, `session` the id the session runs under. A permit that is
/// not open is left as it is — a landing the user ran by hand, or from a
/// tree whose hooks predate the permit, answered to nobody's message.
pub(crate) fn landed(dir: &str, session: &str) {
    if let Some(path) = permit_file(dir, session) {
        spend(&path);
    }
}

/// Stop: distinguish an unfulfilled landing from work left after one.
/// Dirty files count even when HEAD is already on main; they have not
/// reached the gate's committed-tip standing yet.
pub(super) fn unmet(input: &str) -> Option<String> {
    let permit = load(&permit_path(input)?)?;
    if permit.standing == Standing::Closed {
        return None;
    }
    let cwd = string_field(input, "cwd")?;
    let ahead = crate::seats::commits_in(&cwd, "main..HEAD")?;
    let dirty = crate::seats::dirty_lines(&cwd)?;
    if ahead == 0 && dirty == 0 {
        return None;
    }
    let remaining = format!("{ahead} commit(s) not on main and {dirty} uncommitted file(s)");
    Some(match permit.standing {
        Standing::Open => format!(
            "permit: the user's message 「{}」 ({}) asked for main (反映), and this turn ends \
             without a landing having moved it — {remaining} remain in {cwd}.",
            permit.excerpt,
            ago(permit.asked_at)
        ),
        Standing::Spent => format!("permit: work remains after the landing: {remaining} in {cwd}."),
        Standing::Closed => return None,
    })
}

/// Whether a prompt is the user's own words. Two other things arrive as
/// prompts: the context a summarized session carries back, which quotes
/// what the user said before, and the harness's tagged events
/// (`<task-notification>`, `<ci-monitor-event>`), which quote nothing
/// the user said at all.
fn is_the_users_own(prompt: &str) -> bool {
    let text = prompt.trim_start();
    !text.starts_with(CARRIED_CONTEXT) && !opens_a_tag(text)
}

fn opens_a_tag(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next() == Some('<') && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
}

/// Whether the message asks for main.
fn asks_for_main(prompt: &str) -> bool {
    let containers: usize = CONTAINERS
        .iter()
        .map(|form| prompt.matches(form).count())
        .sum();
    prompt.matches(ASK).count() > containers
}

/// The refusal, by what the permit says. Every form ends the same way,
/// because the way out is the same: the branch stays, the user reads.
fn refusal(what: &str, permit: Option<&Permit>) -> String {
    let report = "This permit controls land only. Complete the work under CLAUDE.md §Git 運用 \
                  before reporting its branch/SHA and verification result";
    match permit {
        Some(Permit {
            standing: Standing::Spent,
            asked_at,
            excerpt,
        }) => format!(
            "{what} would move main a second time on one message. The user's message \
             「{excerpt}」 ({}) asked for one landing, and that landing moved main. \
             {report}; the user's next 反映 opens the next landing.",
            ago(*asked_at)
        ),
        Some(Permit {
            standing: Standing::Closed,
            asked_at,
            excerpt,
        }) => format!(
            "{what} would move main, and main moves only on the user's own ask — 反映 \
             (main 反映 / 反映して) in their latest message (CLAUDE.md Git 運用). That \
             message does not ask: 「{excerpt}」 ({}). An ask given before it does not carry \
             over. {report}; when the user asks, the same command goes through.",
            ago(*asked_at)
        ),
        _ => format!(
            "{what} would move main, and main moves only on the user's own ask — 反映 \
             (main 反映 / 反映して), read off the user's own message by the prompt hook \
             (CLAUDE.md Git 運用). No message in \
             this session has asked. {report}."
        ),
    }
}

/// The message on one line, cut to what a refusal can quote.
fn excerpt(prompt: &str) -> String {
    let one_line = prompt.split_whitespace().collect::<Vec<&str>>().join(" ");
    let mut cut: String = one_line.chars().take(EXCERPT).collect();
    if one_line.chars().count() > EXCERPT {
        cut.push('…');
    }
    cut
}

/// This session's permit, out of a hook payload.
fn permit_path(input: &str) -> Option<PathBuf> {
    let session = string_field(input, "session_id")?;
    let cwd = string_field(input, "cwd")?;
    permit_file(&cwd, &session)
}

/// `.permits/<session>.land` beside the primary checkout's `.git` (where
/// the chip ledger sits too), so the seat's landing reads what the prompt
/// hook wrote from wherever the session sat when the message arrived.
fn permit_file(dir: &str, session: &str) -> Option<PathBuf> {
    let session: String = session
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if session.is_empty() {
        return None;
    }
    let common = crate::subprocess::common_git_dir(dir)?;
    let root = Path::new(&common).parent()?;
    Some(root.join(".permits").join(format!("{session}.land")))
}

/// An open permit becomes spent; any other stays what it is.
fn spend(path: &Path) {
    let Some(mut permit) = load(path) else {
        return;
    };
    if permit.standing != Standing::Open {
        return;
    }
    permit.standing = Standing::Spent;
    store(path, &permit);
}

fn load(path: &Path) -> Option<Permit> {
    let text = std::fs::read_to_string(path).ok()?;
    parse(text.lines().next()?)
}

fn parse(line: &str) -> Option<Permit> {
    let mut fields = line.split(SEP);
    let standing = match fields.next()? {
        "open" => Standing::Open,
        "spent" => Standing::Spent,
        "closed" => Standing::Closed,
        _ => return None,
    };
    let asked_at = fields.next()?.parse().ok()?;
    let excerpt = fields.next().unwrap_or_default().to_string();
    Some(Permit {
        standing,
        asked_at,
        excerpt,
    })
}

/// Writes the permit. Advisory in one direction only: a permit that
/// cannot be written reads as absent, and an absent permit refuses.
fn store(path: &Path, permit: &Permit) {
    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let word = match permit.standing {
        Standing::Open => "open",
        Standing::Spent => "spent",
        Standing::Closed => "closed",
    };
    let line = format!("{word}{SEP}{}{SEP}{}\n", permit.asked_at, permit.excerpt);
    if let Err(_unheard) = std::fs::write(path, line) {
        // Nobody to tell from inside a hook; the next landing reads
        // whatever the file holds, and refuses if it holds nothing.
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// How long ago `secs` was, coarsely: what the refusal needs is whether
/// the message is this turn's or an hour old.
fn ago(secs: u64) -> String {
    let elapsed = now().saturating_sub(secs);
    if elapsed < 60 {
        format!("{elapsed}s ago")
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else {
        format!("{}h{}m ago", elapsed / 3600, (elapsed % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EXCERPT, Permit, Standing, asks_for_main, excerpt, is_the_users_own, load, parse, refusal,
        spend, store,
    };

    #[test]
    fn the_users_own_askings_open_and_descriptions_do_not() {
        for prompt in [
            "main反映",
            "OK main反映して終了",
            "許可するから反映し切って",
            "チェックOKならmain反映",
            "main反映後起動",
            "レビュー・必要なら修正とコミットログ再編、チェックOKならmain反映",
            "反映して",
            "未反映の分を反映して",
        ] {
            assert!(asks_for_main(prompt), "{prompt}");
        }
        for prompt in [
            "反映されていない部分が有りそう、直して",
            "直近の修正でmain反映漏れてるものってない？",
            "main反映前に起動して",
            "反映済みなら消して 未反映のは残して",
            "mainに反映するな",
            "起動",
            "1で",
            "ここも直して",
        ] {
            assert!(!asks_for_main(prompt), "{prompt}");
        }
    }

    #[test]
    fn a_summary_and_a_tagged_event_are_nobodys_message() {
        for prompt in [
            "This session is being continued from a previous conversation that ran out \
             of context. The user said: main反映",
            "<task-notification>agent done</task-notification>",
            "<ci-monitor-event>build failed</ci-monitor-event>",
        ] {
            assert!(!is_the_users_own(prompt), "{prompt}");
        }
        for prompt in ["main反映", "1で", "<- この矢印の向きを直して", ""] {
            assert!(is_the_users_own(prompt), "{prompt}");
        }
    }

    #[test]
    fn a_permit_line_reads_back_and_a_foreign_one_reads_as_absent() {
        let permit = parse("spent\t1700000000\tOK main反映").expect("permit");
        assert_eq!(permit.standing, Standing::Spent);
        assert_eq!(permit.asked_at, 1_700_000_000);
        assert_eq!(permit.excerpt, "OK main反映");
        assert!(parse("granted\t1\tx").is_none());
        assert!(parse("open\tsoon\tx").is_none());
    }

    #[test]
    fn the_refusal_names_the_word_the_message_and_the_standing() {
        let closed = Permit {
            standing: Standing::Closed,
            asked_at: 0,
            excerpt: "起動".into(),
        };
        let text = refusal("`cargo xtask land`", Some(&closed));
        assert!(
            text.contains("does not ask") && text.contains("反映") && text.contains("「起動」"),
            "{text}"
        );
        let spent = Permit {
            standing: Standing::Spent,
            ..closed
        };
        let text = refusal("`cargo xtask land`", Some(&spent));
        assert!(
            text.contains("second time") && text.contains("moved main"),
            "{text}"
        );
        let text = refusal("`cargo xtask land`", None);
        assert!(
            text.contains("No message") && text.contains("反映"),
            "{text}"
        );
        for text in [refusal("x", Some(&spent)), refusal("x", None)] {
            assert!(text.contains("CLAUDE.md §Git 運用"), "{text}");
        }
    }

    #[test]
    fn an_excerpt_is_one_line_and_bounded() {
        assert_eq!(excerpt("a\tb\n\nc"), "a b c");
        let cut = excerpt(&"あ".repeat(EXCERPT + 1));
        assert_eq!(cut.chars().count(), EXCERPT + 1);
        assert!(cut.ends_with('…'));
        assert_eq!(excerpt(&"い".repeat(EXCERPT)).chars().count(), EXCERPT);
    }

    #[test]
    fn a_permit_survives_the_file_and_only_an_open_one_is_spent() {
        let dir = crate::yard::Yard::new("permit");
        let path = dir.join("session.land");
        assert!(load(&path).is_none());
        spend(&path);
        assert!(load(&path).is_none(), "spending nothing writes nothing");
        store(
            &path,
            &Permit {
                standing: Standing::Closed,
                asked_at: 7,
                excerpt: "起動".into(),
            },
        );
        spend(&path);
        assert_eq!(load(&path).expect("permit").standing, Standing::Closed);
        store(
            &path,
            &Permit {
                standing: Standing::Open,
                asked_at: 7,
                excerpt: "main反映".into(),
            },
        );
        spend(&path);
        let again = load(&path).expect("permit");
        assert_eq!(
            (again.standing, again.asked_at, again.excerpt.as_str()),
            (Standing::Spent, 7, "main反映")
        );
        spend(&path);
        assert_eq!(load(&path).expect("permit").standing, Standing::Spent);
    }
}
