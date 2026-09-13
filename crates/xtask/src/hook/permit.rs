//! The permission a landing runs on: one message from the user, spent
//! by one landing.
//!
//! `PGG_ALLOW_MAIN` in front of a command is spelled by the session, so
//! it records that the session believed the user asked and nothing
//! more. The ask itself is read where the harness sees the user's own
//! words (UserPromptSubmit) and held here, per session, beside the
//! primary checkout's `.git`:
//!
//! * a message that asks for main opens the permit;
//! * the first landing let through spends it — a second landing on the
//!   same message is a landing nobody asked for, whatever was fixed in
//!   between (a red gate's fix included: main did not move, and what the
//!   fix would land is content the user has not seen);
//! * the next message closes it, spent or not — a correction, an answer
//!   to a question, an approval of something else: none of them carries
//!   the earlier ask forward. A message that asks again opens it again.
//!
//! The word that asks is the user's own, 反映, net of the forms that
//! describe or forbid rather than ask. land / merge / マージ are not read:
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
/// is an ask made now.
const CARRIED_CONTEXT: &str = "This session is being continued from a previous conversation";

/// How much of the message the refusal quotes back, in characters.
const EXCERPT: usize = 60;

/// Field separator in the permit file; every value written has tabs and
/// newlines taken out, as the chip ledger does.
const SEP: char = '\t';

/// Where the permit stands.
#[derive(Debug, PartialEq, Clone, Copy)]
enum Standing {
    /// The latest message asked, and no landing has run on it.
    Open,
    /// The latest message asked, and a landing ran on it.
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

/// UserPromptSubmit: the message opens or closes the permit. Nothing is
/// printed — the answer is given when a landing asks for it.
pub(super) fn prompt_submit(input: &str, prompt: &str) {
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
}

/// PreToolUse: whether `what` — a command that writes main, escape and
/// all — is refused for want of a permit. Prints the refusal when it is.
pub(super) fn landing_denied(input: &str, what: &str) -> bool {
    let permit = permit_path(input).and_then(|path| load(&path));
    if permit.as_ref().map(|p| p.standing) == Some(Standing::Open) {
        return false;
    }
    deny(&refusal(what, permit.as_ref()));
    true
}

/// PreToolUse, after every guard let a landing through: the permit it
/// runs on is spent here and not after the command, since a landing
/// that stops (a red gate, a rebase that halts) ran on it all the same.
pub(super) fn spend(input: &str) {
    let Some(path) = permit_path(input) else {
        return;
    };
    let Some(mut permit) = load(&path) else {
        return;
    };
    permit.standing = Standing::Spent;
    store(&path, &permit);
}

/// Whether the message asks for main.
fn asks_for_main(prompt: &str) -> bool {
    if prompt.trim_start().starts_with(CARRIED_CONTEXT) {
        return false;
    }
    let containers: usize = CONTAINERS
        .iter()
        .map(|form| prompt.matches(form).count())
        .sum();
    prompt.matches(ASK).count() > containers
}

/// The refusal, by what the permit says. Every form ends the same way,
/// because the way out is the same: the branch stays, the user reads.
fn refusal(what: &str, permit: Option<&Permit>) -> String {
    let report = "Leave the work on its branch, report it as ready to merge \
                  (「worktree-<seat> に積んだ。マージ可」) and end the turn";
    match permit {
        Some(Permit {
            standing: Standing::Spent,
            asked_at,
            excerpt,
        }) => format!(
            "{what} would put commits on main a second time on one instruction. The \
             user's message 「{excerpt}」 ({}) opened exactly one landing and the land \
             that followed spent it; whatever has been committed since — a fix for a \
             red gate included — is content the user has not seen land. {report}; the \
             user's next 反映 opens the next one.",
            ago(*asked_at)
        ),
        Some(Permit {
            standing: Standing::Closed,
            asked_at,
            excerpt,
        }) => format!(
            "{what} would put commits on main, and main moves only on an instruction \
             the user gives for this very piece of work (CLAUDE.md Git 運用). The \
             user's latest message does not ask for it: 「{excerpt}」 ({}). The word \
             that asks is 反映 (main 反映 / 反映して) — a correction, an answer to a \
             question or an ask given before that message does not carry over, and \
             each message that asks opens exactly one landing. {report}; when the \
             user asks, the same command goes through.",
            ago(*asked_at)
        ),
        _ => format!(
            "{what} would put commits on main, and main moves only on an instruction \
             the user gives for this very piece of work (CLAUDE.md Git 運用). No \
             message in this session has asked for it: the word that asks is 反映 \
             (main 反映 / 反映して), read off the user's own message by the prompt \
             hook, so nothing a session types stands in for it. {report}."
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

/// This session's permit: `.permits/<session>.land` beside the primary
/// checkout's `.git` (where the chip ledger sits too), so the seat's
/// landing reads what the prompt hook wrote from wherever the session
/// sat when the message arrived.
fn permit_path(input: &str) -> Option<PathBuf> {
    let session: String = string_field(input, "session_id")?
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if session.is_empty() {
        return None;
    }
    let cwd = string_field(input, "cwd")?;
    let common = crate::subprocess::common_git_dir(&cwd)?;
    let root = Path::new(&common).parent()?;
    Some(root.join(".permits").join(format!("{session}.land")))
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
    use super::{EXCERPT, Permit, Standing, asks_for_main, excerpt, load, parse, refusal, store};

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
            "This session is being continued from a previous conversation that ran out \
             of context. The user said: main反映",
        ] {
            assert!(!asks_for_main(prompt), "{prompt}");
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
            text.contains("second time") && text.contains("red gate"),
            "{text}"
        );
        let text = refusal("`git merge`", None);
        assert!(
            text.contains("No message") && text.contains("反映"),
            "{text}"
        );
        for text in [refusal("x", Some(&spent)), refusal("x", None)] {
            assert!(text.contains("マージ可"), "{text}");
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
    fn a_permit_survives_the_file_and_spending_keeps_its_message() {
        let dir = std::env::temp_dir().join(format!("pgg-permit-{}", std::process::id()));
        let path = dir.join("session.land");
        assert!(load(&path).is_none());
        store(
            &path,
            &Permit {
                standing: Standing::Open,
                asked_at: 7,
                excerpt: "main反映".into(),
            },
        );
        let mut back = load(&path).expect("permit");
        assert_eq!(back.standing, Standing::Open);
        back.standing = Standing::Spent;
        store(&path, &back);
        let again = load(&path).expect("permit");
        assert_eq!(
            (again.standing, again.asked_at, again.excerpt.as_str()),
            (Standing::Spent, 7, "main反映")
        );
        std::fs::remove_dir_all(dir).expect("cleanup");
    }
}
