use std::io::Write;

use rand_chacha::ChaChaRng;
use rand_core::RngCore;

#[derive(Debug)]
pub enum ValueTemplate<'text, 'raw> {
    Literal(nojson::RawJsonValue<'text, 'raw>),
    Array(Vec<Self>),
    Object(Vec<(nojson::RawJsonValue<'text, 'raw>, Self)>),
    Generator(ValueGenerator<'text, 'raw>),
}

impl<'text, 'raw> ValueTemplate<'text, 'raw> {
    pub fn generate<W: Write>(&self, writer: &mut W, rng: &mut ChaChaRng) -> std::io::Result<()> {
        match self {
            Self::Literal(v) => write!(writer, "{}", v.as_raw_str())?,
            ValueTemplate::Array(v) => {
                write!(writer, "[")?;
                for (i, element) in v.iter().enumerate() {
                    if i != 0 {
                        write!(writer, ",")?;
                    }
                    element.generate(writer, rng)?;
                }
                write!(writer, "]")?;
            }
            ValueTemplate::Object(v) => {
                write!(writer, "{{")?;
                for (i, (name, value)) in v.iter().enumerate() {
                    if i != 0 {
                        write!(writer, ",")?;
                    }
                    write!(writer, "{}:", name.as_raw_str())?;
                    value.generate(writer, rng)?;
                }
                write!(writer, "}}")?;
            }
            ValueTemplate::Generator(g) => {
                g.generate(writer, rng)?;
            }
        }
        Ok(())
    }
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for ValueTemplate<'text, 'raw> {
    type Error = nojson::JsonParseError;

    fn try_from(raw: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        if let Some(generator) = ValueGenerator::try_parse(raw)? {
            return Ok(Self::Generator(generator));
        }

        match raw.kind() {
            nojson::JsonValueKind::Null
            | nojson::JsonValueKind::Boolean
            | nojson::JsonValueKind::Integer
            | nojson::JsonValueKind::Float
            | nojson::JsonValueKind::String => Ok(Self::Literal(raw)),
            nojson::JsonValueKind::Array => raw.try_into().map(Self::Array),
            nojson::JsonValueKind::Object => raw
                .to_object()?
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect::<Result<_, _>>()
                .map(Self::Object),
        }
    }
}

#[derive(Debug)]
pub enum ValueGenerator<'text, 'raw> {
    Integer {
        bits: usize,
        signed: bool,
    },
    String {
        chars: usize,
    },
    Oneof {
        choices: Vec<ValueTemplate<'text, 'raw>>,
    },
}

impl<'text, 'raw> ValueGenerator<'text, 'raw> {
    fn generate<W: Write>(&self, writer: &mut W, rng: &mut ChaChaRng) -> std::io::Result<()> {
        match self {
            Self::Integer { bits, signed } => {
                let mask = if *bits >= 64 {
                    u64::MAX
                } else {
                    (1u64 << bits) - 1
                };
                let value = rng.next_u64() & mask;

                if *signed && *bits > 0 {
                    let sign_bit = 1u64 << (*bits - 1);
                    if (value & sign_bit) != 0 {
                        let signed_value = (value as i64) - (1i64 << bits);
                        write!(writer, "{signed_value}")?;
                    } else {
                        write!(writer, "{value}")?;
                    }
                } else {
                    write!(writer, "{value}")?;
                }
            }
            Self::String { chars } => {
                const CHARSET: &[u8] =
                    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                write!(writer, "\"")?;
                for _ in 0..*chars {
                    let i = (rng.next_u32() as usize) % CHARSET.len();
                    write!(writer, "{}", CHARSET[i] as char)?;
                }
                write!(writer, "\"")?;
            }
            Self::Oneof { choices } => {
                let i = (rng.next_u32() as usize) % choices.len();
                choices[i].generate(writer, rng)?;
            }
        }
        Ok(())
    }

    fn try_parse(
        raw: nojson::RawJsonValue<'text, 'raw>,
    ) -> Result<Option<Self>, nojson::JsonParseError> {
        match raw.kind() {
            nojson::JsonValueKind::String if raw.as_raw_str().starts_with("\"$") => {
                let s = raw.to_unquoted_string_str()?;
                if let Some(bits) = s.strip_prefix("$i") {
                    let bits: usize = bits.parse().map_err(|e| raw.invalid(e))?;
                    if bits == 0 {
                        return Err(raw.invalid("signed integers must have at least 1 bit"));
                    }
                    if bits > 64 {
                        return Err(raw.invalid("signed integers must be <= 64 bits"));
                    }
                    Ok(Some(Self::Integer { bits, signed: true }))
                } else if let Some(bits) = s.strip_prefix("$u") {
                    let bits: usize = bits.parse().map_err(|e| raw.invalid(e))?;
                    if bits == 0 {
                        return Err(raw.invalid("unsigned integers must have at least 1 bit"));
                    }
                    if bits > 64 {
                        return Err(raw.invalid("unsigned integers must be <= 64 bits"));
                    }
                    Ok(Some(Self::Integer {
                        bits,
                        signed: false,
                    }))
                } else if let Some(s) = s.strip_prefix("$s[")
                    && let Some(chars) = s.strip_suffix(']')
                {
                    let chars: usize = chars.parse().map_err(|e| raw.invalid(e))?;
                    if chars == 0 {
                        return Err(raw.invalid("strings must have at least 1 character"));
                    }
                    Ok(Some(Self::String { chars }))
                } else {
                    Err(raw.invalid(
                        "unknown generator format; expected $i<bits>, $u<bits>, or $s[chars]",
                    ))
                }
            }
            nojson::JsonValueKind::Object => {
                if let Some(choices) = raw.to_member("$oneof")?.get() {
                    if choices.to_array()?.next().is_none() {
                        return Err(raw.invalid("$oneof must have at least one choice"));
                    }
                    Ok(Some(Self::Oneof {
                        choices: choices.try_into()?,
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}
