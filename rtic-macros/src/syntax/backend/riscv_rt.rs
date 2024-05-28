use syn::{
    parse::{Parse, ParseStream},
    Error, Result,
};

#[derive(Debug)]
pub struct BackendArgs {
    // Define your backend-specific input here
}

impl Parse for BackendArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Err(Error::new(
            input.span(),
            "riscv-rt backend does not accept any arguments",
        ))
    }
}
