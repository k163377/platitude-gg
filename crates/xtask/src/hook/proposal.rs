//! A fix that changes what the user sees or does is proposed with its
//! pictures: the problem and the fix, read abreast on the board
//! (.claude/rules/proposals.md). Whether a text proposes such a fix is
//! read off its words — a sentence that names something on the screen and
//! offers, recommends or asks — so a proposal worded otherwise passes: the
//! net holds the common shape, and the rule holds either way. A sentence
//! that only names the screen in passing is caught too, which is why the
//! reminder is said once and the session judges it.
//!
//! Judged where a proposal reaches the user: the closing message (Stop),
//! the wrap-up box's question and AskUserQuestion (PreToolUse). A run read
//! abreast that the session's seat put up after the user's latest message
//! answers it. A seat's pictures from an earlier message do not — the words
//! cannot tell the proposal they show from a new one — so the reminder says
//! that pointing at them is enough when they are this proposal's. Said once
//! per message, across the three: the session that reads its own text as
//! no change to the screen goes on as it stands, asked no more.
//!
//! A proposal written into a P<n>-確認事項 instead of being put to the user
//! is the fourth way out, judged on the lines a Write or Edit adds there,
//! once per message of its own: being told at the closing is no leave to
//! park the proposal.
//!
//! While the user's `/land` stands open, the reminder adds that the landing
//! stops and the user is asked again — pictures put up then hold the
//! landing anyway (`land::approval`), which is the point. After a landing
//! the seat is no longer the session's, and the reminder says to take one.

use std::path::Path;

use super::language::{WRAP_UP, without_link_targets};
use super::payload::{array_items, deny, own_string_field, own_value_field};
use super::permit;

/// The ledger's kind (`.permits/<session>.proposal`) for the three ways to
/// the user: the user's message the reminder was said for, on the board's
/// clock.
const KIND: &str = "proposal";

/// The same for the fourth, the record of what is still open.
const RECORD_KIND: &str = "proposal-record";

/// The records of what is still open: `P<n>-確認事項.md`, by its name.
const RECORD: (&str, &str) = ("P", "-確認事項.md");

/// The tool a session puts a question to the user with.
const ASK_USER: &str = "AskUserQuestion";

/// What is on the screen, or what the hand does there. `印` stands in
/// `矢印` too, which is the screen's as well.
const SCREEN: [&str; 53] = [
    "画面",
    "表示",
    "見た目",
    "見え方",
    "見栄え",
    "意匠",
    "デザイン",
    "ボタン",
    "メニュー",
    "カード",
    "ダイアログ",
    "ツールチップ",
    "パネル",
    "ペイン",
    "帯",
    "チップ",
    "タブ",
    "押下",
    "押す",
    "押し",
    "押せ",
    "クリック",
    "ホバー",
    "hover",
    "色",
    "文言",
    "文字",
    "字幕",
    "書体",
    "アイコン",
    "レイアウト",
    "配置",
    "余白",
    "幅",
    "窓",
    "スクロール",
    "強調",
    "下線",
    "枠",
    "見出し",
    "挙動",
    "操作",
    "入力",
    "選択",
    "フォーカス",
    "カーソル",
    "ポインタ",
    "アニメーション",
    "動き",
    "グラフ",
    "印",
    "UI",
    "UX",
];

/// An option put forward, a recommendation, or a question put. Left out:
/// `判断を`, `決めて`, `選んで`, `どちら` — they stand in reports of what was
/// decided as often as in a question, and a question carries `ますか` or a
/// question mark anyway.
const OFFERS: [&str; 11] = [
    "案",
    "候補",
    "推します",
    "推奨",
    "にするか",
    "ますか",
    "ませんか",
    "ですか",
    "でしょうか",
    "？",
    "?",
];

