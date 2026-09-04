//! Binary entrypoint: run the parsed invocation on the async runtime.

mod commands;
mod executor;
mod parser;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
  executor::execute(parser::parse()).await
}
