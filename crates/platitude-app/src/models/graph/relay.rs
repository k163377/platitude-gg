//! A graph laid out again without walking (`GraphMsg::Relaid`): a delete
//! taking the commits only its names held off the screen, or a refused one
//! putting them back (デザイン規約 §消す操作は先に画面から消す).
//!
//! The session sends its commit rows by id and new lanes; everything else
//! about them is already here — on screen, or set aside by the relaying
//! that took them ([`SetAside`]).

use std::collections::HashSet;

use platitude_core::session::Carried;

use super::*;

/// A row a relaying took off the graph, with what the walk marked on it.
#[derive(Clone)]
pub(super) struct SetAside {
    item: GraphRowItem,
    published: bool,
    parents: Box<[Oid]>,
}

/// What a relaid row is marked with, held apart from the rows it is read
/// off: those are replaced in the same step.
struct Marked {
    oid: Oid,
    published: bool,
    parents: Box<[Oid]>,
    carried: Option<Carried>,
}

impl GraphModel {
    pub(super) fn relay_walk(
        &mut self,
        generation: u64,
        from: u64,
        rows: &[RelaidRow],
        walked: u32,
        truncated: bool,
    ) {
        if generation <= self.generation {
            return; // superseded by a newer stream
        }
        // Laid off a graph this model does not show, or naming a row it
        // never had: nothing to lay it from, so the graph is walked again.
        let laid = (from == self.generation)
            .then(|| self.relaid_items(rows))
            .flatten();
        let Some((mut items, marked)) = laid else {
            tracing::warn!(
                from,
                on = self.generation,
                "a graph laid out again off rows this model does not hold"
            );
            crate::hub::with_session(self.tab_id, |s| s.restart_log());
            return;
        };
        self.generation = generation;
        self.set_aside_the_rest(&marked);
        self.mark_incoming(&mut items);
        self.match_count = items.iter().filter(|i| i.matched).count() as i32;
        self.max_lanes = rows.iter().map(relaid_width).max().unwrap_or(1).max(1);
        self.clear_marks();
        for mark in &marked {
            self.push_mark(
                mark.oid,
                mark.published,
                &mark.parents,
                mark.carried.as_ref(),
            );
        }
        self.index.sort();
        self.splice_notified(items);
        self.settle_ends();
        debug_assert_eq!(self.marks.len(), self.rows.len());
        // The walk's time stands: nothing was walked. So does a press of
        // the tail that is out — its wider walk has not landed, and a
        // footer taking the wait off would let a second press cancel it.
        let elapsed_ms = u64::try_from(self.total_ms).unwrap_or_default();
        let growing = self.growing;
        self.settle_footer(self.rows.len() as i32, elapsed_ms, walked, truncated);
        self.growing = growing;
        tracing::info!(
            total = self.row_total,
            set_aside = self.set_aside.len(),
            "graph laid out again in place"
        );
    }

    /// The rows as they are to stand, and their marks; `None` where one of
    /// them is nowhere here.
    fn relaid_items(&self, rows: &[RelaidRow]) -> Option<(Vec<GraphRowItem>, Vec<Marked>)> {
        let pr = crate::encode::pr_set();
        let avatars = crate::hub::AvatarUrls::current();
        let mut items = Vec::with_capacity(rows.len());
        let mut marked = Vec::with_capacity(rows.len());
        for row in rows {
            match row {
                RelaidRow::Moved { oid, lanes, labels } => {
                    let (mut item, published, parents) = match self.row_at(oid) {
                        Some(at) => (
                            self.rows.get(at)?.clone(),
                            self.published_row(at),
                            Box::from(self.parents_of(at)),
                        ),
                        None => {
                            let aside = self.set_aside.get(oid)?;
                            (aside.item.clone(), aside.published, aside.parents.clone())
                        }
                    };
                    item.node_lane = i32::from(lanes.node_lane);
                    item.node_color = i32::from(lanes.node_color);
                    item.geometry = crate::encode::lanes_of(&lanes.segments);
                    item.labels = crate::encode::chips_of(labels, pr);
                    items.push(item);
                    marked.push(Marked {
                        oid: *oid,
                        published,
                        parents,
                        carried: None,
                    });
                }
                RelaidRow::Made(row) => {
                    items.push(to_row_item(row, &avatars, pr));
                    marked.push(Marked {
                        oid: Oid::from_hex_str(&row.oid_hex)
                            .unwrap_or_else(|_| Oid::zero_unsized()),
                        published: row.published,
                        parents: row.parents.clone(),
                        carried: row.carried.clone(),
                    });
                }
            }
        }
        Some((items, marked))
    }

