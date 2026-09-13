//! What one runner command is, so that the module running it can say so
//! once and every other place can quote that.
//!
//! **Declaring a command here grants nothing.** What may run is the
//! hooks' answer at the moment of the call (`hook::pre_shell`); this says
//! which line to type and what has to be true first. The rule this serves
//! is .claude/rules-refs/structure.md §コマンドの正本.

/// One thing a session may be told to run, declared beside the code that
/// runs it.
pub(crate) struct Command {
    /// What every other place calls this operation — stable, and not
    /// derived from anything below.
    pub(crate) id: &'static str,
    /// The verb and everything after it, as it is typed: no `cargo
    /// xtask`, which [`Command::line`] writes, and no escape, which
    /// [`Permission`] carries.
    pub(crate) call: &'static str,
    /// What the operation is for, in one line.
    pub(crate) purpose: &'static str,
    /// Which tree it is run from.
    pub(crate) run_in: Where,
    /// What this command itself refuses on, in the reader's order. A
    /// rule written down somewhere else is not copied here —
    /// [`Permission`] points at the judgement instead, so that the rule
    /// moves in one place.
    pub(crate) needs: &'static [&'static str],
    /// Who decides whether this session may run it.
    pub(crate) permission: Permission,
}

impl Command {
    /// The whole line, escape and runner included — what a document
    /// quotes and what a deny sentence tells a reader to type.
    pub(crate) fn line(&self) -> String {
        match self.permission {
            Permission::Escape(flag) => format!("{flag}=1 cargo xtask {}", self.call),
            _ => format!("cargo xtask {}", self.call),
        }
    }

    /// The runner verb this operation is spelled with — the first word
    /// of the call, which is what `main.rs` dispatches on.
    pub(crate) fn verb(&self) -> &'static str {
        self.call.split_whitespace().next().unwrap_or(self.call)
    }

    /// The whole contract as one sentence, for a place that has to hand
    /// a reader something runnable while refusing something else.
    pub(crate) fn instruction(&self) -> String {
        let mut text = format!(
            "`{}` — {}, from {}",
            self.line(),
            self.purpose,
            self.run_in.say()
        );
        if !self.needs.is_empty() {
            text.push_str(&format!(" (needs {})", self.needs.join("; and ")));
        }
        if let Permission::Plain = self.permission {
            return text;
        }
        text.push_str(&format!(", and it runs on {}", self.permission.say()));
        text
    }

    /// The options the call spells, so that a page describing the verb
    /// can be held to naming them.
    pub(crate) fn options(&self) -> impl Iterator<Item = &'static str> {
        self.call
            .split_whitespace()
            .filter(|word| word.starts_with("--"))
    }
}

/// Which tree a command is run from — the two the rules separate
/// (CLAUDE.md ビルド・テスト).
pub(crate) enum Where {
    /// A worktree seat — it writes into the tree it runs in.
    Seat,
    /// Either tree: it writes nothing another session's tree owns.
    Either,
}

impl Where {
    pub(crate) fn say(&self) -> &'static str {
        match self {
            Self::Seat => "a seat",
            Self::Either => "either tree",
        }
    }
}

/// Who decides whether this session may run the command — by reference,
/// never by a second copy of the condition.
pub(crate) enum Permission {
    /// Nothing beyond the pre-shell hook's ordinary reading of the line.
    Plain,
    /// The escape the pre-shell hook asks for in front, held as the
    /// hook's own const (`hook::APPROVAL_FLAGS`).
    Escape(&'static str),
    /// The permit `hook::permit` reads off the user's own messages.
    Permit,
}

impl Permission {
    pub(crate) fn say(&self) -> String {
        match self {
            Self::Plain => "the pre-shell hook's ordinary reading".to_string(),
            Self::Escape(flag) => format!("{flag}=1 in front, for an instruction that asked"),
            Self::Permit => "the user's permit for this message (hook::permit)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Permission, Where};

    const PLAIN: Command = Command {
        id: "sample.plain",
        call: "gate --host-only",
        purpose: "the daily tier",
        run_in: Where::Seat,
        needs: &[],
        permission: Permission::Plain,
    };

    const ESCAPED: Command = Command {
        id: "sample.escaped",
        call: "launch",
        purpose: "a real window",
        run_in: Where::Seat,
        needs: &[],
        permission: Permission::Escape("PGG_ALLOW_GUI"),
    };

    #[test]
    fn the_line_carries_the_runner_and_the_escape() {
        assert_eq!(PLAIN.line(), "cargo xtask gate --host-only");
        assert_eq!(ESCAPED.line(), "PGG_ALLOW_GUI=1 cargo xtask launch");
    }

    #[test]
    fn the_verb_is_the_first_word_and_the_options_are_the_dashed_ones() {
        assert_eq!(PLAIN.verb(), "gate");
        assert_eq!(ESCAPED.verb(), "launch");
        assert_eq!(PLAIN.options().collect::<Vec<_>>(), ["--host-only"]);
        assert_eq!(ESCAPED.options().count(), 0);
    }
}
