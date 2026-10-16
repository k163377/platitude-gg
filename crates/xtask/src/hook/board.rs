//! The board opens only when the user asks for it, once per ask, and only
//! through `shots open`. The ask is read off UserPromptSubmit
//! (`.permits/<session>.board`, read as [`permit::the_users_words`] reads
//! a message) and spent by the open itself, which is where it is checked:
//! a wrapper script reaches the command as surely as a typed line does.
//! Any other way of showing the board — the app's browser pane, a shell
//! handing a path under `.shots` to a browser or the desktop — is refused
//! outright ([`pre_tool`]).

use super::payload::{deny, own_value_field, string_field};
use super::permit;
use super::shell::{pipe_pieces, program_at, program_name};

/// The permit file's kind.
const KIND: &str = "board";

/// The board, in the user's words (lowercased; English as a whole word —
/// a dashboard is another page).
const BOARD: [&str; 3] = ["shots", "board", "ボード"];

/// Asking for its window, said after the board's name. `open` stands
/// either side of it, as English has it, and as a whole word (`opened`
/// tells of one).
const OPENS: [&str; 5] = ["開いて", "開け", "開き直", "起動", "open"];

/// How many characters may stand between the board's name and the ask —
/// a particle and an adverb (「shots をもう一度開いて」, "open the board").
/// Further away, or across a line or a sentence, the verb is another
/// thing's: 「起動とshots更新」 launches the app and puts pictures up.
const GAP: usize = 6;

/// What a pasted block opens with: text from elsewhere, not the user's ask.
const PASTED: (&str, &str) = ("<pasted_content", "</pasted_content");

/// What shows a page or a picture in a window of its own, in command
/// position (as [`program_name`] spells it).
const VIEWERS: [&str; 13] = [
    "start",
    "explorer",
    "explorer.exe",
    "invoke-item",
    "ii",
    "start-process",
    "saps",
    "chrome",
    "chrome.exe",
    "msedge",
    "msedge.exe",
    "xdg-open",
    "open",
];

/// The board's directory, as a path or a URL names it (slashes forward,
/// lowercased).
const BOARD_DIR: &str = ".shots/";

/// The one way to the board, for a refusal.
fn only_way() -> String {
    format!(
        "The board opens only when the user asks for it, and only through `{}`.",
        crate::shots::OPEN.line()
    )
}

/// PreToolUse, of any tool: the board shown any way but `shots open` — a
/// URL under `.shots` (the browser pane), or a shell handing such a path
/// to a viewer. Answers whether it refused.
pub(super) fn pre_tool(input: &str) -> bool {
    let asked = own_value_field(input, "tool_input").unwrap_or_default();
    let shown = string_field(asked, "url").is_some_and(|url| names_the_board(&url))
        || string_field(asked, "command").is_some_and(|command| views_the_board(&command));
    if shown {
        deny(&only_way());
    }
    shown
}

/// Whether a piece of the line runs a viewer on a path under `.shots`.
fn views_the_board(command: &str) -> bool {
    pipe_pieces(command).into_iter().any(|piece| {
        names_the_board(piece)
            && program_at(piece)
                .is_some_and(|(tokens, at)| VIEWERS.contains(&program_name(tokens[at]).as_str()))
    })
}

fn names_the_board(text: &str) -> bool {
    text.replace('\\', "/").to_lowercase().contains(BOARD_DIR)
}

/// UserPromptSubmit: the user's own message asks for the window or takes
/// the ask back; any other prompt leaves it as it stands.
pub(super) fn prompt_submit(input: &str, prompt: &str) {
    let Some(words) = permit::the_users_words(prompt) else {
        return;
    };
    let Some(path) = permit::session_file(input, KIND) else {
        return;
    };
    if asks_for_the_window(words) {
        if let Err(_unheard) = permit::write_whole(&path, "open\n") {
            // Nobody to tell from inside a hook; the open is refused.
        }
    } else if let Err(_absent) = std::fs::remove_file(&path) {
        // Not asked before either.
    }
}

