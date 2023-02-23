#![warn(missing_docs)]

// Useful to have this public when using this as a library.
use structopt::StructOpt;

mod dht;
use dht::Dht;

/// Describes all the possible CLI arguments for `hc`, including external subcommands like `hc-scaffold`
#[allow(clippy::large_enum_variant)]
#[derive(Debug, StructOpt)]
// #[structopt(setting = structopt::clap::AppSettings::InferSubcommands)]
pub enum Opt {
    Dht(Dht),
}

impl Opt {
    /// Run this command
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Dht(cmd) => cmd.run()?,
        }
        Ok(())
    }
}
