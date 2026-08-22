//! Generated cargo projects, one per resolution hazard.
//!
//! Generated, never committed. A checked-in `.scip` file or a tarball of a repository
//! drifts from the real encoding as the indexer moves, and then the oracle is grading
//! against a museum piece while production reads something else. The generator is the
//! fixture, and it is also the only readable description of what each hazard is.

pub mod hazards;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One generated cargo project isolating one resolution hazard.
pub struct Fixture {
    /// Directory name under `target/fixtures/`, and the row label in the oracle table.
    pub name: &'static str,
    /// What the fixture is hard for, in one line. Printed by `cargo xtask fixtures`.
    pub hazard: &'static str,
    /// An earlier revision, written and committed before `files`. Empty for every fixture
    /// whose hazard does not involve history.
    pub history: &'static [(&'static str, &'static str)],
    /// The project at HEAD, as (repository-relative path, contents).
    pub files: &'static [(&'static str, &'static str)],
}

/// Where generated fixtures live, relative to the repository root.
pub const DIR: &str = "target/fixtures";

impl Fixture {
    /// Writes the fixture at `root`, replacing any previous copy, and commits it.
    ///
    /// A fixture with `history` gets two commits so the second is a real rename over a real
    /// parent; every fixture gets at least one, because a backend's `stamp()` has to report
    /// a commit and a repository with none would make "no index" and "no commits"
    /// indistinguishable.
    pub fn generate(&self, root: &Path) -> io::Result<PathBuf> {
        let dir = root.join(DIR).join(self.name);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        git(&dir, &["init", "--quiet", "--initial-branch=main"])?;

        if !self.history.is_empty() {
            write_all(&dir, self.history)?;
            commit(&dir, "the revision before the rename")?;
        }
        write_all(&dir, self.files)?;
        commit(&dir, self.hazard)?;
        Ok(dir)
    }
}

fn write_all(dir: &Path, files: &[(&str, &str)]) -> io::Result<()> {
    for (path, contents) in files {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, contents)?;
    }
    Ok(())
}

fn commit(dir: &Path, message: &str) -> io::Result<()> {
    git(dir, &["add", "--all"])?;
    git(dir, &["commit", "--quiet", "--message", message])
}

/// Runs git with an identity and a fixed timestamp supplied on the command line.
///
/// Nothing is read from the developer's `~/.gitconfig` or environment: a fixture whose
/// commit hash depends on who generated it is a fixture that cannot be compared between
/// two machines, and a machine with no `user.email` set would fail here rather than in CI.
fn git(dir: &Path, args: &[&str]) -> io::Result<()> {
    const STAMP: &str = "2001-01-01T00:00:00+00:00";
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=shage fixtures"])
        .args(["-c", "user.email=fixtures@shage.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .env("GIT_AUTHOR_DATE", STAMP)
        .env("GIT_COMMITTER_DATE", STAMP)
        .status()?;
    if status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "git {} failed in {}",
        args.join(" "),
        dir.display()
    )))
}

/// Regenerates every fixture and reports where they landed.
pub fn run(root: &Path) -> io::Result<()> {
    for fixture in hazards::ALL {
        fixture.generate(root)?;
        println!("{:<22} {}", fixture.name, fixture.hazard);
    }
    println!("\n{} fixtures in {}/", hazards::ALL.len(), DIR);
    Ok(())
}
