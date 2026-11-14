//! What a call answers, as the origin of the value it hands back: a
//! function or method of the language's by its contract (`super::builtins`)
//! — a new container made where the call is spelled, holding what the
//! contract says —, a harness or product function by what its own code
//! returns, read as the caller's, and a method of something of the
//! product's the product's. What a function handed to a call answers is
//! read the same way (`callback_answers`). A call nothing here answers for
//! answers a value of no known origin.

use std::ops::Range;

use super::builtins::{self, Answers, GlobalAnswers, Made};
use super::calls::function_body;
use super::origin::{HELD, Origin, Site, split_top};
use super::program::{Program, Value};
use super::scan::Path;
use crate::lex::matching_close;

impl Program {
    /// What a call answers; `args` are the tokens between its parentheses.
    pub(super) fn call_origin(&mut self, code: usize, path: &Path, args: Range<usize>) -> Origin {
        let Some(method) = path.last().map(str::to_string) else {
            return Origin::Unknown;
        };
        let site = (self.codes[code].doc, path.at);
        let listed = self.arguments(code, &args);
        if let Some(head) = self.global_of(code, path) {
            let spread = self.spreads(code, &listed);
            return match builtins::global(&head, &method, listed.len(), spread) {
                Some((answers, made)) => {
                    self.made_at(site, made);
                    self.global_answer(code, answers, site, &listed)
                }
                None => Origin::Unknown,
            };
        }
        match self.resolve(code, path) {
            // The harness's function or the product's: what its own code returns, read as the
            // caller's.
            Value::Function(fd, fu) => {
                if self.declared.contains_key(&(fd, fu)) {
                    return Origin::Unknown;
                }
                {
                    let callee = self.code_of(fd, fu);
                    self.return_origin(fd, fu).handed_from(callee, code)
                }
            }
            Value::Global(g) if builtins::PLAIN_FUNCTIONS.contains(&g.as_str()) => Origin::Plain,
            Value::Global(g) if builtins::NEW_FUNCTIONS.contains(&g.as_str()) => {
                self.made_at(site, Made::Constructed);
                Origin::made(code, site)
            }
            _ if path.members.is_empty() => Origin::Unknown,
            _ => {
                let receiver = path.receiver().unwrap_or_else(|| path.clone());
                let receiver_origin = self.path_origin(code, &receiver);
                self.method_origin(code, receiver_origin, &method, site, args)
            }
        }
    }

    /// Whether any of a call's arguments is spread, so that their count is
    /// not known.
    fn spreads(&self, code: usize, args: &[Range<usize>]) -> bool {
        let d = self.codes[code].doc;
        args.iter().any(|arg| {
            self.docs[d]
                .tokens
                .get(arg.start)
                .is_some_and(|t| t.text == "...")
        })
    }

    /// The elements of a call's argument list, `args` its tokens.
    fn arguments(&self, code: usize, args: &Range<usize>) -> Vec<Range<usize>> {
        let d = self.codes[code].doc;
        split_top(&self.docs[d].tokens[args.clone()], &[","])
            .into_iter()
            .map(|(from, to)| args.start + from..args.start + to)
            .collect()
    }

    /// What a global object's function answers, by its contract.
    fn global_answer(
        &mut self,
        code: usize,
        answers: GlobalAnswers,
        site: Site,
        args: &[Range<usize>],
    ) -> Origin {
        let first = |program: &mut Program| match args.first() {
            Some(arg) => program.origin_of(code, arg.clone()),
            None => Origin::Unknown,
        };
        match answers {
            GlobalAnswers::Plain => Origin::Plain,
            GlobalAnswers::NewPlain => {
                self.store_at(code, site, Origin::Plain);
                Origin::made(code, site)
            }
            GlobalAnswers::NewInsideOfFirst => {
                let first = first(self);
                let held = self.inside_of(&first);
                self.store_at(code, site, held);
                Origin::made(code, site)
            }
            GlobalAnswers::NewOfArguments => {
                for arg in args {
                    let element = self.element_origin(code, arg.clone());
                    self.store_at(code, site, element);
                }
                Origin::made(code, site)
            }
            GlobalAnswers::NewOfCallback(at) => {
                let answered = self.callback_answers(code, args[at].clone());
                self.store_at(code, site, answered);
                Origin::made(code, site)
            }
            GlobalAnswers::NewDeep => {
                self.store_at(code, site, Origin::made(HELD, site));
                Origin::made(code, site)
            }
            GlobalAnswers::First => first(self),
        }
    }

