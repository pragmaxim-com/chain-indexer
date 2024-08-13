use clap::{Parser, ValueEnum};
use core::fmt;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "blockchain-cli")]
pub struct CliConfig {
    #[arg(value_enum, long)]
    pub blockchain: Blockchain,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum Blockchain {
    Bitcoin,
    Cardano,
}

impl fmt::Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Blockchain::Bitcoin => "citcoin",
            Blockchain::Cardano => "cardano",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Blockchain {
    type Err = String;

    fn from_str(input: &str) -> Result<Blockchain, Self::Err> {
        match input.to_lowercase().as_str() {
            "bitcoin" => Ok(Blockchain::Bitcoin),
            "cardano" => Ok(Blockchain::Cardano),
            _ => Err(format!("Unknown blockchain: {}", input)),
        }
    }
}