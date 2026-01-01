#[derive(Debug)]
pub enum ValueTemplate<'text, 'raw> {
    Literal(nojson::RawJsonValue<'text, 'raw>),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
    Generator(ValueGenerator<'text, 'raw>),
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
                .map(|(k, v)| Ok((k.try_into()?, v.try_into()?)))
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
    fn try_parse(
        raw: nojson::RawJsonValue<'text, 'raw>,
    ) -> Result<Option<Self>, nojson::JsonParseError> {
        match raw.kind() {
            nojson::JsonValueKind::String if raw.as_raw_str().starts_with('$') => {
                let s = raw.to_unquoted_string_str()?;
                if let Some(bits) = s.strip_prefix("$i") {
                    let bits: usize = bits.parse().map_err(|e| raw.invalid(e))?;
                    if bits > 64 {
                        return Err(raw.invalid("signed integers must be <= 64 bits"));
                    }
                    Ok(Some(Self::Integer { bits, signed: true }))
                } else if let Some(bits) = s.strip_prefix("$u") {
                    let bits: usize = bits.parse().map_err(|e| raw.invalid(e))?;
                    if bits > 64 {
                        return Err(raw.invalid("unsigned integers must be <= 64 bits"));
                    }
                    Ok(Some(Self::Integer {
                        bits,
                        signed: false,
                    }))
                } else if let Some(s) = s.strip_prefix("$[")
                    && let Some(chars) = s.strip_suffix(']')
                {
                    let chars: usize = chars.parse().map_err(|e| raw.invalid(e))?;
                    Ok(Some(Self::String { chars }))
                } else {
                    Err(raw.invalid(
                        "unknown generator format; expected $i<bits>, $u<bits>, or $[chars]",
                    ))
                }
            }
            nojson::JsonValueKind::Object => {
                todo!();
            }
            _ => Ok(None),
        }
    }
}
