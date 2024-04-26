//! Main svm-rs binary entry point.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use clap::Parser;

mod install;
mod list;
mod print;
mod remove;
mod r#use;
mod utils;

/// zksolc version manager.
#[derive(Debug, Parser)]
#[clap(
    name = "zksvm",
    version = zksvm::VERSION_MESSAGE,
    next_display_order = None,
)]
enum Zksvm {
    List(list::ListCmd),
    Install(install::InstallCmd),
    Use(r#use::UseCmd),
    Remove(remove::RemoveCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Zksvm::parse();

    zksvm::setup_data_dir()?;

    match opt {
        Zksvm::List(cmd) => cmd.run().await?,
        Zksvm::Install(cmd) => cmd.run().await?,
        Zksvm::Use(cmd) => cmd.run().await?,
        Zksvm::Remove(cmd) => cmd.run().await?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Zksvm::command().debug_assert();
    }
}
