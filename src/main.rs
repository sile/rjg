use std::{
    collections::{BTreeMap, HashMap},
    io,
    num::NonZeroUsize,
    str::FromStr,
};

use nojson::{DisplayJson, FromRawJsonValue, Json, JsonParseError};
use rand::{Rng, SeedableRng, seq::IndexedRandom};
use rand_chacha::ChaChaRng;

struct Args {
    count: NonZeroUsize,
    prefix: String,
    seed: Option<u64>,
    var: Vec<Var>,
    json_template: ValueTemplate,
}

impl Args {
    fn parse() -> noargs::Result<Option<Self>> {
        let mut args = noargs::raw_args();
        args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
        args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

        noargs::HELP_FLAG.take_help(&mut args);
        if noargs::VERSION_FLAG.take(&mut args).is_present() {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }

        let this = Self {
            count: noargs::opt("count")
                .short('c')
                .ty("INTEGER")
                .default("1")
                .doc("Number of JSON values to generate")
                .take(&mut args)
                .parse()?,
            prefix: noargs::opt("prefix")
                .short('p')
                .ty("STRING")
                .default("$")
                .doc("Prefix for variable and generator names")
                .take(&mut args)
                .parse()?,
            seed: noargs::opt("seed")
                .short('s')
                .ty("INTEGER")
                .doc("Seed for the random number generator")
                .take(&mut args)
                .parse_if_present()?,
            var: {
                let mut vars = Vec::new();
                while let Some(var) = noargs::opt("var")
                    .short('v')
                    .ty("NAME=JSON_TEMPLATE")
                    .doc("User-defined variables")
                    .take(&mut args)
                    .parse_if_present()?
                {
                    vars.push(var);
                }
                vars
            },
            json_template: noargs::arg("JSON_TEMPLATE")
                .doc("JSON template used to generate values")
                .example(r#"[0, {"$int": {"min": 1, "max": 8}}, 9]"#)
                .take(&mut args)
                .parse()?,
        };

        if let Some(help) = args.finish()? {
            print!("{help}");
            Ok(None)
        } else {
            Ok(Some(this))
        }
    }
}

fn main() -> noargs::Result<()> {
    let Some(args) = Args::parse()? else {
        return Ok(());
    };
    let mut generator = Generator::new(&args);
    let mut rng = ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(rand::random));
    for i in 0..args.count.get() {
        match generator.generate(&mut rng, i, &args.json_template) {
            Ok(value) => {
                println!("{}", Json(value));
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum StringOrVariable {
    String(String),
    Variable(String),
}

#[derive(Debug, Clone)]
enum ObjectOrGenerator {
    Object(BTreeMap<String, ValueTemplate>),
    Generator,
}

#[derive(Debug, Clone)]
enum ValueTemplate {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(StringOrVariable),
    Array(Vec<ValueTemplate>),
    Object(ObjectOrGenerator),
}

impl<'text> FromRawJsonValue<'text> for ValueTemplate {
    fn from_raw_json_value(value: nojson::RawJsonValue<'text, '_>) -> Result<Self, JsonParseError> {
        todo!()
    }
}

impl FromStr for ValueTemplate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|Json(v)| v).map_err(|e| e.to_string())
    }
}

#[derive(Debug)]
pub struct Value {}

impl DisplayJson for Value {
    fn fmt(&self, f: &mut nojson::JsonFormatter<'_, '_>) -> std::fmt::Result {
        todo!()
    }
}

// #[derive(Debug)]
// struct Context<'a> {
//     rng: &'a mut ChaChaRng,
//     eval_stack: Vec<String>,
//     quote_val: bool,
// }

// impl<'a> Context<'a> {
//     fn new(rng: &'a mut ChaChaRng) -> Self {
//         Self {
//             rng,
//             eval_stack: Vec::new(),
//             quote_val: false,
//         }
//     }
// }

#[derive(Debug)]
pub struct Generator {
    // prefix: String,
    // predefined_vars: HashMap<String, Value>,
    // vars: HashMap<String, Value>,
}

impl Generator {
    fn new(args: &Args) -> Self {
        // let prefix = &args.prefix;
        // let mut predefined_vars = [
        //     ("u8", integer(prefix, 0, u8::MAX as i64)),
        //     ("u16", integer(prefix, 0, u16::MAX as i64)),
        //     ("u32", integer(prefix, 0, u32::MAX as i64)),
        //     ("i8", integer(prefix, i8::MIN as i64, i8::MAX as i64)),
        //     ("i16", integer(prefix, i16::MIN as i64, i16::MAX as i64)),
        //     ("i32", integer(prefix, i32::MIN as i64, i32::MAX as i64)),
        //     ("i64", integer(prefix, i64::MIN, i64::MAX)),
        //     ("digit", integer(prefix, 0, 9)),
        //     ("bool", oneof(prefix, &[Value::Bool(true), false.into()])),
        //     (
        //         "alpha",
        //         oneof(
        //             prefix,
        //             &"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        //                 .chars()
        //                 .map(|c| Value::String(c.to_string()))
        //                 .collect::<Vec<_>>(),
        //         ),
        //     ),
        // ]
        // .into_iter()
        // .map(|(k, v)| (format!("{}{k}", args.prefix), v))
        // .collect::<HashMap<_, _>>();
        // for var in &args.var {
        //     predefined_vars.insert(format!("{}{}", args.prefix, var.name), var.value.clone());
        // }
        // Self {
        //     prefix: args.prefix.clone(),
        //     predefined_vars,
        //     vars: HashMap::new(),
        // }
        todo!()
    }

