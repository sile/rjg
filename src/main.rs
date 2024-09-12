use std::{io::ErrorKind, num::NonZeroUsize, str::FromStr};

use clap::Parser;
use rand::SeedableRng;

#[derive(Parser)]
#[clap(version)]
struct Args {
    #[clap(short, long, default_value = "1")]
    count: NonZeroUsize,

    #[clap(short, long, default_value = "__")]
    prefix: String,

    #[clap(short, long)]
    var: Vec<Var>,

    #[clap(short, long)]
    seed: Option<u64>,

    json: serde_json::Value,
}

fn main() -> orfail::Result<()> {
    let args = Args::parse();

    let rng = rand_chacha::ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(rand::random));

    for _ in 0..args.count.get() {
        //println!("{:?}", args.json);
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Var {
    name: String,
    value: serde_json::Value,
}

impl FromStr for Var {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, '=');
        let name = parts.next().expect("unreachable").to_owned();
        let value = parts
            .next()
            .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "missing '='"))?;
        let value = serde_json::from_str(value).map_err(|e| {
            std::io::Error::new(ErrorKind::InvalidInput, format!("invalid value JSON ({e})"))
        })?;
        Ok(Var { name, value })
    }
}
