//! The seven hazards, as the source of the projects that isolate them.
//!
//! One hazard per fixture, and as little else as possible: every extra function is another
//! edge in the oracle's denominator that is not testing anything. Calls are written on one
//! line each, because both tiers anchor a call site to a line and a call split over two
//! lines could anchor to different ones — a disagreement about formatting, reported as a
//! disagreement about resolution.

use super::Fixture;

/// Every fixture, in the order `cargo xtask fixtures` writes them and `oracle` grades them.
pub const ALL: &[Fixture] = &[
    A_INHERENT_SAME_NAME,
    B_GENERIC_BOUND,
    C_TRAIT_OBJECT,
    D_REEXPORT_CHAIN,
    E_ALIASED_IMPORT,
    F_MACRO_GENERATED,
    G_RENAMED_SYMBOL,
];

// `[workspace]` is not decoration: the fixtures live under `target/`, inside this
// repository's workspace, and cargo walks upward looking for one. Without the empty table
// every fixture fails with "believes it's in a workspace when it's not".
macro_rules! manifest {
    ($name:literal) => {
        concat!(
            "[package]\nname = \"",
            $name,
            "\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n"
        )
    };
}

/// Two inherent methods with the same name on different types. A resolver matching on the
/// method name alone cannot tell `a.run()` from `b.run()`.
///
/// `main` deliberately avoids `println!`: a call written inside a macro invocation is a
/// `token_tree` to tree-sitter, not a call expression, so it would smuggle a second hazard
/// — macro-argument calls — into a fixture that is supposed to isolate one. That hazard
/// belongs to `f-macro-generated`.
const A_INHERENT_SAME_NAME: Fixture = Fixture {
    name: "a-inherent-same-name",
    hazard: "two inherent `run` methods on different types",
    history: &[],
    files: &[
        ("Cargo.toml", manifest!("hazard-a")),
        (
            "src/alpha.rs",
            "pub struct Alpha;\n\nimpl Alpha {\n    pub fn run(&self) -> u32 {\n        1\n    }\n}\n",
        ),
        (
            "src/beta.rs",
            "pub struct Beta;\n\nimpl Beta {\n    pub fn run(&self) -> u32 {\n        2\n    }\n}\n",
        ),
        (
            "src/main.rs",
            "mod alpha;\nmod beta;\n\nuse alpha::Alpha;\nuse beta::Beta;\n\n\
             fn call_alpha() -> u32 {\n    let a = Alpha;\n    a.run()\n}\n\n\
             fn call_beta() -> u32 {\n    let b = Beta;\n    b.run()\n}\n\n\
             fn main() {\n    let total = call_alpha() + call_beta();\n    \
             std::process::exit(total as i32);\n}\n",
        ),
    ],
};

/// A trait method with two impls, called through a generic bound. The compiler resolves
/// `t.run()` to the trait's declaration; a name matcher sees three candidates.
const B_GENERIC_BOUND: Fixture = Fixture {
    name: "b-generic-bound",
    hazard: "trait method with two impls, called through `T: Task`",
    history: &[],
    files: &[
        ("Cargo.toml", manifest!("hazard-b")),
        (
            "src/lib.rs",
            "pub trait Task {\n    fn run(&self) -> u32;\n}\n\n\
             pub struct One;\n\npub struct Two;\n\n\
             impl Task for One {\n    fn run(&self) -> u32 {\n        1\n    }\n}\n\n\
             impl Task for Two {\n    fn run(&self) -> u32 {\n        2\n    }\n}\n\n\
             pub fn drive<T: Task>(t: &T) -> u32 {\n    t.run()\n}\n\n\
             pub fn drive_one() -> u32 {\n    drive(&One)\n}\n\n\
             pub fn drive_two() -> u32 {\n    drive(&Two)\n}\n",
        ),
    ],
};

