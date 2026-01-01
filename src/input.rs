#[derive(Debug)]
pub struct Input {
    pub var_prefix: String,
}

impl Input {
    pub const DEFAULT_VAR_PREFIX: &'static str = "$";
}
