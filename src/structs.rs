use crate::{I2Error, I2Result};

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ProLogging {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Header {
    pub(crate) channel_meta_ptr: u32,
    pub(crate) channel_data_ptr: u32,
    pub(crate) event_ptr: u32,

    pub(crate) device_serial: u32,
    pub(crate) device_type: String,
    pub(crate) device_version: u16,

    pub(crate) num_channels: u32,

    // TODO: Replace with timestamp
    pub(crate) date_string: String,
    pub(crate) time_string: String,

    // TODO: Probably should be Option<String>?
    pub(crate) driver: String,
    pub(crate) vehicleid: String,
    pub(crate) venue: String,
    pub(crate) session: String,
    pub(crate) short_comment: String,
    pub(crate) event: String,
    // pub(crate) session: String,
    pub(crate) pro_logging: ProLogging,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Datatype {
    // TODO: Not Too sure about this data type, it shows up as beacon in the sample dataset
    // It behaves as an integer of the same size
    Beacon16,
    Beacon32,

    I16,
    I32,

    F16,
    F32,
}

impl Datatype {
    pub fn from_type_and_size(_type: u16, size: u16) -> I2Result<Self> {
        match (_type, size) {
            (0, 2) => Ok(Datatype::Beacon16),
            (0, 4) => Ok(Datatype::Beacon32),
            (3, 2) => Ok(Datatype::I16),
            (3, 4) => Ok(Datatype::I32),
            (7, 2) => Ok(Datatype::F16),
            (7, 4) => Ok(Datatype::F32),
            _ => Err(I2Error::UnrecognizedDatatype { _type, size }),
        }
    }
}

/// ChannelMetadata is a doubly linked list of blocks in the file
/// This only contains info about a channel, actual data is stored somewhere else on the file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ChannelMetadata {
    pub(crate) prev_addr: u32,
    pub(crate) next_addr: u32,

    pub(crate) data_addr: u32,
    pub(crate) data_count: u32,

    pub(crate) datatype: Datatype,
    /// Sample Rate in Hz
    pub(crate) sample_rate: u16,

    pub(crate) shift: u16,
    pub(crate) mul: u16,
    pub(crate) scale: u16,
    pub(crate) dec_places: i16,

    pub(crate) name: String,
    pub(crate) short_name: String,
    pub(crate) unit: String,
}