/// Words that hold a word of [`SCREEN`] or [`OFFERS`] and mean something
/// else: the app's own cards and chips (a task put up for later), a real
/// window raised for a measurement, printed output, a guide, a string, the
/// options of a question, a span of time. Taken out before either list is
/// read. `操作` stays though it names git's operations too: the two cannot
/// be told apart by the word.
const ELSEWHERE: [&str; 14] = [
    "提案カード",
    "チップに積",
    "チップを積",
    "チップへ積",
    "実窓",
    "印字",
    "印象",
    "案内",
    "案外",
    "文字列",
    "選択肢",
    "大幅",
    "幅広",
    "時間帯",
];

/// What ends a sentence. The full stop is dropped; a question or an
/// exclamation mark stays with the sentence it ends, as [`OFFERS`] reads it.
const ENDS: [char; 5] = ['。', '？', '?', '！', '!'];

/// How much of the sentence the reminder quotes back, in characters.
const EXCERPT: usize = 60;

/// Where the proposal was read, which decides how the session goes on.
#[derive(Clone, Copy)]
enum Place {
    Closing,
    Question,
    Record,
}

/// Why nothing on the board answers the proposal.
enum Unshown {
    /// The session's seat put no run up abreast — since the user's latest
    /// message, or ever when no message of the user's is on record (a side
    /// session before its user has spoken).
    Seat {
        seat: &'static str,
        since_a_message: bool,
    },
    /// The session stands in a seat it holds no claim to — its landing
    /// handed the seat back — and the board takes nothing from there.
    Unclaimed(&'static str),
    /// The session sits in no seat, and the board takes pictures from
    /// seats alone.
    NoSeat,
}

/// The reminder owed: what the board lacks, and whether the user's
/// `/land` stands open.
struct Owed {
    unshown: Unshown,
    landing: bool,
}

/// Stop: the reason to go on when the closing message proposes a change
/// to the screen with nothing on the board to show it. A turn going on
/// because a Stop hook said so is not judged again.
pub(super) fn stop(input: &str) -> Option<String> {
    if own_value_field(input, "stop_hook_active") == Some("true") {
        return None;
    }
    let text = own_string_field(input, "last_assistant_message")?;
    let sentence = sentences(&text).into_iter().find(|s| offers_a_change(s))?;
    let owed = say_once(input, KIND)?;
    Some(reminder(Place::Closing, &sentence, &owed))
}

/// PreToolUse(Write|Edit): the refusal of lines written into a
/// P<n>-確認事項 that propose a change to the screen with nothing on the
/// board to show it. `path` is the payload's, in forward slashes. Only
/// the lines the record does not hold yet are read, for a Write and an Edit
/// alike — the record is full of proposals the user already put on hold,
/// and an entry moved to another section is no new one. A subagent's write
/// is the session's to answer for, and is not judged.
pub(super) fn pre_write(input: &str, path: &str) -> Option<String> {
    if own_value_field(input, "agent_id").is_some() {
        return None;
    }
    let name = path.rsplit('/').next()?;
    let (head, tail) = RECORD;
    let number = name.strip_prefix(head)?.strip_suffix(tail)?;
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let asked = own_value_field(input, "tool_input").unwrap_or_default();
    let written = match own_string_field(input, "tool_name").as_deref() {
        Some("Edit") => own_string_field(asked, "new_string")?,
        Some("Write") => own_string_field(asked, "content")?,
        _ => return None,
    };
    let standing = std::fs::read_to_string(path).unwrap_or_default();
    let kept: std::collections::HashSet<&str> = standing.lines().collect();
    let added = written
        .lines()
        .filter(|line| !kept.contains(line))
        .collect::<Vec<_>>()
        .join("\n");
    let sentence = sentences(&added).into_iter().find(|s| offers_a_change(s))?;
    let owed = say_once(input, RECORD_KIND)?;
    Some(reminder(Place::Record, &sentence, &owed))
}

/// PreToolUse: the wrap-up box's question, or AskUserQuestion's, refused
/// when it proposes a change to the screen with nothing on the board to
/// show it. A question is read whole with the answers it offers — `どちら
/// にしますか` names the screen only in its buttons — and past its code, as
/// a closing message is. Answers whether it refused.
pub(super) fn pre_tool(input: &str) -> bool {
    if own_value_field(input, "agent_id").is_some() {
        return false;
    }
    let asked = own_value_field(input, "tool_input").unwrap_or_default();
    let questions = match own_string_field(input, "tool_name").as_deref() {
        Some(WRAP_UP) => box_question(asked),
        Some(ASK_USER) => questions(asked),
        _ => return false,
    };
    let Some(question) = questions
        .into_iter()
        .map(|question| sentences(&question).join(" "))
        .find(|question| offers_a_change(question))
    else {
        return false;
    };
    let Some(owed) = say_once(input, KIND) else {
        return false;
    };
    deny(&reminder(Place::Question, &question, &owed));
    true
}

/// The wrap-up box's question with its buttons, as one: empty when the
/// box asks nothing.
fn box_question(asked: &str) -> Vec<String> {
    let Some(ask) = own_string_field(asked, "ask") else {
        return Vec::new();
    };
    let buttons = own_value_field(asked, "actions")
        .map(array_items)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|button| own_string_field(button, "label"));
    vec![
        std::iter::once(ask)
            .chain(buttons)
            .collect::<Vec<_>>()
            .join(" / "),
    ]
}

