#[derive(Debug)]
pub enum ValueTemplate<'text, 'raw> {
    Literal(nojson::RawJsonValue<'text, 'raw>),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
    Generator(ValueGenerator<'text, 'raw>),
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