    /// What `method` called on a value of `receiver`'s origin answers, by
    /// its contract; `site` is where a new container it answers is made.
    pub(super) fn method_origin(
        &mut self,
        code: usize,
        receiver: Origin,
        method: &str,
        site: Site,
        args: Range<usize>,
    ) -> Origin {
        if matches!(receiver, Origin::Object(_) | Origin::Product) {
            return Origin::Product;
        }
        if receiver == Origin::Unset {
            return Origin::Unset;
        }
        if !self.language_method(&receiver, method) {
            return Origin::Unknown;
        }
        let args = self.arguments(code, &args);
        let Some(answers) = builtins::method(method, args.len(), self.spreads(code, &args)) else {
            return Origin::Unknown;
        };
        // A method of the language's that answers a new container answers an array.
        self.made_at(site, Made::Array);
        match answers {
            Answers::Plain => Origin::Plain,
            Answers::NewPlain => {
                self.store_at(code, site, Origin::Plain);
                Origin::made(code, site)
            }
            Answers::NewInside => {
                let held = self.inside_of(&receiver);
                self.store_at(code, site, held);
                Origin::made(code, site)
            }
            Answers::NewJoined => {
                let held = self.inside_of(&receiver);
                self.store_at(code, site, held);
                for arg in args {
                    let element = self.element_origin(code, arg);
                    let inside = self.inside_of(&element);
                    self.store_at(code, site, element);
                    self.store_at(code, site, inside);
                }
                Origin::made(code, site)
            }
            Answers::NewOfCallback(at) => {
                let answered = self.callback_answers(code, args[at].clone());
                self.store_at(code, site, answered);
                Origin::made(code, site)
            }
            Answers::NewOfCallbackFlat(at) => {
                let answered = self.callback_answers(code, args[at].clone());
                let flattened = self.inside_of(&answered);
                self.store_at(code, site, answered);
                self.store_at(code, site, flattened);
                Origin::made(code, site)
            }
            Answers::Inside => self.inside_of(&receiver),
            Answers::Receiver => receiver,
        }
    }

    /// What a function handed to a call answers: a function spelled there
    /// answers its returned values, read as the code it is spelled in reads
    /// them — its parameters by what the call hands them (`param_origin`);
    /// a plain global function named there (`Number`) answers a plain value;
    /// a harness or product function named there answers what its code
    /// returns; anything else answers a value of no known origin.
    pub(super) fn callback_answers(&mut self, code: usize, arg: Range<usize>) -> Origin {
        let d = self.codes[code].doc;
        if let Some(body) = function_body(&self.docs[d].tokens[arg.clone()]) {
            let body = arg.start + body.start..arg.start + body.end;
            let tokens = &self.docs[d].tokens;
            let block = tokens[body.start].text == "{"
                && matching_close(tokens, body.start, "{", "}") == Some(body.end - 1);
            if !block {
                return self.origin_of(code, body);
            }
            let returns = super::scan::facts(tokens, body).returns;
            let mut answered = Origin::Unset;
            for (value, _) in returns {
                answered = answered.join(self.origin_of(code, value));
            }
            // Returning nothing answers `undefined`.
            return if answered == Origin::Unset {
                Origin::Plain
            } else {
                answered
            };
        }
        if self.names_plain_function(code, arg.clone()) {
            return Origin::Plain;
        }
        match self.named_callback(code, arg) {
            Some((fd, fu)) => {
                let callee = self.code_of(fd, fu);
                self.return_origin(fd, fu).handed_from(callee, code)
            }
            None => Origin::Unknown,
        }
    }
}
