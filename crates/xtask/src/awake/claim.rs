//! A claim as it is written, and what a hook's mark makes of the one
//! standing.

use super::{CLAIM_FORMAT, Mark, WAKE_GRACE};

/// Whether a claim's session — or subagent — is in a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Working,
    Idle,
}

impl State {
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Idle => "idle",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        match word {
            "working" => Some(Self::Working),
            "idle" => Some(Self::Idle),
            _ => None,
        }
    }
}

/// One tool call, from its start to its end:
/// `<who>:<id>:<name>:<input>:<start>:<end>:<asked>`. While it runs it is
/// work of its own; while a person is asked about it, it is not; and for a
/// shell's call, the children of the Claude process started between its
/// start and its end are work for as long as they run — started in the
/// background, sent there halfway, or run in the foreground.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Call {
    /// `-` for the session's own call, else the subagent's id.
    pub who: String,
    pub id: String,
    pub name: String,
    /// A digest of the call's input, which a permission prompt that names
    /// no call is matched by.
    pub input: String,
    /// In unix milliseconds.
    pub start: u64,
    /// In unix milliseconds; 0 while the call runs.
    pub end: u64,
    /// A person is asked about it: a permission prompt, or a question of
    /// its own.
    pub asked: bool,
}

/// An MCP server's request for a person's input, pending: `<server>:<id>`.
/// It names no call, only its server, and comes during one of that
/// server's calls (the hooks reference): it is counted as holding up one of
/// the session's running calls on that server until its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Request {
    /// The server as its tools' names spell it (`mcp__<server>__…`).
    pub server: String,
    /// The request's own id, `-` when it came with none.
    pub id: String,
}

/// One claim: `r<format> <state> <claude pid> <at> <seat> <wake> <calls>
/// <requests> <transcript>` — one of another [`CLAIM_FORMAT`] does not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Claim {
    pub state: State,
    /// The Claude process whose life the claim lasts.
    pub claude: u32,
    /// When a hook last wrote its state — or the holder saw a notice start
    /// a turn — in unix milliseconds, to set against the transcript's
    /// timestamps.
    pub at: u64,
    /// The seat the session wrote it from, `-` outside one; for a reader.
    pub seat: String,
    /// Until when a scheduled wake-up holds the machine, in unix seconds;
    /// 0 for none. A stop leaves it.
    pub wake: u64,
    /// The tool calls, the session's and its subagents': the session's
    /// claim keeps them all, since a subagent's process outlives it.
    pub calls: Vec<Call>,
    /// The MCP servers' pending requests for input, on the session's
    /// claim: a request names neither a call nor a subagent.
    pub requests: Vec<Request>,
    /// The session's — or subagent's own — transcript, where an
    /// interrupted turn and a call's result are read. `-` when unknown;
    /// last, since a path holds spaces.
    pub transcript: String,
}

fn listed<T>(items: &[T], each: impl Fn(&T) -> String) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.iter().map(each).collect::<Vec<_>>().join(",")
    }
}

fn unlisted<T>(word: &str, each: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    if word == "-" {
        return Some(Vec::new());
    }
    word.split(',').map(each).collect()
}

impl Claim {
    pub(super) fn line(&self) -> String {
        let calls = listed(&self.calls, |call| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                call.who,
                call.id,
                call.name,
                call.input,
                call.start,
                call.end,
                u8::from(call.asked)
            )
        });
        let requests = listed(&self.requests, |request| {
            format!("{}:{}", request.server, request.id)
        });
        format!(
            "r{CLAIM_FORMAT} {} {} {} {} {} {calls} {requests} {}",
            self.state.word(),
            self.claude,
            self.at,
            self.seat,
            self.wake,
            self.transcript
        )
    }

    pub(super) fn parse(text: &str) -> Option<Self> {
        let mut words = text.trim().splitn(9, ' ');
        if words.next()? != format!("r{CLAIM_FORMAT}") {
            return None;
        }
        let state = State::from_word(words.next()?)?;
        let claude = words.next()?.parse().ok()?;
        let at = words.next()?.parse().ok()?;
        let seat = words.next()?.to_string();
        let wake = words.next()?.parse().ok()?;
        let calls = unlisted(words.next()?, parse_call)?;
        let requests = unlisted(words.next()?, |request| {
            let (server, id) = request.split_once(':')?;
            Some(Request {
                server: server.to_string(),
                id: id.to_string(),
            })
        })?;
        Some(Self {
            state,
            claude,
            at,
            seat,
            wake,
            calls,
            requests,
            transcript: words.next()?.to_string(),
        })
    }
}

fn parse_call(text: &str) -> Option<Call> {
    let mut parts = text.split(':');
    let call = Call {
        who: parts.next()?.to_string(),
        id: parts.next()?.to_string(),
        name: parts.next()?.to_string(),
        input: parts.next()?.to_string(),
        start: parts.next()?.parse().ok()?,
        end: parts.next()?.parse().ok()?,
        asked: match parts.next()? {
            "1" => true,
            "0" => false,
            _ => return None,
        },
    };
    parts.next().is_none().then_some(call)
}

/// The hook's own facts, which every claim it writes carries.
#[derive(Clone)]
pub(super) struct Writer {
    pub claude: u32,
    /// The hook's time, in unix milliseconds.
    pub now_ms: u64,
    pub seat: String,
    /// Whose calls this hook reports: `-` for the session's own.
    pub who: String,
    pub transcript: Option<String>,
}

