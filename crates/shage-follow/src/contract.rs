//! What the follow panel is handed, and what it says about it.

use shage_index::contract::{CallSite, Confidence, IndexStamp};

/// One panel's worth of state: which symbol is being followed, what reaches it, and how
/// fresh the answer is.
///
/// The stamp is not optional. A list of call sites with no stamp beside it cannot tell a
/// reviewer whether an empty panel means "nothing calls this" or "the index never saw this
/// commit", and those are opposite conclusions.
#[derive(Debug, Clone)]
pub struct FollowPanel {
    /// The symbol being followed, as the backend displays it.
    pub following: String,
    /// The freshness of the answer below.
    pub stamp: IndexStamp,
    /// The call sites, in backend order. Sorting for display is the panel's business, not
    /// the backend's.
    pub rows: Vec<CallSite>,
}

impl FollowPanel {
    /// The freshness line, one of the states the index contract enumerates.
    ///
    /// Every state is a distinct sentence and none is the absence of another — "no index"
    /// is not an empty version of "index at a1b2c3d", it is a different fact. Commits are
    /// named rather than counted: on a stack of pull requests a distance is either
    /// expensive to compute or simply false.
    ///
    /// A backend that reads the working tree has no indexed commit and is not stale against
    /// one, so it reports the tree it read rather than "no index". Collapsing the two would
    /// print "no index" above a list of real rows, and would make a heuristic answer with
    /// genuinely zero callers render identically to one from a backend with nothing behind
    /// it — the distinction the whole stamp exists to keep.
    pub fn freshness(&self) -> String {
        match (&self.stamp.indexed_commit, &self.stamp.repo_commit) {
            (None, None) => "no index".to_owned(),
            (None, Some(repo)) => format!("no index, tree at {}", short(repo)),
            (Some(indexed), Some(repo)) if indexed != repo => {
                format!("index at {}, tree at {}", short(indexed), short(repo))
            }
            (Some(indexed), _) => format!("index at {}", short(indexed)),
        }
    }
}

/// The four badge words, one per [`Confidence`], fixed width so the column does not move
/// between rows.
///
/// `?` for unresolved, deliberately: it reads as "I do not know", which is what the variant
/// means, rather than as "there is nothing", which is what an empty row would imply.
pub fn badge(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Exact => "exact",
        Confidence::Heuristic => "heur",
        Confidence::Candidate => "cand",
        Confidence::Unresolved => "?",
    }
}

/// What a row says its target is.
///
/// A [`Confidence::Candidate`] row names the count rather than picking one: the panel is
/// not entitled to choose a candidate on the reviewer's behalf, and a row that showed only
/// the first would read exactly like a resolved one.
pub fn target_of(call: &CallSite) -> String {
    match call.target.one() {
        Some(target) => format!(
            "{} ({}:{})",
            target.display,
            target.path.display(),
            target.line
        ),
        None => match call.target.confidence() {
            Confidence::Candidate => {
                let count = candidates(call);
                format!("{count} candidates")
            }
            _ => "unresolved".to_owned(),
        },
    }
}

/// How many targets a candidate row offers. Zero for every other confidence.
pub fn candidates(call: &CallSite) -> usize {
    match &call.target {
        shage_index::contract::Resolution::Candidates(set) => set.as_slice().len(),
        _ => 0,
    }
}

fn short(commit: &str) -> String {
    commit.chars().take(7).collect()
}
