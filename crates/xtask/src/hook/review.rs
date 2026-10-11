//! UserPromptSubmit: what a review is judged by, added to the user's ask
//! for one. The points are the review-points skill's, named by path
//! because a review run by another recipe (`/code-review`) need not load
//! the skill. The instructions are every one the user gave in the
//! session, met exactly once; a summarized context drops those turns
//! first, so the note names the session record.

use std::path::PathBuf;

use super::payload::string_field;

/// The words that ask for a review, matched case-insensitively anywhere
/// in the prompt (`/code-review` included).
const ASKS: [&str; 2] = ["レビュー", "review"];

/// Everyday words that contain an ask. Japanese has no word boundary to
/// test, so these are counted and subtracted.
const CONTAINERS: [&str; 2] = ["プレビュー", "preview"];

/// The review-points skill, from the workspace root.
const SKILL: &str = ".claude/skills/review-points/SKILL.md";

/// UserPromptSubmit: plain stdout becomes this turn's context. The
/// landing permit reads the same message first (`permit`).
pub(super) fn prompt_submit(input: &str) -> Result<(), String> {
    let Some(prompt) = string_field(input, "prompt") else {
        return Ok(());
    };
    super::permit::prompt_submit(input, &prompt);
    if answers(&prompt) {
        println!(
            "{}",
            note(string_field(input, "transcript_path").as_deref())
        );
    }
    Ok(())
}

/// Whether the note answers this prompt: the user's own message asking for
/// a review. A tagged event — a background task's end, a subagent's report
/// — asks for nothing, whatever words it carries, and neither do the app's
/// notes ahead of the user's words (`permit`).
fn answers(prompt: &str) -> bool {
    super::permit::the_users_words(prompt).is_some_and(asks_for_review)
}

/// Whether this prompt asks for a review. One that only mentions a review
/// still gets the note — the cheaper of the two mistakes.
fn asks_for_review(prompt: &str) -> bool {
    let lowered = prompt.to_lowercase();
    occurrences(&lowered, &ASKS) > occurrences(&lowered, &CONTAINERS)
}

fn occurrences(text: &str, words: &[&str]) -> usize {
    words.iter().map(|word| text.matches(word).count()).sum()
}

/// The skill's full path in the tree this hook was built from: the
/// session's cwd may be a subfolder of that tree, which [`SKILL`] does
/// not reach from.
fn skill_path() -> PathBuf {
    crate::tree::workspace_root().join(SKILL)
}

/// What the review is judged by: the skill's points, of which the user's
/// instructions are the session's own to check. Both directions of the
/// instructions are named because only missing work shows itself; work
/// nobody asked for reads as diligence.
fn note(transcript: Option<&str>) -> String {
    let record = transcript
        .map(|path| {
            format!(
                " If earlier instructions are missing from context, read the relevant \
                 turns in the session record: {path}."
            )
        })
        .unwrap_or_default();
    format!(
        "Judge the change by the points of the review-points skill ({skill}), and hand that \
         path to any agent that does the reviewing. The user's instructions are this session's \
         to check: review the implementation and verification against the applicable user \
         instructions, including corrections during the task. Check for missing work and \
         unauthorized scope changes; necessary supporting edits belong to the requested work. \
         Report findings and material verification limits.{record}",
        skill = skill_path().display()
    )
}

#[cfg(test)]
mod tests {
    use super::{answers, asks_for_review, note, prompt_submit, skill_path};

    #[test]
    fn a_report_handed_back_asks_for_nothing() {
        assert!(!answers(
            "<agent-message from=\"a1\">## Review of the diff</agent-message>"
        ));
        assert!(!answers(
            "<task-notification>the review finished</task-notification>"
        ));
        assert!(answers("レビューして"));
    }

    #[test]
    fn takes_the_ask_in_either_language_and_from_a_slash_command() {
        assert!(asks_for_review("レビューして"));
        assert!(asks_for_review("/code-review high"));
        assert!(asks_for_review("Review the branch before I merge it"));
        assert!(!asks_for_review("rebase して起動"));
    }

    #[test]
    fn a_preview_is_not_a_review_in_either_language() {
        assert!(!asks_for_review("プレビューのハイライトを直して"));
        assert!(!asks_for_review("fix the preview pane"));
        assert!(asks_for_review("プレビューの変更をレビューして"));
        assert!(asks_for_review("review the preview pane"));
    }

    #[test]
    fn names_the_record_only_when_the_payload_carries_it() {
        assert!(note(Some("C:/x/session.jsonl")).contains("C:/x/session.jsonl"));
        assert!(!note(None).contains("record"));
    }

    #[test]
    fn names_a_skill_that_is_there() {
        let skill = skill_path();
        assert!(note(None).contains(&skill.display().to_string()));
        assert!(skill.is_file(), "{}", skill.display());
    }

    #[test]
    fn a_payload_without_a_prompt_is_not_a_failure() {
        assert!(prompt_submit(r#"{"session_id":"x"}"#).is_ok());
    }
}
