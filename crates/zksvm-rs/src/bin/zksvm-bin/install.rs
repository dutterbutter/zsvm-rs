use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

/// Install zksolc versions.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct InstallCmd {
    /// zksolc versions to install.
    pub versions: Vec<String>,
}

impl InstallCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let all_versions = zksvm::all_versions().await?;

        for version in self.versions {
            let installed_versions = zksvm::installed_versions().unwrap_or_default();
            let current_version = zksvm::get_global_version()?;
            let version = Version::parse(&version)?;

            if installed_versions.contains(&version) {
                println!("zksolc {version} is already installed");
                let input: String = Input::new()
                    .with_prompt("Would you like to set it as the global version?")
                    .with_initial_text("Y")
                    .default("N".into())
                    .interact_text()?;
                if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
                    zksvm::set_global_version(&version)?;
                    print::set_global_version(&version);
                }
            } else if all_versions.contains(&version) {
                let spinner = print::installing_version(&version);
                zksvm::install(&version).await?;
                spinner.finish_with_message(format!("Downloaded zksolc: {version}"));
                if current_version.is_none() {
                    zksvm::set_global_version(&version)?;
                    print::set_global_version(&version);
                }
            } else {
                print::unsupported_version(&version);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_install() {
        let args: InstallCmd = InstallCmd::parse_from(["zksvm", "1.3.17", "1.3.16"]);
        assert_eq!(
            args,
            InstallCmd {
                versions: vec!["1.3.17".into(), "1.3.16".into()]
            }
        );
    }
}
