pub mod compilation;
pub mod functions_utils;
pub mod info_gathering;
pub mod sig;
pub mod utils;

#[derive(Debug)]
pub enum ArianeError {
    InvalidInput,
    IOError(std::io::Error),
}
