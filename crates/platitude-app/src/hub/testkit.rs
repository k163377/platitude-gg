//! A hub for tests, built bare: its runtime is driven by nobody, so
//! nothing a test asks of it ever starts.

use super::*;

/// A hub whose runtime nobody drives, numbering its tabs after
/// `next_tab_id`: the pictures are held process-wide, so each test
/// numbers its tabs apart from the others'.
pub(super) fn hub(next_tab_id: i32) -> Hub {
    Hub {
        runtime: Some(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("a runtime"),
        ),
        executor: GitExecutor::new(),
        git_program: String::new(),
        restart_wanted: false,
        tabs: HashMap::new(),
        next_tab_id,
        parked_writes: Vec::new(),
        saves: saves::Saves::default(),
        store: Store::ephemeral(),
        settings: Settings::default(),
        saved_state: State::default(),
        state: State::default(),
        held_elsewhere: String::new(),
    }
}
