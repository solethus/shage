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

/// The floor for each fixture, as last blessed.
#[derive(Debug, Default)]
pub struct Baseline {
    rows: BTreeMap<String, (f64, f64)>,
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
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split_whitespace();
            if let (Some(name), Some(precision), Some(recall)) =
                (fields.next(), fields.next(), fields.next())
                && let (Ok(precision), Ok(recall)) = (precision.parse(), recall.parse())
            {
                rows.insert(name.to_owned(), (precision, recall));
            }
        }
        Ok(Self { rows })
    }

    /// Why `score` is a regression against this floor, or `None` if it is not.
    ///
    /// A fixture the floor has never seen is **not** a regression — it is new work, and
    /// failing on it would mean a new fixture could never be added without a red build.
    /// The caller says so out loud instead.
    pub fn regression(&self, fixture: &str, score: &Score) -> Option<String> {
        let (floor_precision, floor_recall) = self.rows.get(fixture)?;
        let mut fallen = Vec::new();
        if score.precision() < floor_precision - EPSILON {
            fallen.push(format!(
                "precision {:.2} below {floor_precision:.2}",
                score.precision()
            ));
        }
        if score.recall() < floor_recall - EPSILON {
            fallen.push(format!(
                "recall {:.2} below {floor_recall:.2}",
                score.recall()
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
         # fixture              precision  recall\n",
    );
    for (name, score) in measured {
        let _ = writeln!(
            out,
            "{name:<22} {:>9.2} {:>7.2}",
            score.precision(),
            score.recall()
        );
    }
    fs::write(root.join(PATH), out)
}
