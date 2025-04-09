use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroUsize,
    str::FromStr,
};

use nojson::{
    DisplayJson, FromRawJsonValue, Json, JsonParseError, JsonValueKind, RawJson, RawJsonValue,
};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

struct Args {
    count: NonZeroUsize,
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

        let prefix: String = noargs::opt("prefix")
            .short('p')
            .ty("STRING")
            .default("$")
            .doc("Prefix for variable and generator names")
            .take(&mut args)
            .parse()?;

        let this = Self {
            count: noargs::opt("count")
                .short('c')
                .ty("INTEGER")
                .default("1")
                .doc("Number of JSON values to generate")
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
                .parse_with(|a| ValueTemplate::parse(a.raw_value_or_empty(), &prefix))?,
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
    let Some(mut args) = Args::parse()? else {
        return Ok(());
    };
    let mut vars = Variables::new(&mut args);
    let mut rng = ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(rand::random));
    for i in 0..args.count.get() {
        vars.index = ValueTemplate::Integer(i as i64);
        match args.json_template.generate(&mut rng, &vars) {
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

impl StringOrVariable {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        let s = raw.to_unquoted_string_str()?;
        if let Some(name) = s.strip_prefix(prefix) {
            Ok(Self::Variable(name.to_owned()))
        } else {
            Ok(Self::String(s.into_owned()))
        }
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        match self {
            StringOrVariable::String(v) => Ok(Value::String(v.clone())),
            StringOrVariable::Variable(name) => vars.get(name)?.generate(rng, vars),
        }
    }
}

#[derive(Debug, Clone)]
enum ObjectOrGenerator {
    Object(BTreeMap<String, ValueTemplate>),
    Generator(Box<Generator>),
}

impl ObjectOrGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        if let Some((name, value)) = raw.to_object()?.next().filter(|(n, _)| {
            n.to_unquoted_string_str()
                .is_ok_and(|s| s.starts_with(prefix))
        }) {
            Ok(Self::Generator(Box::new(Generator::new(
                name, value, prefix,
            )?)))
        } else {
            Ok(Self::Object(
                raw.to_object()?
                    .map(|(n, v)| Ok((n.try_to()?, ValueTemplate::new(v, prefix)?)))
                    .collect::<Result<_, _>>()?,
            ))
        }
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        match self {
            ObjectOrGenerator::Object(v) => v
                .iter()
                .map(|(name, value)| Ok((name.clone(), value.generate(rng, vars)?)))
                .collect::<Result<_, _>>()
                .map(Value::Object),
            ObjectOrGenerator::Generator(generator) => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Generator {
    Oneof(OneofGenerator),
    Int(IntegerGenerator),
    Str(StringGenerator),
    Arr(ArrayGenerator),
    Obj(ObjectGenerator),
    Option(OptionGenerator),
}

impl Generator {
    fn new(
        name: RawJsonValue<'_, '_>,
        value: RawJsonValue<'_, '_>,
        prefix: &str,
    ) -> Result<Self, JsonParseError> {
        match name.to_unquoted_string_str()?.as_ref().strip_prefix(prefix) {
            Some("oneof") => OneofGenerator::new(value, prefix).map(Self::Oneof),
            Some("int") => IntegerGenerator::new(value).map(Self::Int),
            Some("str") => StringGenerator::new(value, prefix).map(Self::Str),
            Some("arr") => ArrayGenerator::new(value, prefix).map(Self::Arr),
            Some("obj") => ObjectGenerator::new(value, prefix).map(Self::Obj),
            Some("option") => OptionGenerator::new(value, prefix).map(Self::Option),
            _ => Err(invalid(name)(format!("unknown generator name: {name}"))),
        }
    }
}

fn invalid<E>(raw: RawJsonValue<'_, '_>) -> impl FnOnce(E) -> JsonParseError
where
    E: Into<Box<dyn Send + Sync + std::error::Error>>,
{
    let kind = raw.kind();
    let position = raw.position();
    move |e| JsonParseError::InvalidValue {
        kind,
        position,
        error: e.into(),
    }
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

impl ValueTemplate {
    fn parse(text: &str, prefix: &str) -> Result<Self, String> {
        let json = RawJson::parse(text).map_err(|e| e.to_string())?;
        let raw = json.value();
        Self::new(raw, prefix).map_err(|e| e.to_string())
    }

    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        match raw.kind() {
            JsonValueKind::Null => Ok(Self::Null),
            JsonValueKind::Boolean => Ok(Self::Boolean(
                raw.as_boolean_str()?.parse().map_err(invalid(raw))?,
            )),
            JsonValueKind::Integer => Ok(Self::Integer(
                raw.as_integer_str()?.parse().map_err(invalid(raw))?,
            )),
            JsonValueKind::Float => Ok(Self::Float(
                raw.as_float_str()?.parse().map_err(invalid(raw))?,
            )),
            JsonValueKind::String => Ok(Self::String(StringOrVariable::new(raw, prefix)?)),
            JsonValueKind::Array => Ok(Self::Array(
                raw.to_array()?
                    .map(|v| Self::new(v, prefix))
                    .collect::<Result<_, _>>()?,
            )),
            JsonValueKind::Object => Ok(Self::Object(ObjectOrGenerator::new(raw, prefix)?)),
        }
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        match self {
            ValueTemplate::Null => Ok(Value::Null),
            ValueTemplate::Boolean(v) => Ok(Value::Boolean(*v)),
            ValueTemplate::Integer(v) => Ok(Value::Integer(*v)),
            ValueTemplate::Float(v) => Ok(Value::Float(*v)),
            ValueTemplate::String(v) => v.generate(rng, vars),
            ValueTemplate::Array(v) => v
                .iter()
                .map(|v| v.generate(rng, vars))
                .collect::<Result<_, _>>()
                .map(Value::Array),
            ValueTemplate::Object(v) => v.generate(rng, vars),
        }
    }
}

// TODO: remove
impl<'text> FromRawJsonValue<'text> for ValueTemplate {
    fn from_raw_json_value(
        _value: nojson::RawJsonValue<'text, '_>,
    ) -> Result<Self, JsonParseError> {
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
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

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
pub struct Variables {
    vars: HashMap<String, ValueTemplate>,
    index: ValueTemplate,
}

impl Variables {
    fn new(args: &mut Args) -> Self {
        let predefined = [
            (
                "u8",
                Generator::Int(IntegerGenerator::min_max(0, u8::MAX as i64)),
            ),
            (
                "u16",
                Generator::Int(IntegerGenerator::min_max(0, u16::MAX as i64)),
            ),
            (
                "u32",
                Generator::Int(IntegerGenerator::min_max(0, u32::MAX as i64)),
            ),
            (
                "i8",
                Generator::Int(IntegerGenerator::min_max(i8::MIN as i64, i8::MAX as i64)),
            ),
            (
                "i16",
                Generator::Int(IntegerGenerator::min_max(i16::MIN as i64, i16::MAX as i64)),
            ),
            (
                "i32",
                Generator::Int(IntegerGenerator::min_max(i32::MIN as i64, i32::MAX as i64)),
            ),
            (
                "i64",
                Generator::Int(IntegerGenerator::min_max(i64::MIN, i64::MAX)),
            ),
            ("digit", Generator::Int(IntegerGenerator::min_max(0, 9))),
            (
                "bool",
                Generator::Oneof(OneofGenerator(vec![
                    ValueTemplate::Boolean(true),
                    ValueTemplate::Boolean(false),
                ])),
            ),
            (
                "alpha",
                Generator::Oneof(OneofGenerator(
                    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
                        .chars()
                        .map(|c| ValueTemplate::String(StringOrVariable::String(c.to_string())))
                        .collect(),
                )),
            ),
        ];
        let vars = predefined
            .into_iter()
            .map(|(name, gn)| {
                (
                    name.to_owned(),
                    ValueTemplate::Object(ObjectOrGenerator::Generator(Box::new(gn))),
                )
            })
            .chain(args.var.drain(..).map(|v| (v.name, v.value)))
            .collect::<HashMap<_, _>>();
        Self {
            vars,
            index: ValueTemplate::Integer(0),
        }
    }

    fn get(&self, name: &str) -> Result<&ValueTemplate, String> {
        if name == "i" {
            Ok(&self.index)
        } else {
            self.vars
                .get(name)
                .ok_or_else(|| format!("unknown variable: {name}"))
        }
    }
}

// TODO: delete
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

#[derive(Debug, Clone)]
struct OneofGenerator(Vec<ValueTemplate>);

impl OneofGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        let choices = raw
            .to_array()?
            .map(|v| ValueTemplate::new(v, prefix))
            .collect::<Result<Vec<_>, _>>()?;
        if choices.is_empty() {
            return Err(invalid(raw)("empty array"));
        }
        Ok(Self(choices))
    }

    //     fn generate(&self, ctx: &mut Context) -> Value {
    //         self.0.choose(ctx.rng).expect("unreachable").clone()
    //     }
}

#[derive(Debug, Clone)]
struct IntegerGenerator {
    min: i64,
    max: i64,
}

impl IntegerGenerator {
    fn min_max(min: i64, max: i64) -> Self {
        Self { min, max }
    }

    fn new(raw: RawJsonValue<'_, '_>) -> Result<Self, JsonParseError> {
        let ([min, max], []) = raw.to_fixed_object(["min", "max"], [])?;
        let min: i64 = min.try_to()?;
        let max: i64 = max.try_to()?;
        if min > max {
            return Err(invalid(raw)("empty range"));
        }
        Ok(Self { min, max })
    }

    //     fn generate(&self, ctx: &mut Context) -> Value {
    //         Value::Number(ctx.rng.random_range(self.min..=self.max).into())
    //     }
}

#[derive(Debug, Clone)]
struct StringGenerator(Vec<ValueTemplate>);

impl StringGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        raw.to_array()?
            .map(|v| ValueTemplate::new(v, prefix))
            .collect::<Result<_, _>>()
            .map(Self)
    }

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
}

#[derive(Debug, Clone)]
struct ArrayGenerator {
    len: usize,
    val: ValueTemplate,
}

impl ArrayGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        let ([len, val], []) = raw.to_fixed_object(["len", "val"], [])?;
        Ok(Self {
            len: len.try_to()?,
            val: ValueTemplate::new(raw, prefix)?,
        })
    }

    //     fn generate(&self, ctx: &mut Context, gn: &Generator) -> Result<Value, String> {
    //         let mut array = Vec::new();
    //         for _ in 0..self.len {
    //             let val = gn.eval_json(ctx, &self.val)?;
    //             array.push(val);
    //         }
    //         Ok(Value::Array(array))
    //     }
}

