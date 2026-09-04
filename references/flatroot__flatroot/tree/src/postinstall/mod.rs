//! Post-install: the configuration a normal install performs after
//! copying files, turning an extracted tree into a rootfs that runs.

mod alpine;
mod arch;
mod debian;
mod hooks;
mod ldconfig;
mod plan;
mod rootfs;
mod script;
mod step_report;
mod strategy;
mod stubs;

pub use plan::{Phase, PostInstallPlan};
pub use rootfs::Rootfs;
pub use strategy::PostInstall;
