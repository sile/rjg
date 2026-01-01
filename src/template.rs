#[derive(Debug)]
pub enum ValueTemplate<'text, 'raw> {
    Literal(nojson::RawJsonValue<'text, 'raw>),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
    // Generator
}
