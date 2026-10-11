//! What git's own trace says of the programs a command ran under it — which
//! hooks, with what arguments, and how each ended. Read where git's words do
//! not name who turned a write down: a commit hook's no adds nothing of
//! git's own (builtin/commit.c), a `pre-push` hook's only `failed to push
//! some refs` (transport.c), an ssh signing program's failure is quoted as
//! that program's own words (gpg-interface.c `sign_buffer_ssh`). The screen
//! then names only what git recorded ([`GitCommand::traced`]).
//!
//! git writes trace2's event stream, one file per process, into the folder
//! `GIT_TRACE2_EVENT` names ([`folder_for`]) — every git the command started
//! included, since they inherit the variable (trace2/tr2_dst.c, the minimum
//! to 2.55). A folder, not stderr: a hook's own git inherits the variable
//! too, and a hook that reads its git's stderr would read the events. Each
//! line is one JSON object; a hook's `child_start` carries `child_class`
//! `hook` and its `hook_name` (hook.c and the minimum's own `pre-push` in
//! transport.c), and its `child_exit` the code, by `child_id` within the
//! process's `sid` (trace2/tr2_tgt_event.c).
//!
//! [`GitCommand::traced`]: super::GitCommand::traced

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The programs one command's own gits started, as their trace recorded
/// them. Empty for a command that was not traced, or whose trace could not
/// be read — which then names nothing.
///
/// Only the operation's own gits count: the one the command ran (its `sid`
/// has no `/`), and the gits it ran itself — a replay's `git commit`s —
/// told from a git a hook of its ran by when it started: git waits on a
/// hook, so one that started while a hook of the command's own ran is the
/// hook's. A git a hook runs inherits the trace, and what its own hooks or
/// signers say is the hook's business: the hook may take that no and end
/// 0. A git the command ran whose start the trace does not time is not
/// counted. Times are git's wall clock, so a clock set back while a hook
/// runs moves which side of it a start falls.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trace {
    children: Vec<Child>,
    own: HashSet<String>,
}

/// One program a git started.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Child {
    /// The `sid` of the git that started it.
    parent: String,
    /// The hook's event (`pre-push`), for a hook.
    hook: Option<String>,
    argv: Vec<String>,
    /// How it ended; `None` for one the trace never saw end.
    code: Option<i64>,
    /// When it started and ended (`time`, UTC to the microsecond, which
    /// orders as text).
    started: Option<String>,
    ended: Option<String>,
}

impl Trace {
    /// Whether one of `hooks` an own git ran ended non-zero — what turns
    /// the write it guards down (git runs no more of it).
    #[must_use]
    pub fn hook_failed(&self, hooks: &[&str]) -> bool {
        self.own_children().any(|child| {
            child
                .hook
                .as_deref()
                .is_some_and(|hook| hooks.contains(&hook))
                && child.code.is_some_and(|code| code != 0)
        })
    }

    /// Whether a program an own git ran to make an ssh signature ended
    /// non-zero: `<program> -Y sign …` (gpg-interface.c `sign_buffer_ssh`,
    /// the minimum to 2.55), whose failure git passes on as that program's
    /// words alone.
    #[must_use]
    pub fn ssh_signer_failed(&self) -> bool {
        self.own_children().any(|child| {
            child.argv.windows(2).any(|pair| pair == ["-Y", "sign"])
                && child.code.is_some_and(|code| code != 0)
        })
    }

    fn own_children(&self) -> impl Iterator<Item = &Child> {
        self.children
            .iter()
            .filter(|child| self.own.contains(&child.parent))
    }

    /// Every file git wrote into `folder`.
    pub(super) fn read(folder: &Path) -> Self {
        let Ok(files) = std::fs::read_dir(folder) else {
            return Self::default();
        };
        let mut text = String::new();
        for file in files.flatten() {
            if let Ok(bytes) = std::fs::read(file.path()) {
                text.push_str(&String::from_utf8_lossy(&bytes));
                text.push('\n');
            }
        }
        Self::parse(&text)
    }

