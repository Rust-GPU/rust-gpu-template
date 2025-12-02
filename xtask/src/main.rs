use clap::Parser;
use log::debug;

#[derive(Parser, Debug)]
#[command(version, about)]
pub enum Command {}

pub fn main() -> anyhow::Result<()> {
    env_logger::init();
    let command = Command::try_parse()?;
    debug!("Command: {command:?}");
    match command {}
}