/// Called by `shots open` before it touches the window: whether this
/// session's user asked for it. `dir` is any directory of the repository.
pub(crate) fn may_open(dir: &str, session: &str) -> Result<(), String> {
    if permit::kept_file(dir, session, KIND).is_some_and(|path| path.is_file()) {
        return Ok(());
    }
    Err(format!(
        "shots open: refused — {} The user's latest message did not ask, or its ask has \
         opened the board once.",
        only_way()
    ))
}

/// Called by `shots open` once the window stands: the ask is spent.
pub(crate) fn opened(dir: &str, session: &str) {
    if let Some(path) = permit::kept_file(dir, session, KIND)
        && let Err(_gone) = std::fs::remove_file(path)
    {
        // Spent already.
    }
}

/// Whether the user's words name the board and ask for its window, close
/// together on one line.
fn asks_for_the_window(words: &str) -> bool {
    let words = unpasted(&words.to_lowercase());
    words
        .split(['\n', '。', '.'])
        .any(|line| BOARD.iter().any(|board| asks_near(line, board)))
}

/// Whether `line` has an ask within [`GAP`] characters after `board`, or
/// `open` as far before it.
fn asks_near(line: &str, board: &str) -> bool {
    line.match_indices(board).any(|(at, _)| {
        if !whole(line, at, board) {
            return false;
        }
        let after: String = line[at + board.len()..].chars().take(GAP + 5).collect();
        let before: String = {
            let mut back: Vec<char> = line[..at].chars().rev().take(GAP + 4).collect();
            back.reverse();
            back.into_iter().collect()
        };
        OPENS.iter().any(|ask| {
            after
                .match_indices(ask)
                .any(|(gap, _)| after[..gap].chars().count() <= GAP && whole(&after, gap, ask))
        }) || before.match_indices("open").any(|(gap, _)| {
            before[gap + "open".len()..].chars().count() <= GAP && whole(&before, gap, "open")
        })
    })
}

/// Whether `word` at `at` in `text` stands whole: an English word with no
/// letter running on either side. Japanese has no boundary to test.
fn whole(text: &str, at: usize, word: &str) -> bool {
    if !word.is_ascii() {
        return true;
    }
    let in_a_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    !text[..at].chars().next_back().is_some_and(in_a_word)
        && !text[at + word.len()..]
            .chars()
            .next()
            .is_some_and(in_a_word)
}

