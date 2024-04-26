use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

/// Set a zksolc version as the global default.
#[derive(Clone, Debug, Parser)]
pub struct UseCmd {
    /// zksolc version to set as the global default.
    pub version: String,
}

impl UseCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let version = Version::parse(&self.version)?;
        let all_versions = zksvm::all_versions().await?;
        let installed_versions = zksvm::installed_versions().unwrap_or_default();
        let current_version = zksvm::get_global_version()?;

        if installed_versions.contains(&version) {
            zksvm::set_global_version(&version)?;
            print::set_global_version(&version);
        } else if all_versions.contains(&version) {
            println!("Solc {version} is not installed");
            let input: String = Input::new()
                .with_prompt("Would you like to install it?")
                .with_initial_text("Y")
                .default("N".into())
                .interact_text()?;
            if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
                let spinner = print::installing_version(&version);
                zksvm::install(&version).await?;
                spinner.finish_with_message(format!("Downloaded zksolc: {version}"));
                if current_version.is_none() {
                    zksvm::set_global_version(&version)?;
                    print::set_global_version(&version);
                }
            }
        } else {
            print::unsupported_version(&version);
        }

        Ok(())
    }
}