    fn generate(
        &mut self,
        rng: &mut ChaChaRng,
        i: usize,
        json: &ValueTemplate,
    ) -> Result<Value, String> {
        // let mut ctx = Context::new(rng);
        // self.predefined_vars
        //     .insert(format!("{}i", self.prefix), Value::Number(i.into()));
        // self.vars = self.predefined_vars.clone();
        // self.eval_json(&mut ctx, json)
        todo!()
    }

    //     fn eval_json(&self, ctx: &mut Context, json: &Value) -> Result<Value, String> {
    //         match json {
    //             Value::Null => Ok(Value::Null),
    //             Value::Bool(v) => Ok(Value::Bool(*v)),
    //             Value::Number(v) => Ok(Value::Number(v.clone())),
    //             Value::String(v) => self.eval_string(ctx, v),
    //             Value::Array(vs) => vs.iter().map(|v| self.eval_json(ctx, v)).collect(),
    //             Value::Object(vs) => self.eval_object(ctx, vs),
    //         }
    //     }

    //     fn eval_object(
    //         &self,
    //         ctx: &mut Context,
    //         object: &serde_json::Map<String, Value>,
    //     ) -> Result<Value, String> {
    //         if object.len() == 1 {
    //             let (key, raw_value) = object.iter().next().expect("unreachable");
    //             let value = self.eval_json(ctx, raw_value)?;
    //             let invalid_generator_error =
    //                 |e| format!("invalid generator: {{{key:?}: {value}}} ({e})");
    //             if key.starts_with(&self.prefix) {
    //                 let value = match &key[self.prefix.len()..] {
    //                     "oneof" => {
    //                         let gn: OneofGenerator = serde_json::from_value(value.clone())
    //                             .and_then(OneofGenerator::validate)
    //                             .map_err(invalid_generator_error)?;
    //                         gn.generate(ctx)
    //                     }
    //                     "int" => {
    //                         let gn: IntegerGenerator = serde_json::from_value(value.clone())
    //                             .and_then(IntegerGenerator::validate)
    //                             .map_err(invalid_generator_error)?;
    //                         gn.generate(ctx)
    //                     }
    //                     "str" => {
    //                         let gn: StringGenerator = serde_json::from_value(value.clone())
    //                             .map_err(invalid_generator_error)?;
    //                         gn.generate(ctx)
    //                     }
    //                     "arr" => {
    //                         ctx.quote_val = true;
    //                         let value = self.eval_json(ctx, raw_value)?;
    //                         ctx.quote_val = false;

    //                         let gn: ArrayGenerator = serde_json::from_value(value.clone())
    //                             .map_err(invalid_generator_error)?;
    //                         gn.generate(ctx, self)?
    //                     }
    //                     "obj" => {
    //                         let gn: ObjectGenerator = serde_json::from_value(value.clone())
    //                             .map_err(invalid_generator_error)?;
    //                         gn.generate(ctx)
    //                     }
    //                     "option" => {
    //                         let gn: OptionGenerator = serde_json::from_value(value.clone())
    //                             .map_err(invalid_generator_error)?;
    //                         gn.generate(ctx)
    //                     }
    //                     _ => return Err(format!("unknown generator: {key:?}")),
    //                 };
    //                 return Ok(value);
    //             }
    //         }

    //         let quote_val = std::mem::take(&mut ctx.quote_val);
    //         object
    //             .iter()
    //             .map(|(k, v)| {
    //                 if quote_val && k == "val" {
    //                     Ok((k, v.clone()))
    //                 } else {
    //                     Ok((k, self.eval_json(ctx, v)?))
    //                 }
    //             })
    //             .collect()
    //     }

    //     fn eval_string(&self, ctx: &mut Context, s: &str) -> Result<Value, String> {
    //         if !s.starts_with(&self.prefix) {
    //             return Ok(Value::String(s.to_owned()));
    //         }

    //         self.resolve_var(ctx, s)
    //     }