    /// Keeps what the rows on screen held that the new rows do not, and
    /// hands back what they take up again.
    fn set_aside_the_rest(&mut self, kept: &[Marked]) {
        let staying: HashSet<Oid> = kept.iter().map(|mark| mark.oid).collect();
        for oid in &staying {
            self.set_aside.remove(oid);
        }
        for (at, item) in self.rows.iter().enumerate() {
            let Some(mark) = self.marks.get(at) else {
                continue;
            };
            let oid = mark.oid();
            if staying.contains(&oid) || Oid::hex_is_zero(&item.oid_hex) {
                continue;
            }
            self.set_aside.insert(
                oid,
                SetAside {
                    item: item.clone(),
                    published: self.published_row(at),
                    parents: Box::from(self.parents_of(at)),
                },
            );
        }
    }

    /// A replacement landed (`replace_walk`, after its marks): what it drew
    /// is no longer aside. The rest stays: a walk that lands while a delete
    /// is out keeps that delete's commits off (`session::leaving`), and a
    /// refusal after it lays them back by id. A reset lets go of it all
    /// (`start_walk`).
    pub(super) fn drawn_again_from_set_aside(&mut self) {
        let drawn = &self.index;
        self.set_aside
            .retain(|oid, _| drawn.binary_search_by(|(at, _)| at.cmp(oid)).is_err());
    }
}

fn relaid_width(row: &RelaidRow) -> i32 {
    match row {
        RelaidRow::Moved { lanes, .. } => i32::from(lanes.width),
        RelaidRow::Made(row) => i32::from(row.width),
    }
}

#[cfg(test)]
mod tests {
    use platitude_core::graph::{GraphBuilder, GraphRow};
    use platitude_core::session::{LogRow, RelaidRow};

    use super::super::GraphModel;
    use super::Oid;

    fn oid(n: u8) -> Oid {
        Oid::from_hex_str(&format!("{n:02x}").repeat(20)).unwrap()
    }

    /// Rows as a walk gives them, children first: `(id, parents)`.
    fn walk(rows: &[(u8, &[u8])]) -> Vec<LogRow> {
        let mut builder = GraphBuilder::new();
        rows.iter()
            .map(|(id, parents)| {
                let mine = oid(*id);
                let parents: Box<[Oid]> = parents.iter().map(|p| oid(*p)).collect();
                let g = builder.push_ids(&mine, &parents, false);
                LogRow {
                    row: g.row,
                    oid_hex: mine.to_hex(),
                    short_sha: mine.short_hex(8),
                    author: "Test".to_string(),
                    author_email: "test@example.com".to_string(),
                    co_authors: Vec::new(),
                    time: 1_700_000_000 + i64::from(*id),
                    subject: format!("commit {id:02x}"),
                    body: String::new(),
                    node_lane: g.node_lane,
                    node_color: g.node_color,
                    width: g.width,
                    segments: g.segments,
                    labels: Vec::new(),
                    stash_ref: String::new(),
                    published: *id == 1,
                    carried: None,
                    parents,
                    provisional: false,
                }
            })
            .collect()
    }

    /// main 1 ─ 2 ─ 3, and a branch forked at 2: 4 ─ 5.
    const FORKED: &[(u8, &[u8])] = &[(5, &[4]), (3, &[2]), (4, &[2]), (2, &[1]), (1, &[])];
    /// The same without the branch.
    const MAIN: &[(u8, &[u8])] = &[(3, &[2]), (2, &[1]), (1, &[])];

    /// What the session sends for `rows`, laid by a fresh builder.
    fn relaid(rows: &[(u8, &[u8])]) -> Vec<RelaidRow> {
        walk(rows)
            .into_iter()
            .map(|row| RelaidRow::Moved {
                oid: Oid::from_hex_str(&row.oid_hex).unwrap(),
                lanes: GraphRow {
                    row: row.row,
                    node_lane: row.node_lane,
                    node_color: row.node_color,
                    segments: row.segments,
                    width: row.width,
                },
                labels: Vec::new(),
            })
            .collect()
    }

