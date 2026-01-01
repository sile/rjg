use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroUsize,
};

use nojson::{DisplayJson, Json, JsonParseError, JsonValueKind, RawJson, RawJsonValue};
use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};

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
            .then(|a| a.value().parse())?;

        let this = Self {
            count: noargs::opt("count")
                .short('c')
                .ty("INTEGER")
                .default("1")
                .doc("Number of JSON values to generate")
                .take(&mut args)
                .then(|a| a.value().parse())?,
            seed: noargs::opt("seed")
                .short('s')
                .ty("INTEGER")
                .doc("Seed for the random number generator")
                .take(&mut args)
                .present_and_then(|a| a.value().parse())?,
            var: std::iter::from_fn(|| {
                noargs::opt("var")
                    .short('v')
                    .ty("NAME=JSON_TEMPLATE")
                    .doc("User-defined variables")
                    .take(&mut args)
                    .present_and_then(|var| -> Result<_, String> {
                        let (name, value) = var.value().split_once('=').ok_or("missing '='")?;
                        let name = name.to_owned();
                        let value = ValueTemplate::parse(value, &prefix)?;
                        Ok(Var { name, value })
                    })
                    .transpose()
            })
            .collect::<Result<_, _>>()?,
            json_template: noargs::arg("JSON_TEMPLATE")
                .doc("JSON template used to generate values")
                .example(r#"[0, {"$int": {"min": 1, "max": 8}}, 9]"#)
                .take(&mut args)
                .then(|a| ValueTemplate::parse(a.value(), &prefix))?,
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
    let mut rng = ChaChaRng::seed_from_u64(args.seed.unwrap_or_else(|| {
        std::time::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_millis() as u64
    }));
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

#[derive(Debug)]
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

#[derive(Debug)]
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
                    .map(|(n, v)| Ok((n.try_into()?, ValueTemplate::new(v, prefix)?)))
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
            ObjectOrGenerator::Generator(gn) => gn.generate(rng, vars),
        }
    }
}

#[derive(Debug)]
enum Generator {
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

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        match self {
            Generator::Oneof(gn) => gn.generate(rng, vars),
            Generator::Int(gn) => gn.generate(rng, vars),
            Generator::Str(gn) => gn.generate(rng, vars),
            Generator::Arr(gn) => gn.generate(rng, vars),
            Generator::Obj(gn) => gn.generate(rng, vars),
            Generator::Option(gn) => gn.generate(rng, vars),
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

#[derive(Debug)]
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
        match self {
            Value::Null => None::<()>.fmt(f),
            Value::Boolean(v) => v.fmt(f),
            Value::Integer(v) => v.fmt(f),
            Value::Float(v) => v.fmt(f),
            Value::String(v) => v.fmt(f),
            Value::Array(v) => v.fmt(f),
            Value::Object(v) => v.fmt(f),
        }
    }
}

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

#[derive(Debug)]
struct Var {
    name: String,
    value: ValueTemplate,
}

#[derive(Debug)]
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

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        let i = (rng.next_u64() as usize) % self.0.len();
        self.0[i].generate(rng, vars)
    }
}

#[derive(Debug)]
struct IntegerGenerator {
    min: i64,
    max: i64,
}

impl IntegerGenerator {
    fn min_max(min: i64, max: i64) -> Self {
        Self { min, max }
    }

    fn new(raw: RawJsonValue<'_, '_>) -> Result<Self, JsonParseError> {
        let min_raw = raw.to_member("min")?.required()?;
        let max_raw = raw.to_member("max")?.required()?;

        let min: i64 = min_raw.try_into()?;
        let max: i64 = max_raw.try_into()?;

        if min > max {
            return Err(invalid(raw)("empty range"));
        }
        Ok(Self { min, max })
    }

    fn generate(&self, rng: &mut ChaChaRng, _vars: &Variables) -> Result<Value, String> {
        let v = rng.next_u64() % (self.max - self.min) as u64;
        Ok(Value::Integer(v as i64 + self.min))
    }
}

#[derive(Debug)]
struct StringGenerator(Vec<ValueTemplate>);

