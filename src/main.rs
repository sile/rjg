use std::{collections::HashMap, io, num::NonZeroUsize, str::FromStr};

use clap::Parser;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng, SeedableRng,
};
use rand_chacha::ChaChaRng;
use serde::{de::Error, Deserialize, Serialize};
use serde_json::Value;

#[derive(Parser)]
#[clap(version)]
struct Args {
    #[clap(short, long, default_value = "1")]
    count: NonZeroUsize,

    #[clap(short, long, default_value = "$")]
    prefix: String,

    #[clap(short, long)]
    var: Vec<Var>,

    #[clap(short, long)]
    seed: Option<u64>,

    json: Json,
}

fn main() {
    let args = Args::parse();
    let mut generator = Generator::new(&args);
    let mut rng = ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(rand::random));
    for i in 0..args.count.get() {
        match generator.generate(&mut rng, i, &args.json.0) {
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
struct Context<'a> {
    rng: &'a mut ChaChaRng,
    eval_stack: Vec<String>,
    quote_val: bool,
}

impl<'a> Context<'a> {
    fn new(rng: &'a mut ChaChaRng) -> Self {
        Self {
            rng,
            eval_stack: Vec::new(),
            quote_val: false,
        }
    }
}

#[derive(Debug)]
pub struct Generator {
    prefix: String,
    vars: HashMap<String, Value>,
}

impl Generator {
    fn new(args: &Args) -> Self {
        let prefix = &args.prefix;
        let mut vars = [
            ("u8", integer(prefix, 0, u8::MAX as i64)),
            ("u16", integer(prefix, 0, u16::MAX as i64)),
            ("u32", integer(prefix, 0, u32::MAX as i64)),
            ("i8", integer(prefix, i8::MIN as i64, i8::MAX as i64)),
            ("i16", integer(prefix, i16::MIN as i64, i16::MAX as i64)),
            ("i32", integer(prefix, i32::MIN as i64, i32::MAX as i64)),
            ("bool", oneof(prefix, &[Value::Bool(true), false.into()])),
            (
                "alpha_chars",
                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".into(),
            ),
            (
                "alphanum_chars",
                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".into(),
            ),
            ("digit_chars", "0123456789".into()),
        ]
        .into_iter()
        .map(|(k, v)| (format!("{}{k}", args.prefix), v))
        .collect::<HashMap<_, _>>();
        for var in &args.var {
            vars.insert(format!("{}{}", args.prefix, var.name), var.value.clone());
        }
        Self {
            prefix: args.prefix.clone(),
            vars,
        }
    }

    fn generate(&mut self, rng: &mut ChaChaRng, i: usize, json: &Value) -> Result<Value, String> {
        let mut ctx = Context::new(rng);
        self.vars
            .insert(format!("{}{i}", self.prefix), Value::Number(i.into()));
        self.eval_json(&mut ctx, json)
    }

    fn eval_json(&self, ctx: &mut Context, json: &Value) -> Result<Value, String> {
        match json {
            Value::Null => Ok(Value::Null),
            Value::Bool(v) => Ok(Value::Bool(*v)),
            Value::Number(v) => Ok(Value::Number(v.clone())),
            Value::String(v) => self.eval_string(ctx, v),
            Value::Array(vs) => vs.iter().map(|v| self.eval_json(ctx, v)).collect(),
            Value::Object(vs) => self.eval_object(ctx, vs),
        }
    }

    fn eval_object(
        &self,
        ctx: &mut Context,
        object: &serde_json::Map<String, Value>,
    ) -> Result<Value, String> {
        if object.len() == 1 {
            let (key, raw_value) = object.iter().next().expect("unreachable");
            let value = self.eval_json(ctx, raw_value)?;
            let invalid_generator_error =
                |e| format!("invalid generator: {{{key:?}: {value}}} ({e})");
            if key.starts_with(&self.prefix) {
                let value = match &key[self.prefix.len()..] {
                    "oneof" => {
                        let gen: OneofGenerator = serde_json::from_value(value.clone())
                            .and_then(OneofGenerator::validate)
                            .map_err(invalid_generator_error)?;
                        gen.generate(ctx)
                    }
                    "int" => {
                        let gen: IntegerGenerator = serde_json::from_value(value.clone())
                            .and_then(IntegerGenerator::validate)
                            .map_err(invalid_generator_error)?;
                        gen.generate(ctx)
                    }
                    "str" => {
                        let gen: StringGenerator = serde_json::from_value(value.clone())
                            .and_then(StringGenerator::validate)
                            .map_err(invalid_generator_error)?;
                        gen.generate(ctx)
                    }
                    "arr" => {
                        ctx.quote_val = true;
                        let value = self.eval_json(ctx, raw_value)?;
                        ctx.quote_val = false;

                        let gen: ArrayGenerator = serde_json::from_value(value.clone())
                            .map_err(invalid_generator_error)?;
                        gen.generate(ctx, self)?
                    }
                    _ => return Err(format!("unknown generator type: {key:?}")),
                };
                return Ok(value);
            }
        }

        let quote_val = std::mem::take(&mut ctx.quote_val);
        object
            .iter()
            .map(|(k, v)| {
                if quote_val && k == "val" {
                    Ok((k, v.clone()))
                } else {
                    Ok((k, self.eval_json(ctx, v)?))
                }
            })
            .collect()
    }

    fn eval_string(&self, ctx: &mut Context, s: &str) -> Result<Value, String> {
        if !s.starts_with(&self.prefix) {
            return Ok(Value::String(s.to_owned()));
        }

        self.resolve_var(ctx, s)
    }

    fn resolve_var(&self, ctx: &mut Context, name: &str) -> Result<Value, String> {
        let name = name.to_owned();
        if ctx.eval_stack.contains(&name) {
            ctx.eval_stack.push(name);
            return Err(format!(
                "circular reference: {}",
                ctx.eval_stack.join(" -> ")
            ));
        }
        ctx.eval_stack.push(name.clone());

        let value = self
            .vars
            .get(&name)
            .ok_or_else(|| format!("undefined variable: {name:?}"))?;
        let value = self.eval_json(ctx, value)?;
        ctx.eval_stack.pop();
        Ok(value)
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

fn oneof(prefix: &str, values: &[Value]) -> Value {
    OneofGenerator(values.to_owned()).to_json(prefix)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OneofGenerator(Vec<Value>);

impl OneofGenerator {
    fn to_json(self, prefix: &str) -> Value {
        let mut object = serde_json::Map::new();
        object.insert(
            format!("{prefix}oneof"),
            serde_json::to_value(self).expect("unreachable"),
        );
        Value::Object(object)
    }

    fn validate(self) -> Result<Self, serde_json::Error> {
        if self.0.is_empty() {
            return Err(serde_json::Error::custom("empty array"));
        }
        Ok(self)
    }

    fn generate(&self, ctx: &mut Context) -> Value {
        self.0.choose(ctx.rng).expect("unreachable").clone()
    }
}

fn integer(prefix: &str, min: i64, max: i64) -> Value {
    IntegerGenerator::new(min, max).to_json(prefix)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntegerGenerator {
    min: i64,
    max: i64,
}

impl IntegerGenerator {
    fn new(min: i64, max: i64) -> Self {
        Self { min, max }
    }

    fn to_json(self, prefix: &str) -> Value {
        let mut object = serde_json::Map::new();
        object.insert(
            format!("{prefix}int"),
            serde_json::to_value(self).expect("unreachable"),
        );
        Value::Object(object)
    }

    fn validate(self) -> Result<Self, serde_json::Error> {
        if self.min > self.max {
            return Err(serde_json::Error::custom("empty range"));
        }
        Ok(self)
    }

    fn generate(&self, ctx: &mut Context) -> Value {
        Value::Number(ctx.rng.gen_range(self.min..=self.max).into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StringGenerator {
    len: usize,
    chars: String,
}

impl StringGenerator {
    fn validate(self) -> Result<Self, serde_json::Error> {
        if self.chars.is_empty() {
            return Err(serde_json::Error::custom("empty chars"));
        }
        Ok(self)
    }

    fn generate(&self, ctx: &mut Context) -> Value {
        let mut s = String::new();
        for _ in 0..self.len {
            let c = self.chars.chars().choose(ctx.rng).expect("unreachable");
            s.push(c);
        }
        Value::String(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArrayGenerator {
    len: usize,
    val: Value,
}

impl ArrayGenerator {
    fn generate(&self, ctx: &mut Context, gen: &Generator) -> Result<Value, String> {
        let mut array = Vec::new();
        for _ in 0..self.len {
            let val = gen.eval_json(ctx, &self.val)?;
            array.push(val);
        }
        Ok(Value::Array(array))
    }
}

// TODO: ObjectGenerator
// TODO: OptionalGenerator