#[derive(Debug, Clone)]
struct ObjectGenerator(Vec<ObjectMemberGenerator>);

impl ObjectGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        raw.to_array()?
            .map(|v| ObjectMemberGenerator::new(v, prefix))
            .collect::<Result<_, _>>()
            .map(Self)
    }

    //     fn generate(&self, _ctx: &mut Context) -> Value {
    //         self.0
    //             .iter()
    //             .filter_map(|m| m.as_ref().map(|m| (m.name.clone(), m.val.clone())))
    //             .collect()
    //     }
}

// TODO: remove clone
#[derive(Debug, Clone)]
enum ObjectMemberGenerator {
    Null,
    Member { name: String, val: ValueTemplate },
    Generator { gn: Generator },
}

impl ObjectMemberGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        if raw.kind().is_null() {
            Ok(Self::Null)
        } else if let Ok(([name, val], [])) = raw.to_fixed_object(["name", "val"], []) {
            Ok(Self::Member {
                name: name.try_to()?,
                val: ValueTemplate::new(raw, prefix)?,
            })
        } else if let Some((name, value)) = raw.to_object()?.next() {
            Ok(Self::Generator {
                gn: Generator::new(name, value, prefix)?,
            })
        } else {
            Err(invalid(raw)("empty object"))
        }
    }
}

#[derive(Debug, Clone)]
struct OptionGenerator(ValueTemplate);

impl OptionGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        ValueTemplate::new(raw, prefix).map(Self)
    }

    //     fn generate(&self, ctx: &mut Context) -> Value {
    //         if ctx.rng.random_bool(0.5) {
    //             self.0.clone()
    //         } else {
    //             Value::Null
    //         }
    //     }
}
