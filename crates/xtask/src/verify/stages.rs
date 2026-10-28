//! The operations' stages in a run's lines (`auto/Awaited.qml`,
//! `auto/StageTimer.qml`). Each operation — one sampler of the page's or
//! the window's driver, named `owner/op` — keeps a chain of its own per
//! generation (`begin`), moved by `stage` lines; nothing orders one
//! operation against another but the joins it declared (`after=`), so
//! two independent operations pass or fail alike in whichever order their
//! lines interleave. A chain breaks where a stage repeats in its
//! generation, goes against the order the operation declared, skips one,
//! moves from a stage it does not stand at, comes from a generation that
//! is over, comes before a stage it joins, or comes after the completion
//! (反映前テストの機械化 §検証時点).

use std::collections::{BTreeMap, BTreeSet};

use super::contract::field;

/// One operation's state as its lines have it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Chain {
    generation: u32,
    stage: String,
    visited: Vec<String>,
    order: Vec<String>,
    after: Vec<String>,
}

/// The chains of one run and the stages they reached.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Stages {
    chains: BTreeMap<String, Chain>,
    /// `owner/op/stage`, the operation's instance mark (`#2`) dropped —
    /// what a join names.
    reached: BTreeSet<String>,
    /// How many stage lines were read.
    pub said: usize,
}

/// The `owner/op` a line places itself at, and its generation.
fn place(line: &str) -> Option<(String, u32)> {
    let (owner, op) = (field(line, "owner")?, field(line, "op")?);
    let generation = field(line, "gen")?.parse().ok()?;
    Some((format!("{owner}/{op}"), generation))
}

/// The list a `key=a,b,c` word holds; empty where the line has none.
fn list(line: &str, key: &str) -> Vec<String> {
    field(line, key)
        .map(super::contract::names)
        .unwrap_or_default()
}

impl Stages {
    /// A `begin` line: a generation of an operation begins, one past the
    /// generation before it.
    pub(super) fn begin(&mut self, line: &str, completed: bool) -> Vec<String> {
        let Some((key, generation)) = place(line) else {
            return vec![format!("a `begin` line that names no operation: {line}")];
        };
        let mut out = Vec::new();
        if completed {
            out.push(format!(
                "{key} began generation {generation} after the completion"
            ));
        }
        let chain = self.chains.entry(key.clone()).or_default();
        if generation != chain.generation + 1 {
            out.push(format!(
                "{key} began generation {generation} where {} was due",
                chain.generation + 1
            ));
        }
        *chain = Chain {
            generation,
            stage: "start".to_string(),
            visited: Vec::new(),
            order: list(line, "order"),
            after: list(line, "after"),
        };
        out
    }

    /// A `stage` line: the operation's generation moves, once to each
    /// stage, from the stage it stands at, in the order it declared, after
    /// the stages it joins. `seq` is the run's count of stage lines, held
    /// by the caller.
    pub(super) fn stage(&mut self, line: &str, completed: bool) -> Vec<String> {
        self.said += 1;
        let (Some((key, generation)), Some(from), Some(to)) =
            (place(line), field(line, "from"), field(line, "to"))
        else {
            return vec![format!("a `stage` line that names no move: {line}")];
        };
        let mut out = Vec::new();
        if completed {
            out.push(format!("{key} moved to {to} after the completion"));
        }
        let Some(chain) = self.chains.get_mut(&key) else {
            return vec![format!("{key} moved to {to} with no generation begun")];
        };
        if generation != chain.generation {
            out.push(format!(
                "{key} moved to {to} in generation {generation}, which is over: generation {} began",
                chain.generation
            ));
            return out;
        }
        if from != chain.stage {
            out.push(format!(
                "{key} moved from {from} to {to} where it stood at {}",
                chain.stage
            ));
        }
        if chain.visited.iter().any(|v| v == to) {
            out.push(format!(
                "{key} reached {to} twice in generation {generation}"
            ));
        }
        out.extend(ordered(&key, chain, to));
        if chain.visited.is_empty() {
            for join in &chain.after {
                if !self.reached.contains(join) {
                    out.push(format!("{key} moved to {to} before {join}, which it joins"));
                }
            }
        }
        chain.visited.push(to.to_string());
        chain.stage = to.to_string();
        let base = key.split('#').next().unwrap_or(&key);
        self.reached.insert(format!("{base}/{to}"));
        out
    }

