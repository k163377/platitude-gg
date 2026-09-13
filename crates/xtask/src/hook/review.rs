//! UserPromptSubmit: the half of a review that is not in the diff.
//!
//! The code is one of the two things a review answers for. The other is
//! the session that asked for it: every instruction the user gave in it,
//! met once and not twice. That half has no diff to read, and the turns
//! it lives in are the first thing a summarized context drops, so the ask
//! for a review carries it back in.

use super::payload::string_field;

/// The words that ask for a review. Matched case-insensitively against
/// the whole prompt, so `/code-review` and a sentence with レビュー in it
/// are the same trigger.
const ASKS: [&str; 2] = ["レビュー", "review"];

/// The everyday words that carry one of those whole inside them. A diff
/// pane is called nothing else here, and Japanese writes no spaces, so
/// there is no word boundary to test for: the containers are named, and
/// a prompt asks for a review when it holds more asks than containers.
const CONTAINERS: [&str; 2] = ["プレビュー", "preview"];

/// UserPromptSubmit: plain stdout becomes this turn's context. The
/// message is also what the landing permit answers to (`permit`), so it
/// goes there first to supply the pre-landing completion check.
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

/// Whether this prompt is asking for a review at all. A prompt that only
/// mentions one pays the paragraph, which is the cheaper of the two
/// mistakes; a prompt that says プレビュー pays nothing.
fn asks_for_review(prompt: &str) -> bool {
    let lowered = prompt.to_lowercase();
    occurrences(&lowered, &ASKS) > occurrences(&lowered, &CONTAINERS)
}

/// How many times any of `words` appears in `text`.
fn occurrences(text: &str, words: &[&str]) -> usize {
    words.iter().map(|word| text.matches(word).count()).sum()
}

/// What the review covers besides the code. Both directions are spelled
/// out because only one of them is looked for on its own: work that is
/// missing announces itself the moment the user reads the result, while
/// work nobody asked for reads as diligence.
fn note(transcript: Option<&str>) -> String {
    let record = transcript
        .map(|path| {
            format!(
                " Earlier turns may have been summarized away, so read the \
                 session's record rather than recalling it: {path}."
            )
        })
        .unwrap_or_default();
    format!(
        "This prompt asks for a review, so the review covers the session \
         and not only the code: hold the work against every instruction \
         the user gave in this session, and answer in both directions — \
         what was asked for and is missing, half-applied or quietly \
         dropped (a correction sent mid-turn is an instruction like any \
         other), and what was done past the ask (files nobody named, scope \
         widened, refactors, docs or tests nobody requested).{record} Name \
         the instructions that were met as well: a review listing only \
         findings cannot be told from one that stopped looking."
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
