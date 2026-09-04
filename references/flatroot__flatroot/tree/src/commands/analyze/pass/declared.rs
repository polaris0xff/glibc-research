//! The declared pass: resolves the dependency closure the package
//! metadata declares, starting from the seed packages.

use anyhow::Result;

use flatroot::dep_tree::DepTree;
use flatroot::resolver::ResolutionEnv;
use flatroot::ui::RunVoice;

use super::super::ctx::AnalysisContext;

/// Whether the declared pass follows `Recommends:` and `Suggests:`
/// dependencies in addition to hard ones.
pub struct Options {
  /// Follow `Recommends:` dependencies when the source format
  /// publishes them (Debian / Ubuntu).
  pub include_recommends: bool,
  /// Follow `Suggests:` dependencies when the source format
  /// publishes them (Debian / Ubuntu).
  pub include_suggests: bool,
}

/// The pass that resolves the dependencies the package metadata
/// declares.
pub struct DeclaredPass;

impl DeclaredPass {
  /// Resolves the declared dependency closure of the seed packages into
  /// a `DepTree`, following `Recommends:` and `Suggests:` dependencies
  /// per `opts`.
  pub fn run(ctx: &AnalysisContext, opts: Options) -> Result<DepTree> {
    let cfg = ResolutionEnv {
      index: ctx.index,
      include_recommends: opts.include_recommends,
      include_suggests: opts.include_suggests,
      exclude: &[],
    };
    let pb = RunVoice::spinner("Resolving declared dependencies");
    let seed_names: Vec<String> = ctx.targets.iter().map(|t| t.name.clone()).collect();
    let tree = DepTree::walk(&cfg, &seed_names)?;
    RunVoice::finish_ok(&pb, format!("Resolved {} declared packages", tree.nodes.len()));
    Ok(tree)
  }
}