    /// The breaches of a completion from a generation of `owner/op`:
    /// it is the operation's current generation, and that generation's
    /// last move was the one to `complete` — the completion route reads a
    /// report only after that transition.
    pub(super) fn completion(&self, line: &str) -> Vec<String> {
        let Some((key, generation)) = place(line) else {
            return vec![format!("a completion that names no operation: {line}")];
        };
        if key == "none/none" {
            return vec!["the run was ended by a breach, not completed".to_string()];
        }
        let Some(chain) = self.chains.get(&key) else {
            return vec![format!("{key} completed with no generation begun")];
        };
        let mut out = Vec::new();
        if generation != chain.generation {
            out.push(format!(
                "{key} completed from generation {generation}, which is over: generation {} began",
                chain.generation
            ));
        } else if chain.stage != "complete" {
            out.push(format!(
                "{key} completed standing at {} — no transition to complete came before",
                chain.stage
            ));
        }
        out
    }
}

/// The breach of a move to `to` against the order `chain` declared: a
/// stage it does not name, one gone back to, one skipped, a completion
/// short of the last.
fn ordered(key: &str, chain: &Chain, to: &str) -> Option<String> {
    if chain.order.is_empty() {
        return None;
    }
    let done = chain.visited.len();
    if to == "complete" {
        return chain
            .order
            .get(done)
            .map(|due| format!("{key} completed before {due}"));
    }
    match chain.order.iter().position(|s| s == to) {
        None => Some(format!(
            "{key} moved to {to}, which its order does not name"
        )),
        Some(at) if at < done => Some(format!("{key} went back to {to}")),
        Some(at) if at > done => Some(format!(
            "{key} skipped {} on its way to {to}",
            chain.order[done]
        )),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Stages;

    fn begin(op: &str, generation: u32, extra: &str) -> String {
        let (owner, name) = op.split_once('/').expect("owner/op");
        format!("INFO bench: begin run=r1 owner={owner} op={name} gen={generation}{extra}")
    }
    fn stage(op: &str, generation: u32, from: &str, to: &str) -> String {
        let (owner, name) = op.split_once('/').expect("owner/op");
        format!(
            "INFO bench: stage run=r1 owner={owner} op={name} gen={generation} from={from} to={to} seq=0"
        )
    }

    /// Reads `lines` as the run would: each `begin`, each `stage`, and
    /// the breaches in the order found.
    fn breaches(lines: &[String]) -> Vec<String> {
        let mut stages = Stages::default();
        let mut out = Vec::new();
        for line in lines {
            if line.contains(" begin ") {
                out.extend(stages.begin(line, false));
            } else {
                out.extend(stages.stage(line, false));
            }
        }
        out
    }

    #[test]
    fn a_chain_in_its_order_moves_once_to_each_stage() {
        let lines = [
            begin("window/clear", 1, " order=pressed,cleared"),
            stage("window/clear", 1, "start", "pressed"),
            stage("window/clear", 1, "pressed", "cleared"),
            stage("window/clear", 1, "cleared", "complete"),
        ];
        assert!(breaches(&lines).is_empty(), "{:?}", breaches(&lines));
    }

    #[test]
    fn a_stage_twice_a_skip_a_way_back_and_a_short_completion_are_each_named() {
        let order = " order=a,b,c";
        let twice = [
            begin("page/x", 1, ""),
            stage("page/x", 1, "start", "a"),
            stage("page/x", 1, "a", "a"),
        ];
        assert!(
            breaches(&twice)
                .iter()
                .any(|b| b == "page/x reached a twice in generation 1")
        );
        let skip = [begin("page/x", 1, order), stage("page/x", 1, "start", "b")];
        assert_eq!(breaches(&skip), ["page/x skipped a on its way to b"]);
        let back = [
            begin("page/x", 1, order),
            stage("page/x", 1, "start", "a"),
            stage("page/x", 1, "a", "b"),
            stage("page/x", 1, "b", "a"),
        ];
        assert!(breaches(&back).iter().any(|b| b == "page/x went back to a"));
        let short = [
            begin("page/x", 1, order),
            stage("page/x", 1, "start", "a"),
            stage("page/x", 1, "a", "complete"),
        ];
        assert_eq!(breaches(&short), ["page/x completed before b"]);
        let unnamed = [begin("page/x", 1, order), stage("page/x", 1, "start", "z")];
        assert_eq!(
            breaches(&unnamed),
            ["page/x moved to z, which its order does not name"]
        );
        let astray = [
            begin("page/x", 1, ""),
            stage("page/x", 1, "start", "a"),
            stage("page/x", 1, "start", "b"),
        ];
        assert_eq!(
            breaches(&astray),
            ["page/x moved from start to b where it stood at a"]
        );
    }

    /// A generation that is over moves nothing, and generations count up
    /// one at a time: the same stage in the next generation is no repeat.
    #[test]
    fn generations_are_chains_of_their_own_and_an_old_one_moves_nothing() {
        let again = [
            begin("page/row", 1, ""),
            stage("page/row", 1, "start", "pressed"),
            begin("page/row", 2, ""),
            stage("page/row", 2, "start", "pressed"),
        ];
        assert!(breaches(&again).is_empty(), "{:?}", breaches(&again));
        let stale = [
            begin("page/row", 1, ""),
            begin("page/row", 2, ""),
            stage("page/row", 1, "start", "pressed"),
        ];
        assert_eq!(
            breaches(&stale),
            ["page/row moved to pressed in generation 1, which is over: generation 2 began"]
        );
        let leap = [begin("page/row", 1, ""), begin("page/row", 3, "")];
        assert_eq!(
            breaches(&leap),
            ["page/row began generation 3 where 2 was due"]
        );
    }

    #[test]
    fn a_join_is_owed_before_the_first_move_and_nothing_else_orders_two_operations() {
        let joined = [
            begin("window/clear", 1, " after=page/act/dispatched"),
            begin("page/act", 1, ""),
            stage("page/act", 1, "start", "dispatched"),
            stage("window/clear", 1, "start", "pressed"),
        ];
        assert!(breaches(&joined).is_empty(), "{:?}", breaches(&joined));
        let early = [
            begin("window/clear", 1, " after=page/act/dispatched"),
            begin("page/act", 1, ""),
            stage("window/clear", 1, "start", "pressed"),
            stage("page/act", 1, "start", "dispatched"),
        ];
        assert_eq!(
            breaches(&early),
            ["window/clear moved to pressed before page/act/dispatched, which it joins"]
        );
    }

    /// The completion is read off the operation it names: its current
    /// generation, standing at `complete`.
    #[test]
    fn a_completion_from_an_old_generation_or_short_of_its_transition_is_named() {
        let mut stages = Stages::default();
        stages.begin(&begin("page/x", 1, ""), false);
        stages.stage(&stage("page/x", 1, "start", "complete"), false);
        stages.begin(&begin("page/x", 2, ""), false);
        let line = "complete run=r1 verb=v owner=page op=x gen=1 stages=1 page=settled";
        assert_eq!(
            stages.completion(line),
            ["page/x completed from generation 1, which is over: generation 2 began"]
        );
        let mut stages = Stages::default();
        stages.begin(&begin("page/x", 1, ""), false);
        assert_eq!(
            stages.completion(line),
            ["page/x completed standing at start — no transition to complete came before"]
        );
    }
}
