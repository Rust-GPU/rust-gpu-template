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
    env_logger::init();
    let command = Command::try_parse()?;
    debug!("Command: {command:?}");
    match command {
        Command::Generate(generate) => generate.run(),
    }
}
