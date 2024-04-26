use std::collections::HashSet;

use crate::print;
use clap::Parser;
use semver::Version;

/// List all zksolc versions.
#[derive(Debug, Parser)]
pub struct ListCmd;

impl ListCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let all_versions = zksvm::all_versions().await?;
        let installed_versions = zksvm::installed_versions().unwrap_or_default();
        let current_version = zksvm::get_global_version()?;

        let a: HashSet<Version> = all_versions.iter().cloned().collect();
        let b: HashSet<Version> = installed_versions.iter().cloned().collect();
        let c = &a - &b;

        let mut available_versions = c.iter().cloned().collect::<Vec<Version>>();
        available_versions.sort();

        print::current_version(current_version);
        print::installed_versions(installed_versions);
        print::available_versions(available_versions);

        Ok(())
    }
}