    //     fn resolve_var(&self, ctx: &mut Context, name: &str) -> Result<Value, String> {
    //         let name = name.to_owned();
    //         if ctx.eval_stack.contains(&name) {
    //             ctx.eval_stack.push(name);
    //             return Err(format!(
    //                 "circular reference: {}",
    //                 ctx.eval_stack.join(" -> ")
    //             ));
    //         }
    //         ctx.eval_stack.push(name.clone());

    //         let value = self
    //             .vars
    //             .get(&name)
    //             .ok_or_else(|| format!("undefined variable: {name:?}"))?;
    //         let value = self.eval_json(ctx, value)?;
    //         ctx.eval_stack.pop();
    //         Ok(value)
    //     }
}

// #[derive(Debug, Clone)]
// struct Json(Value);

// impl FromStr for Json {
//     type Err = io::Error;

//     fn from_str(s: &str) -> io::Result<Self> {
//         serde_json::from_str(s)
//             .map(Json)
//             .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
//     }
// }

#[derive(Debug, Clone)]
struct Var {
    name: String,
    value: ValueTemplate,
}

impl FromStr for Var {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s.split_once('=').ok_or_else(|| "missing '='".to_owned())?;
        let name = name.to_owned();
        let value = value.parse()?;
        Ok(Var { name, value })
    }
}

// fn oneof(prefix: &str, values: &[Value]) -> Value {
//     OneofGenerator(values.to_owned()).to_json(prefix)
// }

// #[derive(Debug, Clone)]
// struct OneofGenerator(Vec<Value>);

// impl OneofGenerator {
//     fn to_json(&self, prefix: &str) -> Value {
//         let mut object = serde_json::Map::new();
//         object.insert(
//             format!("{prefix}oneof"),
//             serde_json::to_value(self).expect("unreachable"),
//         );
//         Value::Object(object)
//     }

//     fn validate(self) -> Result<Self, serde_json::Error> {
//         if self.0.is_empty() {
//             return Err(serde_json::Error::custom("empty array"));
//         }
//         Ok(self)
//     }

//     fn generate(&self, ctx: &mut Context) -> Value {
//         self.0.choose(ctx.rng).expect("unreachable").clone()
//     }
// }

// fn integer(prefix: &str, min: i64, max: i64) -> Value {
//     IntegerGenerator::new(min, max).to_json(prefix)
// }

// #[derive(Debug, Clone)]
// struct IntegerGenerator {
//     min: i64,
//     max: i64,
// }

// impl IntegerGenerator {
//     fn new(min: i64, max: i64) -> Self {
//         Self { min, max }
//     }

//     fn to_json(&self, prefix: &str) -> Value {
//         let mut object = serde_json::Map::new();
//         object.insert(
//             format!("{prefix}int"),
//             serde_json::to_value(self).expect("unreachable"),
//         );
//         Value::Object(object)
//     }

//     fn validate(self) -> Result<Self, serde_json::Error> {
//         if self.min > self.max {
//             return Err(serde_json::Error::custom("empty range"));
//         }
//         Ok(self)
//     }

//     fn generate(&self, ctx: &mut Context) -> Value {
//         Value::Number(ctx.rng.random_range(self.min..=self.max).into())
//     }
// }

// #[derive(Debug, Clone)]
// struct StringGenerator(Vec<Value>);

// impl StringGenerator {
//     fn generate(&self, _ctx: &mut Context) -> Value {
//         let mut s = String::new();
//         for v in &self.0 {
//             match v {
//                 Value::Null => {}
//                 Value::String(v) => s.push_str(v),
//                 _ => s.push_str(&v.to_string()),
//             }
//         }
//         Value::String(s)
//     }
// }

// #[derive(Debug, Clone)]
// struct ArrayGenerator {
//     len: usize,
//     val: Value,
// }

// impl ArrayGenerator {
//     fn generate(&self, ctx: &mut Context, gn: &Generator) -> Result<Value, String> {
//         let mut array = Vec::new();
//         for _ in 0..self.len {
//             let val = gn.eval_json(ctx, &self.val)?;
//             array.push(val);
//         }
//         Ok(Value::Array(array))
//     }
// }

// #[derive(Debug, Clone)]
// struct ObjectGenerator(Vec<Option<ObjectMember>>);

// impl ObjectGenerator {
//     fn generate(&self, _ctx: &mut Context) -> Value {
//         self.0
//             .iter()
//             .filter_map(|m| m.as_ref().map(|m| (m.name.clone(), m.val.clone())))
//             .collect()
//     }
// }

// #[derive(Debug, Clone)]
// struct ObjectMember {
//     name: String,
//     val: Value,
// }

// #[derive(Debug, Clone)]
// struct OptionGenerator(Value);

// impl OptionGenerator {
//     fn generate(&self, ctx: &mut Context) -> Value {
//         if ctx.rng.random_bool(0.5) {
//             self.0.clone()
//         } else {
//             Value::Null
//         }
//     }
// }