/// A trait object call. The target is only known at run time, so the honest static answer
/// is the trait declaration — never one of the impls.
const C_TRAIT_OBJECT: Fixture = Fixture {
    name: "c-trait-object",
    hazard: "dynamic dispatch through `&dyn Handler`",
    history: &[],
    files: &[
        ("Cargo.toml", manifest!("hazard-c")),
        (
            "src/lib.rs",
            "pub trait Handler {\n    fn handle(&self) -> u32;\n}\n\n\
             pub struct First;\n\npub struct Second;\n\n\
             impl Handler for First {\n    fn handle(&self) -> u32 {\n        1\n    }\n}\n\n\
             impl Handler for Second {\n    fn handle(&self) -> u32 {\n        2\n    }\n}\n\n\
             pub fn dispatch(h: &dyn Handler) -> u32 {\n    h.handle()\n}\n\n\
             pub fn run_first() -> u32 {\n    dispatch(&First)\n}\n\n\
             pub fn run_second() -> u32 {\n    dispatch(&Second)\n}\n",
        ),
    ],
};

/// A `pub use` re-export chain three modules deep. The trap is resolving to the re-export
/// statement instead of the definition it forwards to.
const D_REEXPORT_CHAIN: Fixture = Fixture {
    name: "d-reexport-chain",
    hazard: "`pub use` chain three modules deep",
    history: &[],
    files: &[
        ("Cargo.toml", manifest!("hazard-d")),
        (
            "src/deep/inner/core.rs",
            "pub fn target() -> u32 {\n    7\n}\n",
        ),
        (
            "src/deep/inner/mod.rs",
            "pub mod core;\n\npub use core::target;\n",
        ),
        (
            "src/deep/mod.rs",
            "pub mod inner;\n\npub use inner::target;\n",
        ),
        (
            "src/lib.rs",
            "pub mod deep;\n\npub use deep::target;\n\n\
             pub fn caller() -> u32 {\n    target()\n}\n",
        ),
    ],
};

/// An aliased import. The name at the call site exists nowhere as a definition, and a
/// decoy with a plausible name sits next to the real target.
const E_ALIASED_IMPORT: Fixture = Fixture {
    name: "e-aliased-import",
    hazard: "`use inner::real as alias`, with a decoy beside the target",
    history: &[],
    files: &[
        ("Cargo.toml", manifest!("hazard-e")),
        (
            "src/inner.rs",
            "pub fn real() -> u32 {\n    5\n}\n\npub fn decoy() -> u32 {\n    6\n}\n",
        ),
        (
            "src/lib.rs",
            "mod inner;\n\nuse inner::real as alias;\n\n\
             pub fn caller() -> u32 {\n    alias()\n}\n",
        ),
    ],
};

/// A macro-generated function. The definition does not exist in the token stream a
/// syntactic pass can see, so the honest tier-1 answer is `Unresolved`.
const F_MACRO_GENERATED: Fixture = Fixture {
    name: "f-macro-generated",
    hazard: "function defined by `macro_rules!` expansion",
    history: &[],
    files: &[
        ("Cargo.toml", manifest!("hazard-f")),
        (
            "src/lib.rs",
            "macro_rules! make {\n    ($name:ident, $value:expr) => {\n        \
             pub fn $name() -> u32 {\n            $value\n        }\n    };\n}\n\n\
             make!(generated, 9);\n\n\
             pub fn caller() -> u32 {\n    generated()\n}\n",
        ),
    ],
};

/// A symbol renamed between two commits in the fixture's own history. Graded at HEAD like
/// every other fixture; the first commit is there so freshness work has a real index that
/// is one commit behind a real tree.
const G_RENAMED_SYMBOL: Fixture = Fixture {
    name: "g-renamed-symbol",
    hazard: "symbol renamed between two commits of the fixture's own history",
    history: &[
        ("Cargo.toml", manifest!("hazard-g")),
        (
            "src/lib.rs",
            "pub fn old_name() -> u32 {\n    3\n}\n\n\
             pub fn caller() -> u32 {\n    old_name()\n}\n",
        ),
    ],
    files: &[
        ("Cargo.toml", manifest!("hazard-g")),
        (
            "src/lib.rs",
            "pub fn new_name() -> u32 {\n    3\n}\n\n\
             pub fn caller() -> u32 {\n    new_name()\n}\n",
        ),
    ],
};