impl StringGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        raw.to_array()?
            .map(|v| ValueTemplate::new(v, prefix))
            .collect::<Result<_, _>>()
            .map(Self)
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        let mut s = String::new();
        for v in &self.0 {
            match v.generate(rng, vars)? {
                Value::Null => {}
                Value::String(v) => s.push_str(&v),
                v => s.push_str(&Json(v).to_string()),
            }
        }
        Ok(Value::String(s))
    }
}

#[derive(Debug)]
struct ArrayGenerator {
    len: ValueTemplate,
    val: ValueTemplate,
}

impl ArrayGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        let len_raw = raw.to_member("len")?.required()?;
        let val_raw = raw.to_member("val")?.required()?;

        Ok(Self {
            len: ValueTemplate::new(len_raw, prefix)?,
            val: ValueTemplate::new(val_raw, prefix)?,
        })
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        let len = self.len.generate(rng, vars)?;
        let Value::Integer(len) = len else {
            return Err(format!("Array length is not an integer: {}", Json(len)));
        };
        (0..len)
            .map(|_| self.val.generate(rng, vars))
            .collect::<Result<_, _>>()
            .map(Value::Array)
    }
}

#[derive(Debug)]
struct ObjectGenerator(Vec<ObjectMemberGenerator>);

impl ObjectGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        raw.to_array()?
            .map(|v| ObjectMemberGenerator::new(v, prefix))
            .collect::<Result<_, _>>()
            .map(Self)
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        self.0
            .iter()
            .filter_map(|v| v.generate(rng, vars).transpose())
            .collect::<Result<_, _>>()
            .map(Value::Object)
    }
}

#[derive(Debug)]
enum ObjectMemberGenerator {
    Null,
    Member { name: String, val: ValueTemplate },
    Generator { gn: Generator },
}

impl ObjectMemberGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        if raw.kind().is_null() {
            Ok(Self::Null)
        } else if let (Ok(name_member), Ok(val_member)) =
            (raw.to_member("name"), raw.to_member("val"))
        {
            if let (Ok(name_raw), Ok(val_raw)) = (name_member.required(), val_member.required()) {
                Ok(Self::Member {
                    name: name_raw.try_into()?,
                    val: ValueTemplate::new(val_raw, prefix)?,
                })
            } else {
                Err(invalid(raw)("missing 'name' or 'val'"))
            }
        } else if let Some((name, value)) = raw.to_object()?.next() {
            Ok(Self::Generator {
                gn: Generator::new(name, value, prefix)?,
            })
        } else {
            Err(invalid(raw)("empty object"))
        }
    }

    fn generate(
        &self,
        rng: &mut ChaChaRng,
        vars: &Variables,
    ) -> Result<Option<(String, Value)>, String> {
        match self {
            ObjectMemberGenerator::Null => Ok(None),
            ObjectMemberGenerator::Member { name, val } => {
                Ok(Some((name.clone(), val.generate(rng, vars)?)))
            }
            ObjectMemberGenerator::Generator { gn } => {
                let v = gn.generate(rng, vars)?;
                if matches!(v, Value::Null) {
                    return Ok(None);
                }
                let Value::Object(mut v) = v else {
                    return Err(format!("not object: {}", Json(v)));
                };
                let name = v
                    .remove("name")
                    .ok_or_else(|| format!("'name' member not found: {}", Json(&v)))?;
                let value = v
                    .remove("val")
                    .ok_or_else(|| format!("'val' member not found: {}", Json(&v)))?;
                let Value::String(name) = name else {
                    return Err(format!("'name' is not a string: {}", Json(&name)));
                };
                Ok(Some((name, value)))
            }
        }
    }
}

#[derive(Debug)]
struct OptionGenerator(ValueTemplate);

impl OptionGenerator {
    fn new(raw: RawJsonValue<'_, '_>, prefix: &str) -> Result<Self, JsonParseError> {
        ValueTemplate::new(raw, prefix).map(Self)
    }

    fn generate(&self, rng: &mut ChaChaRng, vars: &Variables) -> Result<Value, String> {
        if rng.next_u32() & 1 == 1 {
            self.0.generate(rng, vars)
        } else {
            Ok(Value::Null)
        }
    }
}
