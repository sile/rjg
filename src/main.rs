use std::{collections::HashMap, io, num::NonZeroUsize, str::FromStr};

use clap::Parser;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde_json::Value;

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

    json: Json,
}

fn main() {
    let args = Args::parse();
    let mut rng = ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(rand::random));
    for i in 0..args.count.get() {
        let mut generator = Generator::new(i, &args.prefix, &mut rng);
        match generator.generate(&args.var, &args.json.0) {
            Ok(json) => {
                println!("{json}");
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug)]
pub struct Generator<'a> {
    prefix: &'a str,
    rng: &'a mut ChaChaRng,
    vars: HashMap<&'a str, Value>,
}

impl<'a> Generator<'a> {
    fn new(i: usize, prefix: &'a str, rng: &'a mut ChaChaRng) -> Self {
        let mut vars = HashMap::new();
        vars.insert("i", Value::Number(i.into()));
        Self { prefix, rng, vars }
    }

    fn generate(&mut self, vars: &'a [Var], json: &Value) -> Result<Value, String> {
        for var in vars {
            let value = self.eval_json(&var.value)?;
            self.vars.insert(&var.name, value);
        }
        self.eval_json(json)
    }

    fn eval_json(&mut self, json: &Value) -> Result<Value, String> {
        match json {
            Value::Null => Ok(Value::Null),
            Value::Bool(v) => Ok(Value::Bool(*v)),
            Value::Number(v) => Ok(Value::Number(v.clone())),
            Value::String(v) => self.eval_string(v),
            Value::Array(vs) => vs.iter().map(|v| self.eval_json(v)).collect(),
            Value::Object(vs) => vs
                .iter()
                .map(|(k, v)| Ok((k, self.eval_json(v)?)))
                .collect(),
        }
    }

    fn eval_string(&mut self, s: &str) -> Result<Value, String> {
        if !s.starts_with(self.prefix) {
            return Ok(Value::String(s.to_owned()));
        }

        let s = &s[self.prefix.len()..];
        if s.starts_with("$") {
            self.resolve_var(&s[1..])
        } else {
            todo!();
        }
    }

    fn resolve_var(&self, name: &str) -> Result<Value, String> {
        self.vars
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined variable {name:?}"))
    }
}

#[derive(Debug, Clone)]
struct Json(Value);

impl FromStr for Json {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<Self> {
        serde_json::from_str(s)
            .map(Json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }
}

#[derive(Debug, Clone)]
struct Var {
    name: String,
    value: Value,
}

impl FromStr for Var {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<Self> {
        let mut parts = s.splitn(2, '=');
        let name = parts.next().expect("unreachable").to_owned();
        let value = parts
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing '='"))?;
        let value = serde_json::from_str(value).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid value JSON ({e})"),
            )
        })?;
        Ok(Var { name, value })
    }
}
