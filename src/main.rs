use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
    str::FromStr,
};

use clap::Parser;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

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

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rng = ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(rand::random));
    for i in 0..args.count.get() {
        let mut generator = Generator::new(i, &args.prefix, &mut rng);
        let json = generator.generate(&args.var, &args.json)?;
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
pub struct Generator<'a> {
    prefix: &'a str,
    rng: &'a mut ChaChaRng,
    vars: HashMap<String, serde_json::Value>,
}

impl<'a> Generator<'a> {
    fn new(i: usize, prefix: &'a str, rng: &'a mut ChaChaRng) -> Self {
        let mut vars = HashMap::new();
        vars.insert("i".to_owned(), serde_json::Value::Number(i.into()));
        Self { prefix, rng, vars }
    }

    fn generate(
        &mut self,
        input_vars: &[Var],
        input_json: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        todo!()
    }
}

#[derive(Debug, Clone)]
struct Var {
    name: String,
    value: serde_json::Value,
}

impl FromStr for Var {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts = s.splitn(2, '=');
        let name = parts.next().expect("unreachable").to_owned();
        let value = parts
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing '='"))?;
        let value = serde_json::from_str(value).map_err(|e| {
            Error::new(ErrorKind::InvalidInput, format!("invalid value JSON ({e})"))
        })?;
        Ok(Var { name, value })
    }
}
