//! Where this session's chips are written down, and what a line of them
//! says.

use std::path::{Path, PathBuf};

use super::Chip;
use crate::hook::payload::string_field;

/// Field separator inside a ledger line. Every value written is stripped
/// of tabs and newlines, so the format has no escape rules to get wrong
/// — xtask carries no JSON library (CLAUDE.md 技術スタック).
const SEP: char = '\t';

/// SessionEnd: the ledger goes with the session that wrote it.
pub(crate) fn session_end(input: &str) {
    let Some(path) = ledger_path(input) else {
        return;
    };
    if let Err(_unheard) = std::fs::remove_file(&path) {
        // Never written, or already gone. Nobody is left to tell.
    }
}

/// This session's ledger: `.chips/<session>.tsv` beside the primary
/// checkout's `.git`, so every seat writes to one directory (as the shot
/// board does) while each session keeps a file of its own — the set a
/// session must keep readable is the set it stacked.
fn ledger_path(input: &str) -> Option<PathBuf> {
    let session: String = string_field(input, "session_id")?
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if session.is_empty() {
        return None;
    }
    let cwd = string_field(input, "cwd")?;
    let common = crate::subprocess::git_query(
        &cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let root = Path::new(&common).parent()?;
    Some(root.join(".chips").join(format!("{session}.tsv")))
}

pub(super) fn load(input: &str) -> Vec<Chip> {
    let Some(path) = ledger_path(input) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines().filter_map(parse_chip).collect()
}

fn parse_chip(line: &str) -> Option<Chip> {
    let mut fields = line.split(SEP);
    let id = fields.next()?.to_string();
    let priority = fields.next()?.parse().ok()?;
    let body = fields.next()?.to_string();
    let targets = fields
        .next()
        .unwrap_or_default()
        .split(' ')
        .filter(|target| !target.is_empty())
        .map(str::to_string)
        .collect();
    // Written last, so a ledger from before the mark existed reads as a
    // set of chips that recommend work — which is what they were.
    let optional = fields.next() == Some("1");
    Some(Chip {
        id,
        priority,
        body,
        optional,
        targets,
    })
}

/// Writes the ledger back. It is advisory: a session whose ledger cannot
/// be written keeps working, and the guard simply sees no chips.
pub(super) fn store(input: &str, live: &[Chip]) {
    let Some(path) = ledger_path(input) else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let text: String = live
        .iter()
        .map(|chip| {
            format!(
                "{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}\n",
                one_line(&chip.id),
                chip.priority,
                one_line(&chip.body),
                chip.targets
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<&str>>()
                    .join(" "),
                usize::from(chip.optional)
            )
        })
        .collect();
    if let Err(_unheard) = std::fs::write(&path, text) {
        // Nobody to tell from inside a hook; the guard falls back to
        // judging whatever the file still holds.
    }
}

fn one_line(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}
