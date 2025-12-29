use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, value_enum)]
    pub mode: Mode,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Mode {
    Client,
    Server,
}
