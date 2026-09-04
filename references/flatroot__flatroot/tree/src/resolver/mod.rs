//! The dependency resolver: expands the requested packages into their
//! full dependency closure, ordered so every package comes after its
//! dependencies — the order extraction follows.

mod edge;
mod env;
mod install_if;
mod name;
mod rich_dep;
mod transitive;
mod walker;

pub use edge::DepEdge;
pub use env::ResolutionEnv;
pub use walker::DepWalker;
pub(crate) use walker::Resolution;
