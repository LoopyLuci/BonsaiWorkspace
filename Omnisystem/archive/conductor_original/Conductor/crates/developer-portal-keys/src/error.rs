//! Error
#[derive(Debug, Clone)]
pub enum Error { Other(String), }
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self { Error::Other(msg) => write!(f, "{}", msg), }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
