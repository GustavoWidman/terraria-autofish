#![feature(custom_inner_attributes)]

mod bot;
mod constants;
mod utils;

use clap::Parser;
use eyre::Result;

use crate::utils::cli::Args;
use crate::utils::config::config;
use crate::utils::log::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    Logger::init(args.verbosity);

    let config = config(&args.config)?;

    let (scanner, scanner_tx) = bot::scanner::MemoryScanner::new(config.clone())?;

    bot::fisher::Fisher::new(config, scanner_tx)?
        .run(scanner)
        .await?;

    todo!()
}