/// Each of AskUserQuestion's questions, with its options' names and
/// descriptions.
fn questions(asked: &str) -> Vec<String> {
    let Some(questions) = own_value_field(asked, "questions") else {
        return Vec::new();
    };
    array_items(questions)
        .into_iter()
        .map(|question| {
            let options = own_value_field(question, "options")
                .map(array_items)
                .unwrap_or_default();
            own_string_field(question, "question")
                .into_iter()
                .chain(options.into_iter().flat_map(|option| {
                    ["label", "description"]
                        .into_iter()
                        .filter_map(move |field| own_string_field(option, field))
                }))
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .collect()
}

/// Whether `sentence` names the screen and puts something forward.
fn offers_a_change(sentence: &str) -> bool {
    let read = ELSEWHERE
        .iter()
        .fold(sentence.to_string(), |read, other| read.replace(other, " "));
    SCREEN.iter().any(|word| read.contains(word)) && OFFERS.iter().any(|word| read.contains(word))
}

/// The message's sentences, past what is code, a place or someone else's
/// words: fenced blocks, inline code, link targets, ASCII words that hold
/// a slash (paths, URLs), and quoted lines — the user's words quoted back
/// propose nothing.
fn sentences(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        let head = line.trim_start();
        if head.starts_with("```") {
            fenced.toggle();
            continue;
        }
        if fenced || head.starts_with('>') {
            continue;
        }
        let unquoted = line.split('`').step_by(2).collect::<Vec<_>>().join(" ");
        let prose = without_link_targets(&unquoted)
            .split_whitespace()
            .filter(|word| !(word.is_ascii() && word.contains(['/', '\\'])))
            .collect::<Vec<_>>()
            .join(" ");
        let mut sentence = String::new();
        for c in prose.chars() {
            if c != '。' {
                sentence.push(c);
            }
            if ENDS.contains(&c) {
                found.push(std::mem::take(&mut sentence));
            }
        }
        found.push(sentence);
    }
    found
        .into_iter()
        .map(|sentence| sentence.trim().to_string())
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

/// Whether the reminder is owed, marking it said in the ledger of `kind`
/// when it is: `None` when a run abreast from the session's seat answers
/// the user's latest message, or the reminder was said for that message
/// already.
fn say_once(input: &str, kind: &str) -> Option<Owed> {
    let cwd = own_string_field(input, "cwd")?;
    let session = own_string_field(input, "session_id")?;
    let asked = permit::asked(&cwd, &session).map(|asked| asked.millis());
    // No message on record reads as the board's whole life: once ever.
    let since = asked.unwrap_or(0);
    let ledger = permit::kept_file(&cwd, &session, kind)?;
    if said_for(&ledger) == Some(since) {
        return None;
    }
    // The claim first: the board names a run's seat, not its session, and a
    // pair another session put up from the letter since says nothing here.
    let unshown = match crate::seats::roster_letter(&cwd) {
        Some(seat) if !crate::seats::holds(&cwd, &session) => Unshown::Unclaimed(seat),
        Some(seat) if !crate::shots::compared_since(&cwd, seat, since).is_empty() => return None,
        Some(seat) => Unshown::Seat {
            seat,
            since_a_message: asked.is_some(),
        },
        None => Unshown::NoSeat,
    };
    if let Err(_unheard) = permit::write_whole(&ledger, &format!("{since}\n")) {
        // Nobody to tell from inside a hook; the next exit says it again.
    }
    // A landing is stopped only from the seat it lands.
    let landing = matches!(unshown, Unshown::Seat { .. }) && permit::is_open(&cwd, &session);
    Some(Owed { unshown, landing })
}

fn said_for(ledger: &Path) -> Option<u128> {
    std::fs::read_to_string(ledger).ok()?.trim().parse().ok()
}

fn reminder(place: Place, sentence: &str, owed: &Owed) -> String {
    let (said, then, unasked) = match place {
        Place::Closing => (
            "締めの文",
            "載せたら締め直す。画面の見え方・操作を変えない話なら、この件で書き足す物は無い\
             (締めはもう届いている)",
            "締めと問い",
        ),
        Place::Question => (
            "この問い",
            "載せたら問い直す。画面の見え方・操作を変えない話なら、同じ問いをそのまま出し直してよい",
            "締めと問い",
        ),
        Place::Record => (
            "確認事項へ書く行",
            "P<n>-確認事項 に書くのは、絵で示して訊いた上でユーザーが保留と決めた物だけ — 書いて終えずに、\
             載せて訊く。そう決まった物か、画面の見え方・操作を変えない話なら、同じ書き込みをそのまま撃ち直\
             してよい",
            "確認事項への書き込み",
        ),
    };
    let landing = if owed.landing {
        format!(
            "`/land` の後に見つかった UX の課題なら、反映する変更と関係が無くても反映を止めて訊き直す — \
             先に `{}` で列から外れる。",
            crate::land::LEAVE.line()
        )
    } else {
        String::new()
    };
    let take = crate::seats::commands::TAKE.line();
    let missing = match &owed.unshown {
        Unshown::Seat {
            seat,
            since_a_message: true,
        } => {
            format!(
                "席 {seat} はユーザーの最新の発言の後に、横並びの run を board へ載せていない(前の発言の時に\
                 載せた絵がこの提案の物なら、そのラベルで指せば足りる)"
            )
        }
        Unshown::Seat {
            seat,
            since_a_message: false,
        } => format!("席 {seat} は横並びの run を board へ載せていない"),
        Unshown::Unclaimed(seat) => format!(
            "席 {seat} の claim はこのセッションの物でない(land で外れた後・他のセッションの席)ので、この席からは \
             board に載らない — 新しい提案として `{take}` で席を取り、そこで載せる"
        ),
        Unshown::NoSeat => format!(
            "このセッションは席に居ないので board に絵が無い(絵は席からしか載らない — `{take}` で取る)"
        ),
    };
    format!(
        "{said}は UX を変える修正の提案に読める(「{}」)が、{missing}。{landing}UX を変える提案は、問題点(今の画面)を \
         before、修正案(席に実装した有るべき形。作れない場面は代用の絵で、代用とラベルに書く)を after にした 1 run\
         (`shots add --before … --after …`。after の案が複数なら `--part 今 … --part A … --part B …`)で board に\
         載せ、本文は件をラベルの綴りで、案を A, B で指して出す(.claude/rules/proposals.md)。{then} — この発言の間、\
         {unasked}ではもう訊かない。",
        excerpt(sentence)
    )
}

fn excerpt(sentence: &str) -> String {
    let mut cut: String = sentence.chars().take(EXCERPT).collect();
    if sentence.chars().count() > EXCERPT {
        cut.push('…');
    }
    cut
}

#[cfg(test)]
mod tests {
    use super::super::payload::own_value_field;
    use super::{
        box_question, offers_a_change, permit, pre_tool, pre_write, questions, sentences, stop,
    };

    fn proposes(text: &str) -> bool {
        sentences(text)
            .iter()
            .any(|sentence| offers_a_change(sentence))
    }

    /// Shapes the sessions' own proposals took: an option put forward, a
    /// recommendation, a question — each naming the screen.
    #[test]
    fn a_sentence_that_puts_forward_a_change_to_the_screen_proposes_one() {
        for text in [
            "暗い行はメニューなしにし、破線のコミット行は「追加するだけ」の操作に絞る案を提案します。",
            "待ちの文言はどちらにしますか?",
            "名前の字の余白も数えて印を中央に置く案を、絵で作りますか？",
            "- **推奨: 中央・UI なし**(設定カードだけで完結する意匠を崩さない)",
            "矢印キーで行を移った時もノードを見える所へ寄せますか?",
            "- 左メニューだけにするか",
            "gate は PASS した。\n\n残りは 1 つで、帯の右端のボタンを左へ寄せる案を推します。",
        ] {
            assert!(proposes(text), "{text}");
        }
    }

    /// A report of the screen changed, a question about anything else, the
    /// harness's own cards and chips, words that only hold a screen's word,
    /// and what is code, a place or the user's words quoted back.
    #[test]
    fn a_report_or_a_question_about_anything_else_proposes_nothing() {
        for text in [
            "席 c で直しを commit し、gate が PASS した。",
            "ボタンの色を黄に直した。帯の右端も揃えた。",
            "commit しますか?",
            "gate を回しますか？ 表示の変更は無いので省けます。",
            "gate を回しますか? 表示の変更は無いので省けます。",
            "チップに積みますか?",
            "チップを積む案で進めますか?",
            "チップへ積む案。",
            "画面の案内を読んだ。",
            "表示が案外早い。",
            "幅広い候補を読んだ。",
            "上の提案カードで hook を直すかのどちらかになります。",
            "実窓を立てて main と perf の A/B を撃ってよいですか？",
            "印字されたパスで続けますか?",
            "その印象で進めますか?",
            "どの選択肢で進めますか?",
            "文字列の扱いはどうしますか?",
            "大幅に減らす案で進めますか?",
            "時間帯の案内を読みますか?",
            "> ボタンの色を変えますか？\n\n上の問いには答えた。",
            "`ボタン` を消す案で進めますか?",
            "```\nボタンの色を変えますか？\n```\n直した。",
            "[節](https://example.com/a?q=ボタン) を読みました。",
            "src/UI/hover?.qml を読んだ。",
        ] {
            assert!(!proposes(text), "{text}");
        }
    }

    /// A question names the screen in its answers as often as in itself.
    #[test]
    fn a_question_is_read_with_the_answers_it_offers() {
        let asked = |payload: &str| {
            own_value_field(payload, "tool_input")
                .unwrap_or_default()
                .to_string()
        };
        let boxed = asked(
            r#"{"tool_input":{"recap":"r","ask":"どちらにしますか？","actions":[{"label":"ボタンを左へ"},{"label":"今のまま"}]}}"#,
        );
        let read = box_question(&boxed);
        assert_eq!(read, ["どちらにしますか？ / ボタンを左へ / 今のまま"]);
        assert!(offers_a_change(&read[0]));
        let unasked = asked(r#"{"tool_input":{"recap":"ボタンの色を直した。どちらにしますか？"}}"#);
        assert!(
            box_question(&unasked).is_empty(),
            "a box with no question asks nothing"
        );

        let put = asked(
            r#"{"tool_input":{"questions":[{"question":"畳む入口はどれにしますか","header":"入口","options":[{"label":"帯の席","description":"フィルタ帯の右端に置く"},{"label":"今のまま","description":"変えない"}],"multiSelect":false},{"question":"commit しますか","header":"c","options":[{"label":"はい","description":"する"}],"multiSelect":false}]}}"#,
        );
        let read = questions(&put);
        assert_eq!(
            read,
            [
                "畳む入口はどれにしますか / 帯の席 / フィルタ帯の右端に置く / 今のまま / 変えない",
                "commit しますか / はい / する"
            ]
        );
        assert!(offers_a_change(&read[0]));
        assert!(!offers_a_change(&read[1]));
    }

    /// A repository with seat `d`'s and seat `e`'s directories, as the
    /// roster names them, the board beside its `.git`, and — when a session
    /// is named — that session's claim on both (they share the repository's
    /// git directory, where the claim is read).
    struct Board {
        _yard: crate::yard::Yard,
        repo: String,
    }

    impl Board {
        fn new(what: &str, claimed_by: Option<&str>) -> Self {
            let yard = crate::yard::Yard::new(what);
            let repo = crate::seats::slashed(yard.as_ref());
            assert!(crate::subprocess::git_query(&repo, &["init", "-q"]).is_some());
            for seat in ["d", "e"] {
                std::fs::create_dir_all(format!("{repo}/.claude/worktrees/{seat}"))
                    .expect("a seat's directory");
            }
            std::fs::create_dir_all(format!("{repo}/.shots/runs")).expect("the board's runs");
            if let Some(session) = claimed_by {
                let claim = crate::seats::Identity::current(Some(session)).reason();
                std::fs::write(format!("{repo}/.git/locked"), format!("{claim}\n"))
                    .expect("the claim");
            }
            Self { _yard: yard, repo }
        }

        fn seat(&self, letter: &str) -> String {
            format!("{}/.claude/worktrees/{letter}", self.repo)
        }

        /// A run of seat `d`'s, as the board writes one.
        fn put_up(&self, at: u64, pictures: &[&str], abreast: bool) {
            let mut run = format!("label\t1. 帯のボタン\nseat\td\nat\t{at}\n");
            if abreast {
                run.push_str("abreast\t1\n");
            }
            for (n, caption) in pictures.iter().enumerate() {
                run.push_str(&format!("shot\timg/{n}.png\tapp.png\t1\t1\t{caption}\n"));
            }
            std::fs::write(format!("{}/.shots/runs/{at}-d-x.tsv", self.repo), run)
                .expect("the run is written");
        }

        fn asked(&self, session: &str, words: &str, at: u64) {
            permit::record_prompt_at(&self.repo, session, words, at).expect("recorded");
        }
    }

    fn closing(session: &str, cwd: &str, active: bool) -> String {
        format!(
            r#"{{"session_id":"{session}","cwd":"{cwd}","hook_event_name":"Stop","stop_hook_active":{active},"last_assistant_message":"gate は PASS した。帯のボタンを左へ寄せる案を推します。"}}"#
        )
    }

    fn question(session: &str, cwd: &str, tool: &str, extra: &str) -> String {
        format!(
            r#"{{"session_id":"{session}","cwd":"{cwd}","hook_event_name":"PreToolUse","tool_name":"{tool}","tool_input":{{"ask":"ボタンの色を変えますか？"}}{extra}}}"#
        )
    }

    /// One reminder per message of the user's, wherever the session sits:
    /// a seat is named with its pictures missing, no seat with the way to
    /// one. A turn going on because a Stop hook said so is not judged.
    #[test]
    fn the_reminder_is_said_once_for_each_of_the_users_messages() {
        let board = Board::new("proposal-once", Some("s1"));
        let seat = board.seat("d");
        board.asked("s1", "帯を直して", 1_000_000);
        let first = stop(&closing("s1", &seat, false)).expect("nothing on the board answers it");
        assert!(
            first.contains("席 d はユーザーの最新の発言の後に"),
            "{first}"
        );
        assert!(
            first.contains("「帯のボタンを左へ寄せる案を推します」"),
            "{first}"
        );
        assert!(
            !first.contains("land leave"),
            "no landing stands open: {first}"
        );
        assert_eq!(
            stop(&closing("s1", &seat, false)),
            None,
            "said for this message already"
        );

        board.asked("s1", "では直して", 2_000_000);
        assert_eq!(stop(&closing("s1", &seat, true)), None, "a turn going on");
        let unseated =
            stop(&closing("s1", &board.repo, false)).expect("said again for the next message");
        assert!(unseated.contains("cargo xtask seat"), "{unseated}");
    }

    /// The question is refused where it is asked, and the closing after it
    /// goes free: once across the three ways to the user. Only the
    /// session's own question, and only the two tools that put one.
    #[test]
    fn a_question_is_refused_once_and_the_closing_after_it_goes_free() {
        let board = Board::new("proposal-question", Some("s1"));
        let seat = board.seat("d");
        board.asked("s1", "帯を直して", 1_000_000);
        let wrap_up = super::WRAP_UP;
        assert!(!pre_tool(&question("s1", &seat, "Bash", "")));
        assert!(!pre_tool(&question(
            "s1",
            &seat,
            wrap_up,
            r#","agent_id":"a1""#
        )));
        assert!(pre_tool(&question("s1", &seat, wrap_up, "")));
        assert!(!pre_tool(&question("s1", &seat, wrap_up, "")), "said once");
        assert_eq!(stop(&closing("s1", &seat, false)), None, "said at the box");

        board.asked("s1", "入口を直して", 2_000_000);
        let put = format!(
            r#"{{"session_id":"s1","cwd":"{seat}","hook_event_name":"PreToolUse","tool_name":"AskUserQuestion","tool_input":{{"questions":[{{"question":"畳む入口はどれにしますか","header":"入口","options":[{{"label":"帯の席","description":"フィルタ帯の右端"}}],"multiSelect":false}}]}}}}"#
        );
        assert!(pre_tool(&put), "AskUserQuestion is the other way to ask");
    }

    /// What answers the proposal: the seat's pictures abreast since the
    /// user's latest message — not a lone picture, not a pair from before
    /// it.
    #[test]
    fn the_seats_pair_since_the_message_answers_it() {
        let board = Board::new("proposal-pair", Some("s1"));
        let seat = board.seat("d");
        board.put_up(4_000_000, &["before", "after"], true);
        board.asked("s1", "帯を直して", 5_000_000);
        let earlier = stop(&closing("s1", &seat, false)).expect("a pair from before the message");
        assert!(earlier.contains("前の発言の時に"), "{earlier}");

        board.put_up(6_000_000, &[""], false);
        board.asked("s1", "もう一度", 5_500_000);
        assert!(
            stop(&closing("s1", &seat, false)).is_some(),
            "a lone picture"
        );

        board.put_up(7_000_000, &["before", "after"], true);
        board.asked("s1", "さらに", 6_500_000);
        assert_eq!(stop(&closing("s1", &seat, false)), None);
    }

    /// With no message of the user's on record, any pair of the seat's
    /// answers, and the reminder speaks of no message.
    #[test]
    fn with_no_message_on_record_the_seats_pairs_ever_answer() {
        let board = Board::new("proposal-unasked", Some("s1"));
        board.put_up(7_000_000, &["before", "after"], true);
        assert_eq!(stop(&closing("s1", &board.seat("d"), false)), None);
        let empty = stop(&closing("s1", &board.seat("e"), false)).expect("seat e has none");
        assert!(empty.contains("席 e は横並びの run"), "{empty}");
        assert!(!empty.contains("最新の発言"), "{empty}");
    }

    /// A proposal found after `/land` stops the landing and asks again; one
    /// read from a seat the session does not hold — its landing handed it
    /// back — is a new proposal, put up from a seat taken anew, and speaks
    /// of no landing to stop. The claim is read before the board: a pair
    /// another session put up from the letter answers nothing here.
    #[test]
    fn an_open_landing_stops_and_a_landed_seat_is_taken_anew() {
        let landing = Board::new("proposal-landing", Some("s1"));
        landing.asked("s1", "/land", 1_000_000);
        let stops = stop(&closing("s1", &landing.seat("d"), false)).expect("a landing open");
        assert!(stops.contains("反映を止めて訊き直す"), "{stops}");
        assert!(stops.contains(&crate::land::LEAVE.line()), "{stops}");

        let landed = Board::new("proposal-landed", None);
        landed.put_up(2_000_000, &["before", "after"], true);
        landed.asked("s1", "/land", 1_000_000);
        let anew = stop(&closing("s1", &landed.seat("d"), false)).expect("a seat handed back");
        assert!(anew.contains("claim はこのセッションの物でない"), "{anew}");
        assert!(anew.contains("cargo xtask seat"), "{anew}");
        assert!(!anew.contains("反映を止めて"), "{anew}");
    }

    fn record_edit(cwd: &str, file: &str, new: &str, extra: &str) -> String {
        format!(
            r#"{{"session_id":"s1","cwd":"{cwd}","hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{{"file_path":"{file}","old_string":"要判断","new_string":"{new}"}}{extra}}}"#
        )
    }

    fn record_write(cwd: &str, file: &str, content: &str) -> String {
        format!(
            r#"{{"session_id":"s1","cwd":"{cwd}","hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{{"file_path":"{file}","content":"{content}"}}}}"#
        )
    }

    /// A proposal parked in a P<n>-確認事項 is refused once per message —
    /// on its own ledger, so a reminder at the closing is no leave to park
    /// it. Only the lines the record does not hold yet are read, by a Write
    /// and an Edit alike; only the records, and only the session's own.
    #[test]
    fn a_proposal_parked_in_the_record_is_refused_once() {
        let board = Board::new("proposal-record", Some("s1"));
        let seat = board.seat("d");
        std::fs::create_dir_all(format!("{seat}/internal-docs")).expect("the docs");
        let p3 = format!("{seat}/internal-docs/P3-確認事項.md");
        let held = "## 要判断\n- 帯のボタンを左へ寄せる案\n";
        std::fs::write(&p3, held).expect("the record");
        board.asked("s1", "帯を直して", 1_000_000);
        assert!(
            stop(&closing("s1", &seat, false)).is_some(),
            "the closing's own"
        );

        let parked = "## 要判断\\n- 窓の幅を広げる案";
        for elsewhere in ["デザイン規約.md", "P-確認事項.md"] {
            let other = format!("{seat}/internal-docs/{elsewhere}");
            assert_eq!(
                pre_write(&record_edit(&seat, &other, parked, ""), &other),
                None,
                "{elsewhere}"
            );
        }
        let moved = "## 要判断\\n\\n## 移した先\\n- 帯のボタンを左へ寄せる案";
        assert_eq!(
            pre_write(&record_edit(&seat, &p3, moved, ""), &p3),
            None,
            "an entry the record holds, moved"
        );
        assert_eq!(
            pre_write(&record_edit(&seat, &p3, parked, r#","agent_id":"a1""#), &p3),
            None,
            "a subagent's write"
        );
        let refused =
            pre_write(&record_edit(&seat, &p3, parked, ""), &p3).expect("a proposal parked");
        assert!(refused.contains("確認事項へ書く行"), "{refused}");
        assert!(
            refused.contains("確認事項への書き込みではもう訊かない"),
            "{refused}"
        );
        assert_eq!(
            pre_write(&record_edit(&seat, &p3, parked, ""), &p3),
            None,
            "said once"
        );

        board.asked("s1", "では書いて", 2_000_000);
        let p5 = format!("{seat}/internal-docs/P5-確認事項.md");
        std::fs::write(&p5, held).expect("the record");
        let whole = |added: &str| format!("## 要判断\\n- 帯のボタンを左へ寄せる案\\n{added}\\n");
        assert_eq!(
            pre_write(&record_write(&seat, &p5, &whole("- 直した")), &p5),
            None,
            "the whole file written, a line added that proposes nothing"
        );
        let rewritten = pre_write(
            &record_write(&seat, &p5, &whole("- メニューの行を詰める案")),
            &p5,
        )
        .expect("the whole file written with a proposal added");
        assert!(rewritten.contains("メニューの行を詰める案"), "{rewritten}");
    }
}
