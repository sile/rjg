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
    Float {
        bits: usize,
    },
    String {
        chars: usize,
    },
    Oneof {
        choices: Vec<ValueTemplate<'text, 'raw>>,
    },
}