    fn parse(text: &str) -> Self {
        let mut children: Vec<Child> = Vec::new();
        let mut at: HashMap<(String, i64), usize> = HashMap::new();
        let mut starts: Vec<(String, Option<String>)> = Vec::new();
        for line in text.lines() {
            let Some(fields) = json::top_level(line) else {
                continue;
            };
            let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
            let text = |name: &str| match field(name) {
                Some(json::Value::Str(text)) => Some(text.clone()),
                _ => None,
            };
            let (Some(event), Some(sid)) = (text("event"), text("sid")) else {
                continue;
            };
            let id = match field("child_id") {
                Some(json::Value::Int(id)) => Some(*id),
                _ => None,
            };
            match (event.as_str(), id) {
                ("start", _) => starts.push((sid, text("time"))),
                ("child_start", Some(id)) => {
                    let hook = match text("child_class").as_deref() {
                        Some("hook") => text("hook_name"),
                        _ => None,
                    };
                    let argv = match field("argv") {
                        Some(json::Value::Strs(argv)) => argv.clone(),
                        _ => Vec::new(),
                    };
                    at.insert((sid.clone(), id), children.len());
                    children.push(Child {
                        parent: sid,
                        hook,
                        argv,
                        code: None,
                        started: text("time"),
                        ended: None,
                    });
                }
                ("child_exit", Some(id)) => {
                    if let (Some(&i), Some(json::Value::Int(code))) =
                        (at.get(&(sid, id)), field("code"))
                    {
                        children[i].code = Some(*code);
                        children[i].ended = text("time");
                    }
                }
                _ => {}
            }
        }
        let own = own_gits(&starts, &children);
        Self { children, own }
    }
}

