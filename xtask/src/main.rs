use crate::generate::Generate;
use clap::Parser;
use log::debug;

mod generate;

#[derive(Parser, Debug)]
#[command(version, about)]
pub enum Command {
    Generate(Generate),
}

pub fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let command = Command::try_parse()?;
    debug!("Command: {command:?}");
    match command {
        Command::Generate(generate) => generate.run(),
    }
}
