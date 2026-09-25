//! UserPromptSubmit: the half of a review that is not in the diff — every
//! instruction the user gave in the session, met exactly once. A
//! summarized context drops those turns first, so the ask for a review
//! carries them back in.

use super::payload::string_field;

/// The words that ask for a review, matched case-insensitively anywhere
/// in the prompt (`/code-review` included).
const ASKS: [&str; 2] = ["レビュー", "review"];

/// Everyday words that contain an ask. Japanese has no word boundary to
/// test, so these are counted and subtracted.
const CONTAINERS: [&str; 2] = ["プレビュー", "preview"];

/// UserPromptSubmit: plain stdout becomes this turn's context. The
/// landing permit reads the same message first (`permit`).
pub(super) fn prompt_submit(input: &str) -> Result<(), String> {
    let Some(prompt) = string_field(input, "prompt") else {
        return Ok(());
    };
    super::permit::prompt_submit(input, &prompt);
    if asks_for_review(&prompt) {
        println!(
            "{}",
            note(string_field(input, "transcript_path").as_deref())
        );
    }
    Ok(())
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

/// What the review covers besides the code. Both directions are named
/// because only missing work shows itself; work nobody asked for reads as
/// diligence.
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
        "Review the implementation and verification against the applicable user instructions, \
         including corrections during the task. Check for missing work and unauthorized scope \
         changes; necessary supporting edits belong to the requested work. Report findings \
         and material verification limits.{record}"
    )
}

#[cfg(test)]
mod tests {
    use super::{asks_for_review, note, prompt_submit};

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
    fn a_payload_without_a_prompt_is_not_a_failure() {
        assert!(prompt_submit(r#"{"session_id":"x"}"#).is_ok());
    }
}