/// The command's own gits ([`Trace`]): each root — a `sid` with no `/` — and
/// each git one level under it that started outside every hook of the
/// root's. git runs a hook to its end before it goes on, so a git that
/// started while one ran is that hook's.
fn own_gits(starts: &[(String, Option<String>)], children: &[Child]) -> HashSet<String> {
    let mut own = HashSet::new();
    for (root, _) in starts.iter().filter(|(sid, _)| !sid.contains('/')) {
        own.insert(root.clone());
        let hooks: Vec<&Child> = children
            .iter()
            .filter(|child| &child.parent == root && child.hook.is_some())
            .collect();
        for (sid, started) in starts {
            let under_root = sid
                .strip_prefix(root.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .is_some_and(|rest| !rest.contains('/'));
            let Some(started) = started.as_deref().filter(|_| under_root) else {
                continue;
            };
            if !hooks.iter().any(|hook| ran_during(hook, started)) {
                own.insert(sid.clone());
            }
        }
    }
    own
}

/// Whether `when` falls while `hook` ran — or might have, where the trace
/// does not time its start.
fn ran_during(hook: &Child, when: &str) -> bool {
    match hook.started.as_deref() {
        None => true,
        Some(started) => when >= started && hook.ended.as_deref().is_none_or(|ended| when <= ended),
    }
}

/// The folder one traced command's gits write into: a scratch folder under
/// the repository's git directory (`<git_dir>/platitude/`, where the
/// commit messages and patches handed to git go too — `crate::scratch`),
/// found from the command's working directory, the top of its worktree.
/// `None` where none can be made, and the command then runs untraced — its
/// refusals named by git's words alone. Removed with what git wrote when the
/// handle drops.
pub(super) fn folder_for(workdir: Option<&Path>) -> Option<crate::scratch::ScratchDir> {
    let git_dir = crate::repo::git_dir_of(workdir?)?;
    crate::scratch::ScratchDir::fresh(&git_dir, "TRACE").ok()
}

/// Just enough JSON for trace2's lines: the top level's keys, with string,
/// integer and string-array values read and anything else stepped over.
mod json {
    pub(super) enum Value {
        Str(String),
        Int(i64),
        Strs(Vec<String>),
        Other,
    }

    pub(super) fn top_level(line: &str) -> Option<Vec<(String, Value)>> {
        let mut at = Cursor {
            bytes: line.trim().as_bytes(),
            i: 0,
        };
        at.expect(b'{')?;
        let mut fields = Vec::new();
        if at.eat(b'}') {
            return Some(fields);
        }
        loop {
            let key = at.string()?;
            at.expect(b':')?;
            fields.push((key, at.value()?));
            if at.eat(b'}') {
                return Some(fields);
            }
            at.expect(b',')?;
        }
    }

    struct Cursor<'a> {
        bytes: &'a [u8],
        i: usize,
    }

    impl Cursor<'_> {
        fn blank(&mut self) {
            while self.bytes.get(self.i).is_some_and(u8::is_ascii_whitespace) {
                self.i += 1;
            }
        }

        fn eat(&mut self, byte: u8) -> bool {
            self.blank();
            let hit = self.bytes.get(self.i) == Some(&byte);
            if hit {
                self.i += 1;
            }
            hit
        }

        fn expect(&mut self, byte: u8) -> Option<()> {
            self.eat(byte).then_some(())
        }

        fn string(&mut self) -> Option<String> {
            self.expect(b'"')?;
            let mut out = Vec::new();
            loop {
                let byte = *self.bytes.get(self.i)?;
                self.i += 1;
                match byte {
                    b'"' => return Some(String::from_utf8_lossy(&out).into_owned()),
                    b'\\' => {
                        let escaped = *self.bytes.get(self.i)?;
                        self.i += 1;
                        match escaped {
                            b'n' => out.push(b'\n'),
                            b'r' => out.push(b'\r'),
                            b't' => out.push(b'\t'),
                            b'b' => out.push(0x08),
                            b'f' => out.push(0x0c),
                            b'u' => {
                                let hex = std::str::from_utf8(self.bytes.get(self.i..self.i + 4)?)
                                    .ok()?;
                                self.i += 4;
                                let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?)?;
                                out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                            }
                            other => out.push(other),
                        }
                    }
                    other => out.push(other),
                }
            }
        }

        fn value(&mut self) -> Option<Value> {
            self.blank();
            match *self.bytes.get(self.i)? {
                b'"' => self.string().map(Value::Str),
                b'[' => self.array(),
                b'{' => self.skip_object().map(|()| Value::Other),
                _ => self.scalar(),
            }
        }

        fn array(&mut self) -> Option<Value> {
            self.expect(b'[')?;
            let mut strings = Some(Vec::new());
            if self.eat(b']') {
                return strings.map(Value::Strs);
            }
            loop {
                match self.value()? {
                    Value::Str(s) => {
                        if let Some(list) = strings.as_mut() {
                            list.push(s);
                        }
                    }
                    _ => strings = None,
                }
                if self.eat(b']') {
                    return Some(strings.map_or(Value::Other, Value::Strs));
                }
                self.expect(b',')?;
            }
        }

        fn skip_object(&mut self) -> Option<()> {
            self.expect(b'{')?;
            if self.eat(b'}') {
                return Some(());
            }
            loop {
                self.string()?;
                self.expect(b':')?;
                self.value()?;
                if self.eat(b'}') {
                    return Some(());
                }
                self.expect(b',')?;
            }
        }

        /// A number, `true`, `false` or `null`: the run of bytes up to the
        /// next separator. An integer is read; the rest is stepped over.
        fn scalar(&mut self) -> Option<Value> {
            let start = self.i;
            while self
                .bytes
                .get(self.i)
                .is_some_and(|b| !matches!(b, b',' | b'}' | b']') && !b.is_ascii_whitespace())
            {
                self.i += 1;
            }
            let word = std::str::from_utf8(&self.bytes[start..self.i]).ok()?;
            if word.is_empty() {
                return None;
            }
            Some(word.parse().map_or(Value::Other, Value::Int))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recorded off git 2.55.0.windows.3, `GIT_TRACE2_EVENT` a folder: the
    // `start` and `child_*` lines (the rest left out). The fields are the
    // minimum's too (trace2/tr2_tgt_event.c).

    /// `git push` whose `pre-push` hook said no.
    const PUSH_TURNED_DOWN: &str = r#"{"event":"start","sid":"20261011T162739.988071Z-H5c51439d-P000046ac","thread":"main","time":"2026-10-11T16:27:39.991072Z","file":"common-init.c","line":58,"t_abs":0.006237,"argv":["git.exe","push","--porcelain","origin","main:refs/heads/main"]}
{"event":"child_start","sid":"20261011T162739.988071Z-H5c51439d-P000046ac","thread":"main","time":"2026-10-11T16:27:40.006894Z","file":"run-command.c","line":741,"child_id":0,"child_class":"transport/file","use_shell":true,"argv":["git-receive-pack '../remote.git'"]}
{"event":"child_start","sid":"20261011T162739.988071Z-H5c51439d-P000046ac","thread":"main","time":"2026-10-11T16:27:40.050592Z","file":"run-command.c","line":741,"child_id":1,"child_class":"hook","hook_name":"pre-push","use_shell":false,"argv":[".git/hooks/pre-push","origin","../remote.git"]}
{"event":"child_exit","sid":"20261011T162739.988071Z-H5c51439d-P000046ac","thread":"main","time":"2026-10-11T16:27:40.063100Z","file":"run-command.c","line":1008,"child_id":1,"pid":70036,"code":1,"t_rel":0.012504}
{"event":"child_exit","sid":"20261011T162739.988071Z-H5c51439d-P000046ac","thread":"main","time":"2026-10-11T16:27:40.065099Z","file":"run-command.c","line":1008,"child_id":0,"pid":67288,"code":0,"t_rel":0.058209}"#;

    /// `git commit -S` with `gpg.format=ssh` and a key file that is not
    /// there: git said only ssh-keygen's words and `failed to write commit
    /// object`.
    const SIGNING_FAILED: &str = r#"{"event":"start","sid":"20261011T162740.171053Z-H5c51439d-P00007f48","thread":"main","time":"2026-10-11T16:27:40.174053Z","file":"common-init.c","line":58,"t_abs":0.006343,"argv":["git.exe","commit","-S","-q","-m","two"]}
{"event":"child_start","sid":"20261011T162740.171053Z-H5c51439d-P00007f48","thread":"main","time":"2026-10-11T16:27:40.197052Z","file":"run-command.c","line":741,"child_id":0,"child_class":"?","use_shell":false,"argv":["ssh-keygen","-Y","sign","-n","git","-f","C:\\Users\\wrongwrong\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\f\\target\\tmp\\pggt2\\no-such-key.pub","C:\\Users\\WRONGW~1\\AppData\\Local\\Temp/.git_signing_buffer_tmpZ4Lb3N"]}
{"event":"child_exit","sid":"20261011T162740.171053Z-H5c51439d-P00007f48","thread":"main","time":"2026-10-11T16:27:40.217382Z","file":"run-command.c","line":1008,"child_id":0,"pid":9632,"code":255,"t_rel":0.020246}"#;

    /// `git commit` whose `pre-commit` hook ran a commit in another
    /// repository, whose own `pre-commit` said no — and passed anyway.
    const NO_INSIDE_A_HOOK: &str = r#"{"event":"start","sid":"20261011T160706.659040Z-H5c51439d-P00008aa8","thread":"main","time":"2026-10-11T16:07:06.663044Z","file":"common-init.c","line":58,"t_abs":0.007379,"argv":["git.exe","commit","-q","--allow-empty","-m","outer"]}
{"event":"child_start","sid":"20261011T160706.659040Z-H5c51439d-P00008aa8","thread":"main","time":"2026-10-11T16:07:06.685811Z","file":"run-command.c","line":741,"child_id":0,"child_class":"hook","hook_name":"pre-commit","use_shell":false,"argv":[".git/hooks/pre-commit"]}
{"event":"child_exit","sid":"20261011T160706.659040Z-H5c51439d-P00008aa8","thread":"main","time":"2026-10-11T16:07:06.761158Z","file":"run-command.c","line":1008,"child_id":0,"pid":50032,"code":0,"t_rel":0.075162}
{"event":"start","sid":"20261011T160706.659040Z-H5c51439d-P00008aa8/20261011T160706.715250Z-H5c51439d-P0000ad74","thread":"main","time":"2026-10-11T16:07:06.718048Z","file":"common-init.c","line":58,"t_abs":0.004530,"argv":["C:\\Program Files\\Git\\mingw64\\libexec\\git-core\\git.exe","-C","C:/Users/wrongwrong/IdeaProjects/platitude-gg/.claude/worktrees/f/target/tmp/pggt/inner","commit","-q","--allow-empty","-m","nested"]}
{"event":"child_start","sid":"20261011T160706.659040Z-H5c51439d-P00008aa8/20261011T160706.715250Z-H5c51439d-P0000ad74","thread":"main","time":"2026-10-11T16:07:06.742842Z","file":"run-command.c","line":741,"child_id":0,"child_class":"hook","hook_name":"pre-commit","use_shell":false,"argv":[".git/hooks/pre-commit"]}
{"event":"child_exit","sid":"20261011T160706.659040Z-H5c51439d-P00008aa8/20261011T160706.715250Z-H5c51439d-P0000ad74","thread":"main","time":"2026-10-11T16:07:06.756646Z","file":"run-command.c","line":1008,"child_id":0,"pid":9724,"code":1,"t_rel":0.014022}"#;

    /// `git rebase -i` whose `exec git commit --amend` a `commit-msg` hook
    /// turned down — a git the replay ran itself, through its shell.
    const REWORD_TURNED_DOWN: &str = r#"{"event":"start","sid":"20261011T160706.958754Z-H5c51439d-P0000e058","thread":"main","time":"2026-10-11T16:07:06.961262Z","file":"common-init.c","line":58,"t_abs":0.006817,"argv":["git.exe","rebase","-i","HEAD~1"]}
{"event":"child_start","sid":"20261011T160706.958754Z-H5c51439d-P0000e058","thread":"main","time":"2026-10-11T16:07:07.016003Z","file":"run-command.c","line":741,"child_id":1,"child_class":"?","use_shell":true,"argv":["git commit --amend --allow-empty -m reworded"]}
{"event":"child_exit","sid":"20261011T160706.958754Z-H5c51439d-P0000e058","thread":"main","time":"2026-10-11T16:07:07.099969Z","file":"run-command.c","line":1008,"child_id":1,"pid":73264,"code":1,"t_rel":0.083230}
{"event":"start","sid":"20261011T160706.958754Z-H5c51439d-P0000e058/20261011T160707.057545Z-H5c51439d-P000091ec","thread":"main","time":"2026-10-11T16:07:07.060056Z","file":"common-init.c","line":58,"t_abs":0.004171,"argv":["C:\\Program Files\\Git\\mingw64\\libexec\\git-core\\git.exe","commit","--amend","--allow-empty","-m","reworded"]}
{"event":"child_start","sid":"20261011T160706.958754Z-H5c51439d-P0000e058/20261011T160707.057545Z-H5c51439d-P000091ec","thread":"main","time":"2026-10-11T16:07:07.082187Z","file":"run-command.c","line":741,"child_id":0,"child_class":"hook","hook_name":"commit-msg","use_shell":false,"argv":[".git/hooks/commit-msg",".git/COMMIT_EDITMSG"]}
{"event":"child_exit","sid":"20261011T160706.958754Z-H5c51439d-P0000e058/20261011T160707.057545Z-H5c51439d-P000091ec","thread":"main","time":"2026-10-11T16:07:07.096695Z","file":"run-command.c","line":1008,"child_id":0,"pid":42296,"code":1,"t_rel":0.014076}"#;

    /// `git rebase -i` whose own `post-checkout` hook ran a commit in
    /// another repository, whose `pre-commit` said no — and the replay went.
    const NO_INSIDE_A_REPLAYS_HOOK: &str = r#"{"event":"start","sid":"20261011T160707.317645Z-H5c51439d-P0000f240","thread":"main","time":"2026-10-11T16:07:07.320818Z","file":"common-init.c","line":58,"t_abs":0.006781,"argv":["git.exe","rebase","-i","--force-rebase","HEAD~1"]}
{"event":"child_start","sid":"20261011T160707.317645Z-H5c51439d-P0000f240","thread":"main","time":"2026-10-11T16:07:07.376367Z","file":"run-command.c","line":741,"child_id":1,"child_class":"hook","hook_name":"post-checkout","use_shell":false,"argv":[".git/hooks/post-checkout","5dacc2b19bad7f490d2f57b37ad84cccc7c17ae3","bb1f563b23aa1ded4c51546c4ebf0ed3a9efdac9","1"]}
{"event":"child_exit","sid":"20261011T160707.317645Z-H5c51439d-P0000f240","thread":"main","time":"2026-10-11T16:07:07.467081Z","file":"run-command.c","line":1008,"child_id":1,"pid":74344,"code":0,"t_rel":0.090878}
{"event":"start","sid":"20261011T160707.317645Z-H5c51439d-P0000f240/20261011T160707.413029Z-H5c51439d-P0000f4c8","thread":"main","time":"2026-10-11T16:07:07.415033Z","file":"common-init.c","line":58,"t_abs":0.004176,"argv":["C:\\Program Files\\Git\\mingw64\\libexec\\git-core\\git.exe","-C","C:/Users/wrongwrong/IdeaProjects/platitude-gg/.claude/worktrees/f/target/tmp/pggt/inner","commit","-q","--allow-empty","-m","nested"]}
{"event":"child_start","sid":"20261011T160707.317645Z-H5c51439d-P0000f240/20261011T160707.413029Z-H5c51439d-P0000f4c8","thread":"main","time":"2026-10-11T16:07:07.439056Z","file":"run-command.c","line":741,"child_id":0,"child_class":"hook","hook_name":"pre-commit","use_shell":false,"argv":[".git/hooks/pre-commit"]}
{"event":"child_exit","sid":"20261011T160707.317645Z-H5c51439d-P0000f240/20261011T160707.413029Z-H5c51439d-P0000f4c8","thread":"main","time":"2026-10-11T16:07:07.463079Z","file":"run-command.c","line":1008,"child_id":0,"pid":69132,"code":1,"t_rel":0.023661}"#;

    const COMMIT_HOOKS: [&str; 3] = ["pre-commit", "prepare-commit-msg", "commit-msg"];

    #[test]
    fn a_hook_that_ended_non_zero_is_read_by_its_event() {
        let trace = Trace::parse(PUSH_TURNED_DOWN);
        assert!(trace.hook_failed(&["pre-push"]));
        assert!(!trace.hook_failed(&COMMIT_HOOKS));
        assert!(!trace.ssh_signer_failed(), "the transport is no signer");
        let passed = PUSH_TURNED_DOWN.replace("\"code\":1,", "\"code\":0,");
        assert!(!Trace::parse(&passed).hook_failed(&["pre-push"]));
    }

    /// A `child_id` counts within its own process: the hook's end in one
    /// process is no end of the same number started in another — here a
    /// git the command ran itself, whose hook does count once its own end
    /// is moved over with it.
    #[test]
    fn a_child_is_matched_to_its_end_within_its_own_process() {
        let sid = "20261011T162739.988071Z-H5c51439d-P000046ac";
        let another = format!("{sid}/another");
        let moved = |events: &[&str]| {
            let lines = PUSH_TURNED_DOWN.lines().map(|line| {
                match events
                    .iter()
                    .any(|e| line.contains(&format!("\"event\":\"{e}\"")))
                {
                    true => line.replace(sid, &another),
                    false => line.to_string(),
                }
            });
            std::iter::once(event_line(&another, "start", "16:27:40.000000", ""))
                .chain(lines)
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(Trace::parse(&moved(&["child_start", "child_exit"])).hook_failed(&["pre-push"]));
        assert!(!Trace::parse(&moved(&["child_start"])).hook_failed(&["pre-push"]));
    }

    /// One trace2 line of `event` in process `sid` at `time` (of the
    /// fixtures' day), `rest` its other fields — what [`Trace::parse`] reads.
    fn event_line(sid: &str, event: &str, time: &str, rest: &str) -> String {
        format!(r#"{{"event":"{event}","sid":"{sid}","time":"2026-10-11T{time}Z"{rest}}}"#)
    }

    /// A `pre-commit` that ended `code` in `sid`, from `from` to `to`.
    fn hook_in(sid: &str, from: &str, to: &str, code: i32) -> [String; 2] {
        [
            event_line(
                sid,
                "child_start",
                from,
                r#","child_id":0,"child_class":"hook","hook_name":"pre-commit","argv":["h"]"#,
            ),
            event_line(
                sid,
                "child_exit",
                to,
                &format!(r#","child_id":0,"code":{code}"#),
            ),
        ]
    }

    /// The gits the command ran are one level under it; a git one of those
    /// ran — through its hook, or itself — is that git's business, as a
    /// hook's own git is the command's hook's.
    #[test]
    fn a_git_two_levels_down_is_not_the_commands() {
        let root = "R";
        let level = |sid: &str| {
            [event_line(root, "start", "10:00:00.000000", "")]
                .into_iter()
                .chain([event_line(sid, "start", "10:00:01.000000", "")])
                .chain(hook_in(sid, "10:00:02.000000", "10:00:03.000000", 1))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(Trace::parse(&level("R/A")).hook_failed(&COMMIT_HOOKS));
        let two_down = format!(
            "{}\n{}",
            event_line("R/A", "start", "10:00:00.500000", ""),
            level("R/A/B")
        );
        assert!(!Trace::parse(&two_down).hook_failed(&COMMIT_HOOKS));
    }

    #[test]
    fn an_ssh_signer_that_failed_is_read_off_its_arguments() {
        let trace = Trace::parse(SIGNING_FAILED);
        assert!(trace.ssh_signer_failed());
        assert!(!trace.hook_failed(&COMMIT_HOOKS));
        let signed = SIGNING_FAILED.replace("\"code\":255,", "\"code\":0,");
        assert!(!Trace::parse(&signed).ssh_signer_failed());
    }

    /// The no a hook's own git met is the hook's to take: here it took it
    /// and passed.
    #[test]
    fn a_no_inside_a_hook_is_not_the_commands() {
        assert!(!Trace::parse(NO_INSIDE_A_HOOK).hook_failed(&COMMIT_HOOKS));
    }

    /// A git the replay ran itself — its `exec`'s `git commit`, outside any
    /// hook of the replay's — is the replay's own.
    #[test]
    fn a_commit_the_replay_runs_itself_is_its_own() {
        assert!(Trace::parse(REWORD_TURNED_DOWN).hook_failed(&COMMIT_HOOKS));
    }

    /// A git that started while a hook of the replay's ran is that hook's.
    #[test]
    fn a_git_a_replays_hook_ran_is_the_hooks() {
        assert!(!Trace::parse(NO_INSIDE_A_REPLAYS_HOOK).hook_failed(&COMMIT_HOOKS));
    }

    /// No process start, no command to own the hook: nothing is named.
    #[test]
    fn a_trace_with_no_start_names_nothing() {
        let headless = PUSH_TURNED_DOWN
            .lines()
            .filter(|line| !line.contains("\"event\":\"start\""))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!Trace::parse(&headless).hook_failed(&["pre-push"]));
    }

    #[test]
    fn lines_that_are_not_events_are_stepped_over() {
        let trace = Trace::parse(
            "not json\n{\"event\":\"child_exit\"}\n{\"nested\":{\"a\":[1,{}]},\"x\":null}\n",
        );
        assert_eq!(trace, Trace::default());
    }

    #[test]
    fn escapes_in_a_string_are_read() {
        let fields = json::top_level(r#"{"argv":["a\"b","c\\d","\u0001"]}"#).expect("an object");
        let Some((_, json::Value::Strs(argv))) = fields.first() else {
            panic!("argv is a list of strings");
        };
        assert_eq!(argv, &["a\"b", "c\\d", "\u{1}"]);
    }
}
