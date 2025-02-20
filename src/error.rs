use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[allow(dead_code)]
#[derive(Debug, From)]
pub enum Error {
    #[from]
    MongoDBError(mongodb::error::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
