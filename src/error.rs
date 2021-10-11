use std::error::Error;
use std::fmt;
use std::io;

pub type I2Result<T> = Result<T, I2Error>;

#[derive(Debug)]
pub enum I2Error {
    IOError(io::Error),

    // Parsing Errors
    InvalidHeaderMarker { found: u32, expected: u32 },
    UnrecognizedDatatype { _type: u16, size: u16 },
}

impl fmt::Display for I2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I2Error::IOError(e) => write!(f, "Underlying IO Error: {}", e),
            I2Error::InvalidHeaderMarker { found, expected } => write!(
                f,
                "Invalid Header Marker found {}, expected {}",
                found, expected
            ),
            I2Error::UnrecognizedDatatype { _type, size } => write!(
                f,
                "Unrecognized Datatype found (_type: {}, size: {})",
                _type, size
            ),
        }
    }
}

impl Error for I2Error {}

impl From<io::Error> for I2Error {
    fn from(e: io::Error) -> Self {
        I2Error::IOError(e)
    }
}