    fn on_screen(model: &GraphModel) -> Vec<(String, i32, i32)> {
        model
            .rows
            .iter()
            .map(|r| (r.subject.clone(), r.node_lane, r.node_color))
            .collect()
    }

    fn walked_model() -> GraphModel {
        let mut model = GraphModel::default();
        let rows = walk(FORKED);
        model.replace_walk(1, &rows, 0, rows.len() as u32, false);
        model
    }

    #[test]
    fn a_relaying_keeps_the_rows_it_names_on_their_new_lanes() {
        let mut model = walked_model();

        model.relay_walk(2, 1, &relaid(MAIN), 3, false);

        let expected: Vec<_> = walk(MAIN)
            .iter()
            .map(|r| {
                (
                    r.subject.clone(),
                    i32::from(r.node_lane),
                    i32::from(r.node_color),
                )
            })
            .collect();
        assert_eq!(on_screen(&model), expected);
        assert_eq!((model.generation, model.walked_total), (2, 3));
        // The marks moved with the rows: found by id, with the parents and
        // the remote's mark the walk left on them.
        assert_eq!(model.row_at(&oid(2)), Some(1));
        assert_eq!(model.parents_of(1), [oid(1)]);
        assert!(model.published_row(2));
        assert_eq!(
            model.row_at(&oid(5)),
            None,
            "a row taken away is still found"
        );
    }

    /// A refused delete lays the rows it took back out, and they come back
    /// whole — the model is the only one still holding them.
    #[test]
    fn rows_a_relaying_took_come_back_whole() {
        let mut model = walked_model();
        let before = on_screen(&model);
        model.relay_walk(2, 1, &relaid(MAIN), 3, false);

        model.relay_walk(3, 2, &relaid(FORKED), 5, false);

        assert_eq!(on_screen(&model), before);
        assert_eq!(model.row_at(&oid(5)), Some(0));
        assert_eq!(model.parents_of(0), [oid(4)]);
        assert!(
            model.set_aside.is_empty(),
            "the rows set aside were kept past their return"
        );
    }

    /// Laid off a graph this model is not on, there is nothing to take the
    /// rows from: they stand as they are, for the walk it asks for.
    #[test]
    fn a_relaying_off_another_graph_moves_nothing() {
        let mut model = walked_model();
        let before = on_screen(&model);

        model.relay_walk(3, 2, &relaid(MAIN), 3, false);

        assert_eq!(on_screen(&model), before);
        assert_eq!(model.generation, 1);
    }

    /// A walk landing while the delete is out draws the graph without its
    /// commits too, and the refusal after it still finds them.
    #[test]
    fn a_walk_between_keeps_the_rows_for_the_refusal() {
        let mut model = walked_model();
        let before = on_screen(&model);
        model.relay_walk(2, 1, &relaid(MAIN), 3, false);

        model.replace_walk(3, &walk(MAIN), 0, 3, false);
        assert_eq!(model.set_aside.len(), 2);
        model.relay_walk(4, 3, &relaid(FORKED), 5, false);

        assert_eq!(on_screen(&model), before);
        assert!(model.set_aside.is_empty());
    }

    /// A walk that draws a row again takes it back, and keeps the rest.
    #[test]
    fn a_walk_takes_back_what_it_draws() {
        let mut model = walked_model();
        model.relay_walk(2, 1, &relaid(MAIN), 3, false);

        model.replace_walk(
            3,
            &walk(&[(4, &[2]), (3, &[2]), (2, &[1]), (1, &[])]),
            0,
            4,
            false,
        );
        assert_eq!(
            model.set_aside.keys().copied().collect::<Vec<_>>(),
            [oid(5)]
        );
    }

    /// A relaying leaves a press of the graph's tail waiting on its wider
    /// walk: taking the wait off would let a second press cancel it.
    #[test]
    fn a_relaying_leaves_a_press_of_the_tail_out() {
        let mut model = walked_model();
        model.growing = true;

        model.relay_walk(2, 1, &relaid(MAIN), 3, false);

        assert!(model.growing);
    }
}
