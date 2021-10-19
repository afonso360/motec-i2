use crate::{I2Error, I2Result};

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Header {
    pub channel_meta_ptr: u32,
    pub channel_data_ptr: u32,
    pub event_ptr: u32,

    pub device_serial: u32,
    pub device_type: String,
    pub device_version: u16,

    pub num_channels: u32,

    // TODO: Replace with timestamp
    pub date_string: String,
    pub time_string: String,

    // TODO: Probably should be Option<String>?
    pub driver: String,
    pub vehicleid: String,
    pub venue: String,
    pub session: String,
    pub short_comment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sample {
    I16(i16),
    I32(i32),
    F32(f32),
}

impl Sample {
    /// Calculates the final value of this sample as a f64
    pub fn decode_f64(&self, channel: &ChannelMetadata) -> f64 {
        let value = match self {
            Sample::I16(v) => *v as f64,
            Sample::I32(v) => *v as f64,
            Sample::F32(v) => *v as f64,
        };

        // TODO: channel.shift figures into this somewhere... but we don't know what to do with it
        // Is it a binary shift? or a decimal shift? I'm leaning towards decimal shift, but not sure
        assert_eq!(channel.shift, 0);
        let value = value / channel.scale as f64;
        let value = value * (10.0f64.powi(-channel.dec_places as i32));
        let value = value * channel.mul as f64;
        value
    }
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

    Invalid,
}

impl Datatype {
    /// Size in bytes that this datatype occupies on file
    pub fn size(&self) -> u16 {
        match self {
            Datatype::Beacon16 | Datatype::I16 | Datatype::F16 => 2,
            Datatype::Beacon32 | Datatype::I32 | Datatype::F32 => 4,

            // We really don't know what these values are
            Datatype::Invalid => 0,
        }
    }

    pub fn _type(&self) -> u16 {
        match self {
            Datatype::Beacon16 | Datatype::Beacon32 => 0,
            Datatype::I16 | Datatype::I32 => 3,
            Datatype::F16 | Datatype::F32 => 7,
            Datatype::Invalid => 999,
        }
    }

    pub fn from_type_and_size(_type: u16, size: u16) -> I2Result<Self> {
        match (_type, size) {
            (0, 2) => Ok(Datatype::Beacon16),
            (0, 4) => Ok(Datatype::Beacon32),
            (3, 2) => Ok(Datatype::I16),
            (3, 4) => Ok(Datatype::I32),
            // 20160903-0051401.ld uses 5 for ints?
            (5, 2) => Ok(Datatype::I16),
            (5, 4) => Ok(Datatype::I32),
            (7, 2) => Ok(Datatype::F16),
            (7, 4) => Ok(Datatype::F32),

            // The mu iracing exporter exports these values on Damper Pos FL/FR/RL, they have 0 samples
            (17536, 5) | (6566, 5) | (29813, 5) => Ok(Datatype::Invalid),
            // This should be Beacon40 ?, but the iRacing mu exporter puts this in Damper Pos RR
            (0, 5) => Ok(Datatype::Invalid),
            // Iracing mu exporter Ride Height Center 0 samples
            (15, 5) => Ok(Datatype::Invalid),
            _ => Err(I2Error::UnrecognizedDatatype { _type, size }),
        }
    }
}

/// ChannelMetadata is a doubly linked list of blocks in the file
/// This only contains info about a channel, actual data is stored somewhere else on the file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ChannelMetadata {
    pub prev_addr: u32,
    pub next_addr: u32,

    pub data_addr: u32,
    pub data_count: u32,

    pub datatype: Datatype,
    /// Sample Rate in Hz
    pub sample_rate: u16,

    pub shift: u16,
    pub mul: u16,
    pub scale: u16,
    pub dec_places: i16,

    pub name: String,
    pub short_name: String,
    pub unit: String,
}

impl ChannelMetadata {
    /// Size of a metadata entry in bytes
    pub(crate) const ENTRY_SIZE: u32 = 124;

    /// Calculates the size in bytes of the data section for this channel
    pub(crate) fn data_size(&self) -> u32 {
        self.data_count * self.datatype.size() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Event {
    /// Max 64 chars
    pub name: String,
    /// Max 64 chars
    pub session: String,
    /// Max 1024 chars
    pub comment: String,

    pub venue_addr: u16,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Venue {
    /// Max 64 chars
    pub name: String,

    pub vehicle_addr: u16,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Vehicle {
    /// Max 64 chars
    pub id: String,
    pub weight: u32,
    /// Max 32 chars
    pub _type: String,
    /// Max 32 chars
    pub comment: String,
}
