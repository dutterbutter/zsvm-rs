use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

/// Remove a zksolc version, or "all" to remove all versions.
#[derive(Clone, Debug, Parser)]
pub struct RemoveCmd {
    /// zksolc version to remove, or "all" to remove all versions.
    pub version: String,
}

impl RemoveCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        if self.version.to_ascii_lowercase() == "all" {
            for v in zksvm::installed_versions().unwrap_or_default() {
                zksvm::remove_version(&v)?;
            }
            zksvm::unset_global_version()?;
            return Ok(());
        } else {
            let mut installed_versions = zksvm::installed_versions().unwrap_or_default();
            let current_version = zksvm::get_global_version()?;
            let version = Version::parse(&self.version)?;

            if installed_versions.contains(&version) {
                let input: String = Input::new()
                    .with_prompt("Are you sure?")
                    .with_initial_text("Y")
                    .default("N".into())
                    .interact_text()?;
                if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
                    zksvm::remove_version(&version)?;
                    if let Some(v) = current_version {
                        if version == v {
                            if let Some(i) = installed_versions.iter().position(|x| *x == v) {
                                installed_versions.remove(i);
                                if let Some(new_version) = installed_versions.pop() {
                                    zksvm::set_global_version(&new_version)?;
                                    print::set_global_version(&new_version);
                                } else {
                                    zksvm::unset_global_version()?;
                                }
                            }
                        }
                    }
                }
            } else {
                print::version_not_found(&version);
            }
        }

        Ok(())
    }
}
