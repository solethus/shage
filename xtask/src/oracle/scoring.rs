//! Grading one backend's edges against another's.
//!
//! | quantity | rule |
//! |---|---|
//! | true positive | the site is in both, with the same target |
//! | false positive | the pass under test committed to an edge the oracle does not have |
//! | false negative | the oracle has an edge the pass under test did not commit to |
//! | precision | `TP / (TP + FP)`, defined as `1.00` when it committed to nothing |
//! | recall | `TP / |truth|`, defined as `1.00` when there is nothing to find |
//! | ambiguous | sites answered with a candidate set, reported beside the metrics |
//! | contained | of those, how many held the right answer |
//!
//! **A candidate set scores as a miss.** Counting one as a hit would let a pass reach
//! recall 1.00 by returning every symbol in the repository, and the panel already renders
//! a candidate set as "I do not know which" — the metric agrees with the badge rather than
//! flattering it. `contained` is reported separately so the gap between "no idea" and
//! "nearly there" stays visible without being scored.

use super::edges::{Edges, Loc};

/// How one pass did against the oracle on one fixture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Score {
    /// Edges the pass got right.
    pub hits: usize,
    /// Edges it committed to that the oracle does not have.
    pub wrong: usize,
    /// Oracle edges it did not commit to, whether by silence or by candidate set.
    pub missed: usize,
    /// Sites it answered with a candidate set.
    pub ambiguous: usize,
    /// Candidate sets that held the right answer.
    pub contained: usize,
    /// How many edges the oracle found. The denominator of recall, and the honest measure
    /// of whether a fixture is exercising anything at all.
    pub truth: usize,
}

impl Score {
    /// Of the edges it committed to, how many were right. `1.00` when it committed to none:
    /// a pass that answers nothing has told no lies, which recall is there to punish.
    pub fn precision(&self) -> f64 {
        let claimed = self.hits + self.wrong;
        if claimed == 0 {
            return 1.0;
        }
        self.hits as f64 / claimed as f64
    }

    /// Of the edges the oracle found, how many it got. `1.00` when the oracle found none,
    /// so an empty fixture reads as "nothing to miss" rather than as total failure.
    pub fn recall(&self) -> f64 {
        if self.truth == 0 {
            return 1.0;
        }
        self.hits as f64 / self.truth as f64
    }
}

/// What the pass under test said at a site where the oracle disagrees.
#[derive(Debug, Clone)]
pub enum Got {
    /// It resolved the site to something else.
    Wrong(Loc),
    /// It offered a set and committed to none of them.
    Ambiguous(Vec<Loc>),
    /// It said nothing at all, or never saw the site.
    Nothing,
}

/// One site the two passes do not agree on.
#[derive(Debug, Clone)]
pub struct Disagreement {
    /// Where the call is written.
    pub site: Loc,
    /// What the oracle resolved it to. `None` when only the pass under test saw an edge
    /// here, which is the shape of a false positive.
    pub expected: Option<Loc>,
    /// What the pass under test said instead.
    pub got: Got,
}

/// Grades `under_test` against `truth`, and lists every site they disagree on.
pub fn grade(truth: &Edges, under_test: &Edges) -> (Score, Vec<Disagreement>) {
    let mut score = Score {
        truth: truth.resolved.len(),
        ambiguous: under_test.ambiguous.len(),
        ..Score::default()
    };
    let mut disagreements = Vec::new();

    for (site, expected) in &truth.resolved {
        if under_test
            .resolved
            .contains(&(site.clone(), expected.clone()))
        {
            score.hits += 1;
            continue;
        }
        score.missed += 1;
        let got = match under_test.ambiguous.get(site) {
            Some(candidates) => {
                if candidates.contains(expected) {
                    score.contained += 1;
                }
                Got::Ambiguous(candidates.iter().cloned().collect())
            }
            // An edge the oracle also has at this site belongs to a *different* call on the
            // same line — `f() + g()` is one site and two edges — and reporting it here
            // would name a correct answer as the wrong one. For this expected edge the pass
            // produced nothing, and that is what the report says.
            None => under_test
                .resolved
                .iter()
                .find(|(at, to)| {
                    at == site && !truth.resolved.contains(&(site.clone(), to.clone()))
                })
                .map_or(Got::Nothing, |(_, to)| Got::Wrong(to.clone())),
        };
        disagreements.push(Disagreement {
            site: site.clone(),
            expected: Some(expected.clone()),
            got,
        });
    }

    for (site, to) in &under_test.resolved {
        if truth.resolved.contains(&(site.clone(), to.clone())) {
            continue;
        }
        score.wrong += 1;
        // A wrong target at a site the oracle also covers is already reported above, as the
        // `Got::Wrong` half of that site's disagreement. Only an edge at a site the oracle
        // has nothing at is new here — an invented call.
        if !truth.resolved.iter().any(|(at, _)| at == site) {
            disagreements.push(Disagreement {
                site: site.clone(),
                expected: None,
                got: Got::Wrong(to.clone()),
            });
        }
    }

    disagreements.sort_by(|a, b| a.site.cmp(&b.site));
    (score, disagreements)
}