impl Writer {
    /// The claim `mark` leaves in place of `before`; None for none. A
    /// mark on the calls touches the calls alone: a subagent's calls land
    /// on its session's claim and change nothing of the session's turn, nor
    /// the transcript its claim reads.
    pub(super) fn claim(&self, mark: Mark<'_>, before: Option<&Claim>) -> Option<Claim> {
        let mut claim = before.cloned().unwrap_or_else(|| Claim {
            state: State::Idle,
            claude: self.claude,
            at: self.now_ms,
            seat: self.seat.clone(),
            wake: 0,
            calls: Vec::new(),
            requests: Vec::new(),
            transcript: "-".to_string(),
        });
        claim.claude = self.claude;
        claim.seat.clone_from(&self.seat);
        match mark {
            Mark::Gone => return None,
            Mark::ToolStart { id, name, input } => {
                // The holder forgets a call once it has ended and left no
                // process behind.
                if !claim.calls.iter().any(|call| call.id == id) {
                    claim.calls.push(Call {
                        who: self.who.clone(),
                        id: id.to_string(),
                        name: name.to_string(),
                        input: input.to_string(),
                        start: self.now_ms,
                        end: 0,
                        asked: false,
                    });
                }
                return Some(claim);
            }
            Mark::ToolEnd(id) => {
                for call in claim.calls.iter_mut().filter(|call| call.id == id) {
                    if call.end == 0 {
                        call.end = self.now_ms;
                    }
                }
                return Some(claim);
            }
            Mark::Asked { id, name, input } => {
                if let Some(call) = self.asked_call(&mut claim, id, name, input) {
                    call.asked = true;
                }
                return Some(claim);
            }
            Mark::Elicited {
                server,
                id,
                pending,
            } => {
                let id = id.map_or_else(|| "-".to_string(), listed_word);
                let this = |request: &Request| request.server == server && request.id == id;
                if pending {
                    if id == "-" || !claim.requests.iter().any(this) {
                        claim.requests.push(Request {
                            server: server.to_string(),
                            id,
                        });
                    }
                    return Some(claim);
                }
                let answered = claim.requests.iter().position(this).or_else(|| {
                    // An answer with no id takes a request with none, else
                    // the server's oldest.
                    (id == "-").then_some(())?;
                    claim
                        .requests
                        .iter()
                        .position(|request| request.server == server)
                });
                if let Some(at) = answered {
                    claim.requests.remove(at);
                }
                return Some(claim);
            }
            Mark::CallsEnd => {
                self.end_calls(&mut claim);
                return Some(claim);
            }
            Mark::Working | Mark::WakeIn(_) | Mark::NoWake => {
                claim.state = State::Working;
                match mark {
                    Mark::WakeIn(seconds) => {
                        claim.wake = self.now_ms / 1000 + seconds + WAKE_GRACE;
                    }
                    Mark::NoWake => claim.wake = 0,
                    _ => {}
                }
            }
            Mark::Prompt => {
                // A prompt starts a turn: a call of the turn before that
                // never ended — turned down, interrupted — started all it
                // will, and a person no longer waits on it.
                claim.state = State::Working;
                self.end_turn(&mut claim);
            }
            Mark::Idle => {
                claim.state = State::Idle;
                self.end_turn(&mut claim);
            }
        }
        if let Some(transcript) = &self.transcript {
            claim.transcript.clone_from(transcript);
        }
        claim.at = self.now_ms;
        Some(claim)
    }

    /// The writer's own calls that have not ended.
    fn own_open<'c>(&self, claim: &'c mut Claim) -> impl Iterator<Item = &'c mut Call> {
        let who = self.who.clone();
        claim
            .calls
            .iter_mut()
            .filter(move |call| call.who == who && call.end == 0)
    }

    /// The call a person is asked about: by its id when the hook names it,
    /// else — a permission prompt names only the tool and its input, and
    /// follows the call's start — the latest of the writer's open calls
    /// with that name and input not asked about yet. The digests match only
    /// while both hooks write the input alike, which no run has shown: with
    /// none matching, the one such call with that name, if one alone; with
    /// more than one, none — the call then counts as running.
    fn asked_call<'c>(
        &self,
        claim: &'c mut Claim,
        id: Option<&str>,
        name: &str,
        input: &str,
    ) -> Option<&'c mut Call> {
        if let Some(id) = id {
            return claim.calls.iter_mut().find(|call| call.id == id);
        }
        let who = self.who.clone();
        let mut named: Vec<&'c mut Call> = claim
            .calls
            .iter_mut()
            .filter(|call| call.who == who && call.end == 0 && !call.asked && call.name == name)
            .collect();
        if let Some(at) = named
            .iter()
            .enumerate()
            .filter(|(_, call)| call.input == input)
            .max_by_key(|(_, call)| call.start)
            .map(|(at, _)| at)
        {
            return Some(named.swap_remove(at));
        }
        (named.len() == 1).then(|| named.swap_remove(0))
    }

    /// Ends the writer's own calls that never ended, and no one else's: a
    /// subagent's may run on after its session stopped.
    fn end_calls(&self, claim: &mut Claim) {
        let now = self.now_ms;
        for call in self.own_open(claim) {
            call.end = now;
        }
    }

    /// A turn of the session's ends: its calls, and the requests for input
    /// that came during it — one whose answer never reached a hook would
    /// hold up a call of the next. A background subagent's request still
    /// pending then goes too, and its call counts as running.
    fn end_turn(&self, claim: &mut Claim) {
        self.end_calls(claim);
        if self.who == "-" {
            claim.requests.clear();
        }
    }
}

/// A word a claim's lists can carry: what is not a letter, a digit, `_`,
/// `-` or `.` becomes `_`; `-` for none.
fn listed_word(text: &str) -> String {
    let word: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if word.is_empty() {
        "-".to_string()
    } else {
        word
    }
}

/// A digest of a call's input as the payload writes it (FNV-1a), for
/// telling one call from another of the same tool.
pub(crate) fn digest(input: &str) -> String {
    let hash = input.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}
