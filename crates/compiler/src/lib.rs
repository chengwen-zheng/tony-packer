pub struct Compiler {}

pub struct Config {
    pub input: String,
    pub output: String,
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler::new()
    }
}
impl Compiler {
    pub fn new() -> Compiler {
        Compiler {}
    }

    pub fn compile(&self) {
        todo!();
    }
}