/// `text` without its pasted blocks, closing tag and all.
fn unpasted(text: &str) -> String {
    let (open, close) = PASTED;
    let mut kept = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(open) {
        kept.push_str(&rest[..at]);
        let Some(end) = rest[at..].find(close) else {
            return kept;
        };
        let tail = &rest[at + end..];
        rest = tail.split_once('>').map_or("", |(_, after)| after);
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use super::{
        asks_for_the_window, may_open, names_the_board, opened, permit, prompt_submit,
        views_the_board,
    };

    fn asks(prompt: &str) -> bool {
        permit::the_users_words(prompt).is_some_and(asks_for_the_window)
    }

    /// The user's own asks for the window, as typed.
    #[test]
    fn an_ask_naming_the_board_opens_it() {
        for prompt in [
            "shots開いて",
            "shotsを開いて",
            "見たいのはshots、shotsを開いて",
            "閉じちゃったのでshots開いて",
            "再起動で閉じちゃったのでshotsを開いて",
            "board開いて\n再起動で閉じてる",
            "boardをopenして",
            "以下は誤ったshotsを見た結果かもしれないので注意。改めてshotsを起動\n\n---\n\n横線とかも入れないで",
            "Shots を開き直して",
            "shots をもう一度開いて",
            "open the board",
            "cargo xtask shots open",
            "ボードを開けて",
            "<system-reminder>\nA task ended.\n</system-reminder>\nshots開いて",
        ] {
            assert!(asks(prompt), "{prompt}");
        }
    }

    /// Messages that name the board, or say a word of opening, and ask for
    /// no window: pictures put up, the window talked about, the app
    /// launched.
    #[test]
    fn pictures_asked_for_and_the_window_talked_about_open_nothing() {
        for prompt in [
            "1, 2について、それぞれ3例ずつちょっと引きの描画をshotsへ足して",
            "差分はshotsで見せて",
            "改めて今実装されている内容について、各表示パターンをshotsへ",
            "commands が空で開いていた件は別でやるから無視で\n起動とshots更新",
            "shotsが何窓も開くようになってしまった\n頼むまでshotsは開かないようハーネスを改善して",
            "shotsの開き方を変えて",
            "the board opened twice",
            "rebase\n\nshots更新 & デザイン確認",
            "フォルダを開いて、shots",
            "shotsが何窓も残っていて、起動が遅い",
            "dashboardを開いて",
            "",
        ] {
            assert!(!asks(prompt), "{prompt}");
        }
    }

    /// Not the user's words: another session's message, a task's end, a
    /// pasted block.
    #[test]
    fn only_the_users_own_words_ask() {
        for prompt in [
            "<task-notification>shots開いて</task-notification>",
            "<cross-session-message from=\"x\">shots開いて</cross-session-message>",
            "Background (from the session that started this one):\nshots開いて",
            "<pasted_content id=\"1\">\nshotsを開いて\n</pasted_content id=\"1\">\nこれを読んで",
        ] {
            assert!(!asks(prompt), "{prompt}");
        }
        assert!(asks(
            "<pasted_content id=\"1\">\nlog\n</pasted_content id=\"1\">\nshotsを開いて"
        ));
    }

    /// The board shown past `shots open`: the browser pane on its page, a
    /// shell handing its page or a picture to a viewer. Reading its files
    /// shows nothing.
    #[test]
    fn the_board_is_shown_through_shots_open_alone() {
        for url in [
            "file:///C:/Users/wrongwrong/IdeaProjects/platitude-gg/.shots/index.html",
            "file:///C:/x/.shots/img/a.png",
        ] {
            assert!(names_the_board(url), "{url}");
        }
        for command in [
            "start ../../../.shots/index.html",
            "cmd /c start C:\\x\\.shots\\index.html",
            "Invoke-Item C:/x/.shots/index.html",
            "Start-Process msedge \"C:/x/.shots/index.html\"",
            "explorer.exe C:\\x\\.shots\\img\\a.png",
            "cargo xtask shots path && start ../../../.shots/index.html",
        ] {
            assert!(views_the_board(command), "{command}");
        }
        for command in [
            "ls .shots/runs | head -20",
            "grep -o 'data-label=\"[^\"]*\"' ../../../.shots/index.html | head -30",
            "sed -e 's#<head>#<head>#' ../../../.shots/index.html > target/board-probe/v2.html",
            "cargo xtask shots open",
            "start target/report.html",
        ] {
            assert!(!views_the_board(command), "{command}");
        }
    }

    /// One ask, one window: the open spends it, the next message of the
    /// user's that does not ask takes it back, and a task's end in between
    /// leaves it as it stands.
    #[test]
    fn an_ask_opens_the_window_once() {
        let yard = crate::yard::Yard::new("board-permit");
        let repo = crate::seats::slashed(yard.as_ref());
        assert!(crate::subprocess::git_query(&repo, &["init", "-q"]).is_some());
        let input = format!(r#"{{"session_id":"s1","cwd":"{repo}"}}"#);
        assert!(may_open(&repo, "s1").is_err(), "nothing asked yet");

        prompt_submit(&input, "shotsを開いて");
        prompt_submit(&input, "<task-notification>done</task-notification>");
        assert!(may_open(&repo, "s2").is_err(), "another session's ask");
        assert!(may_open(&repo, "s1").is_ok());
        opened(&repo, "s1");
        assert!(may_open(&repo, "s1").is_err(), "spent by the open");

        prompt_submit(&input, "board開いて");
        assert!(may_open(&repo, "s1").is_ok());
        prompt_submit(&input, "shotsへ足して");
        assert!(
            may_open(&repo, "s1").is_err(),
            "taken back by the next message"
        );
    }
}
