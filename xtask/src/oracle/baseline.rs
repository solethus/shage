//! The committed floor, and what counts as falling through it.
//!
//! A floor, never an equality. The pass under test will not match a compiler frontend, and
//! a test that demands it does gets deleted by whoever hits it on a Friday afternoon. What
//! is worth defending is that the numbers never go *down* without someone saying so.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use super::scoring::Score;

/// Two measurements never differ by less than this without being the same number.
///
/// The file records two decimals, so a measurement is compared against what was written,
/// not against a full-precision value nobody can read in a diff.
const EPSILON: f64 = 0.005;

/// One fixture's blessed floor.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Floor {
    precision: f64,
    recall: f64,
    /// How many edges the oracle found when this row was blessed.
    ///
    /// Floored like the other two, because it is the denominator of recall: if the oracle
    /// side stops seeing part of a fixture, `truth` shrinks, `hits / truth` goes *up*, and a
    /// fixture that has quietly stopped exercising its hazard reports a better score than
    /// the one that was blessed.
    truth: usize,
}

/// The floor for each fixture, as last blessed.
#[derive(Debug, Default)]
pub struct Baseline {
    rows: BTreeMap<String, Floor>,
}

/// Where the floor lives, relative to the repository root.
pub const PATH: &str = "xtask/oracle.baseline";

impl Baseline {
    /// Reads the floor. A missing file is an empty floor, not an error: the first run on a
    /// new checkout should report numbers and tell you to bless them, not fail to start.
    pub fn load(root: &Path) -> io::Result<Self> {
        let text = match fs::read_to_string(root.join(PATH)) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
        };
        let mut rows = BTreeMap::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // A row that does not parse is an error rather than a skip. Dropping it silently
            // turns the fixture into one the floor "has never seen", which `verdict` reports
            // as new work and does not fail on — so a typo in this file disables the gate it
            // exists to be, and nothing says so.
            let malformed = |why: &str| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{PATH}:{}: {why}: {line}", number + 1),
                )
            };
            let mut fields = line.split_whitespace();
            let (Some(name), Some(precision), Some(recall), Some(truth)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                return Err(malformed("expected `fixture precision recall truth`"));
            };
            let (Ok(precision), Ok(recall)) = (precision.parse::<f64>(), recall.parse::<f64>())
            else {
                return Err(malformed("precision and recall must be numbers"));
            };
            // NaN compares false against every bound, so an infinite or NaN floor is not a
            // lenient floor, it is no floor at all — and it would read as a passing row.
            if !precision.is_finite() || !recall.is_finite() {
                return Err(malformed("precision and recall must be finite"));
            }
            let Ok(truth) = truth.parse::<usize>() else {
                return Err(malformed("truth must be a whole number of edges"));
            };
            rows.insert(
                name.to_owned(),
                Floor {
                    precision,
                    recall,
                    truth,
                },
            );
        }
        Ok(Self { rows })
    }

    /// Why `score` is a regression against this floor, or `None` if it is not.
    ///
    /// A fixture the floor has never seen is **not** a regression — it is new work, and
    /// failing on it would mean a new fixture could never be added without a red build.
    /// The caller says so out loud instead.
    pub fn regression(&self, fixture: &str, score: &Score) -> Option<String> {
        let floor = self.rows.get(fixture)?;
        let mut fallen = Vec::new();
        if score.precision() < floor.precision - EPSILON {
            fallen.push(format!(
                "precision {:.2} below {:.2}",
                score.precision(),
                floor.precision
            ));
        }
        if score.recall() < floor.recall - EPSILON {
            fallen.push(format!(
                "recall {:.2} below {:.2}",
                score.recall(),
                floor.recall
            ));
        }
        // Exact, not epsilon: edges are counted, not measured. Fewer of them means the
        // fixture is exercising less than it was blessed exercising, whatever the ratios say.
        if score.truth < floor.truth {
            fallen.push(format!(
                "oracle found {} edges, was {} — the fixture is exercising less than it was",
                score.truth, floor.truth
            ));
        }
        (!fallen.is_empty()).then(|| fallen.join(", "))
    }

    /// Whether the floor has a row for this fixture.
    pub fn covers(&self, fixture: &str) -> bool {
        self.rows.contains_key(fixture)
    }
}

/// Rewrites the floor from a run's measurements.
pub fn bless(root: &Path, measured: &[(&str, Score)]) -> io::Result<()> {
    let mut out = String::from(
        "# cargo xtask oracle — the committed floor. Regenerate with `cargo xtask oracle --bless`.\n\
         #\n\
         # A floor, not an equality: the heuristic pass will never match a compiler frontend,\n\
         # and demanding parity is how a test gets deleted rather than fixed. The oracle fails\n\
         # when a measurement drops below its row, so these numbers only ever go up.\n\
         #\n\
         # fixture              precision  recall  edges\n",
    );
    for (name, score) in measured {
        let _ = writeln!(
            out,
            "{name:<22} {:>9.2} {:>7.2} {:>6}",
            score.precision(),
            score.recall(),
            score.truth
        );
    }
    fs::write(root.join(PATH), out)
}
