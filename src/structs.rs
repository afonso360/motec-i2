use crate::{I2Error, I2Result};
use std::io::SeekFrom;
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct FileAddr(u32);

impl FileAddr {
    pub(crate) fn zero() -> Self {
        FileAddr(0)
    }

    pub(crate) fn seek(self) -> SeekFrom {
        SeekFrom::Start(self.0 as u64)
    }

    /// Is this a zero addr
    pub(crate) fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Is this a zero addr
    pub(crate) fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Add<u64> for FileAddr {
    type Output = FileAddr;

    fn add(self, rhs: u64) -> Self::Output {
        FileAddr(self.0 + rhs as u32)
    }
}

impl Add<u32> for FileAddr {
    type Output = FileAddr;

    fn add(self, rhs: u32) -> Self::Output {
        FileAddr(self.0 + rhs)
    }
}

impl From<FileAddr> for u32 {
    fn from(addr: FileAddr) -> Self {
        addr.0
    }
}

impl From<u32> for FileAddr {
    fn from(addr: u32) -> Self {
        FileAddr(addr)
    }
}

impl From<u16> for FileAddr {
    fn from(addr: u16) -> Self {
        FileAddr(addr as u32)
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Header {
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

impl Header {
    /// Offset from the start of this structure where channel metadata address exists
    pub(crate) const CHANNEL_META_OFFSET: u64 = 8;
    /// Offset from the start of this structure where channel data address exists
    pub(crate) const CHANNEL_DATA_OFFSET: u64 = 12;
    /// Offset from the start of this structure where event address exists
    pub(crate) const EVENT_OFFSET: u64 = 36;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sample {
    I16(i16),
    I32(i32),
    F32(f32),
}

impl Sample {
    /// Calculates the final value of this sample as a f64
    pub fn decode_f64(&self, channel: &Channel) -> f64 {
        let value = match self {
            Sample::I16(v) => *v as f64,
            Sample::I32(v) => *v as f64,
            Sample::F32(v) => *v as f64,
        };

        // TODO: Test channel.offset with values of mul != 1
        let value = value / channel.scale as f64;
        let value = value * (10.0f64.powi(-channel.dec_places as i32));
        let value = value * channel.mul as f64;
        let value = value + channel.offset as f64;
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

/// Represents the metadata about a channel that is written on a file. This includes
/// the location in the file of the channel, as well as metadata present in [Channel]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FileChannel {
    pub prev_addr: FileAddr,
    pub next_addr: FileAddr,
    pub data_addr: FileAddr,
    pub samples: u32,
    pub channel: Channel,
}

impl FileChannel {
    /// Size of a channel header entry in bytes
    pub(crate) const ENTRY_SIZE: u32 = 124;

    /// Offset of the next addr field from the start of this entry
    pub(crate) const NEXT_ADDR_OFFSET: u32 = 4;

    /// Offset of the data addr field from the start of this entry
    pub(crate) const DATA_ADDR_OFFSET: u32 = 8;

    /// Calculates the size in bytes of the data section for this channel
    pub(crate) fn data_size(&self) -> u32 {
        self.samples * self.channel.datatype.size() as u32
    }
}

/// Metadata about a channel
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Channel {
    pub datatype: Datatype,
    /// Sample Rate in Hz
    pub sample_rate: u16,

    /// This number is added after the rest of the transformations have been applied
    pub offset: u16,
    pub mul: u16,
    pub scale: u16,
    pub dec_places: i16,

    pub name: String,
    pub short_name: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Event {
    /// Max 64 chars
    pub name: String,
    /// Max 64 chars
    pub session: String,
    /// Max 1024 chars
    pub comment: String,
}

impl Event {
    /// Offset from the start of this structure where venue address exists
    pub(crate) const VENUE_ADDR_OFFSET: u64 = 1152;
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Venue {
    /// Max 64 chars
    pub name: String,
}

impl Venue {
    /// Offset from the start of this structure where vehicle address exists
    pub(crate) const VEHICLE_ADDR_OFFSET: u64 = 1098;
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
