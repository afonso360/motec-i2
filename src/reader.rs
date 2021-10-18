use crate::{
    ChannelMetadata, Datatype, Event, FileAddr, Header, I2Error, I2Result, Sample, Vehicle, Venue,
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use std::{io, iter};

pub(crate) const LD_HEADER_MARKER: u32 = 64;

/// Holds all the addresses of important structures in the file
#[derive(Debug, Clone, PartialEq)]
struct AddressTable {
    channel_meta: FileAddr,
    channel_data: FileAddr,
    event: Option<FileAddr>,
    venue: Option<FileAddr>,
    vehicle: Option<FileAddr>,
}

#[derive(Debug)]
pub struct LDReader<'a, S: Read + Seek> {
    source: &'a mut S,
    address_table: Option<AddressTable>,
}

impl<'a, S: Read + Seek> LDReader<'a, S> {
    pub fn new(source: &'a mut S) -> Self {
        Self {
            source,
            address_table: None,
        }
    }

    /// Retrieves a copy of the address table if it exists, otherwise finds all the addresses.
    fn address_table(&mut self) -> I2Result<AddressTable> {
        if let Some(addr_tbl) = &self.address_table {
            return Ok(addr_tbl.clone());
        }

        self.source
            .seek(SeekFrom::Start(Header::CHANNEL_META_OFFSET))?;
        let channel_meta = self.source.read_u32::<LittleEndian>()?.into();

        self.source
            .seek(SeekFrom::Start(Header::CHANNEL_DATA_OFFSET))?;
        let channel_data = self.source.read_u32::<LittleEndian>()?.into();

        self.source.seek(SeekFrom::Start(Header::EVENT_OFFSET))?;
        let event = match self.source.read_u32::<LittleEndian>()? {
            0 => None,
            addr => Some(addr.into()),
        };

        let venue = match event {
            Some(event_addr) => {
                let venue_addr: FileAddr = event_addr + Event::VENUE_ADDR_OFFSET;
                self.source.seek(venue_addr.seek())?;
                match self.source.read_u16::<LittleEndian>()? {
                    0 => None,
                    addr => Some(addr.into()),
                }
            }
            None => None,
        };

        let vehicle = match venue {
            Some(venue_addr) => {
                let vehicle_addr: FileAddr = venue_addr + Venue::VEHICLE_ADDR_OFFSET;
                self.source.seek(vehicle_addr.seek())?;
                match self.source.read_u16::<LittleEndian>()? {
                    0 => None,
                    addr => Some(addr.into()),
                }
            }
            None => None,
        };

        let addr_tbl = AddressTable {
            channel_meta,
            channel_data,
            event,
            venue,
            vehicle,
        };
        self.address_table = Some(addr_tbl.clone());
        Ok(addr_tbl)
    }

    pub fn read_header(&mut self) -> I2Result<Header> {
        // Header is always at start
        self.source.seek(SeekFrom::Start(0))?;

        let ldmarker = self.source.read_u32::<LittleEndian>()?;
        if ldmarker != LD_HEADER_MARKER {
            return Err(I2Error::InvalidHeaderMarker {
                found: ldmarker,
                expected: LD_HEADER_MARKER,
            });
        }

        let _unknown = self.source.read_u32::<LittleEndian>()?;

        // TODO: We can probably skip reading these bytes
        let _channel_meta_ptr = self.source.read_u32::<LittleEndian>()?;
        let _channel_data_ptr = self.source.read_u32::<LittleEndian>()?;

        let mut _unknown = self.read_bytes(20)?;
        // assert_eq!(_unknown, [0u8; 20]);

        // Sample1.ld has this at addr 0x6E2, that is probably the length of the header????
        // TODO: We can probably skip reading these bytes
        let _event_ptr = self.source.read_u32::<LittleEndian>()?;

        let mut _unknown = self.read_bytes(24)?;
        // Not 0 in 20160903-0051401.ld
        // assert_eq!(_unknown, [0u8; 24]);

        // TODO: These may not actually be const...
        let _unknown_const_1 = self.source.read_u16::<LittleEndian>()?;
        // assert_eq!(_unknown_const_1, 0x0000);
        let _unknown_const_2 = self.source.read_u16::<LittleEndian>()?;
        // assert_eq!(_unknown_const_2, 0x4240);
        let _unknown_const_3 = self.source.read_u16::<LittleEndian>()?;
        // assert_eq!(_unknown_const_3, 0x000F);

        let device_serial = self.source.read_u32::<LittleEndian>()?;
        let device_type = self.read_string(8)?;
        let device_version = self.source.read_u16::<LittleEndian>()?;

        // TODO: This may not actually be const...
        let _unknown_const_4 = self.source.read_u16::<LittleEndian>()?;
        // assert_eq!(_unknown_const_4, 0x0080);

        let num_channels = self.source.read_u32::<LittleEndian>()?;
        let _unknown = self.source.read_u32::<LittleEndian>()?;

        let date_string = self.read_string(16)?;
        let _unknown = self.read_bytes(16)?;
        let time_string = self.read_string(16)?;
        let _unknown = self.read_bytes(16)?;

        let driver = self.read_string(64)?;
        let vehicleid = self.read_string(64)?;
        let _unknown = self.read_bytes(64)?;
        let venue = self.read_string(64)?;
        let _unknown = self.read_bytes(64)?;

        let _unknown = self.read_bytes(1024)?;

        let pro_logging_bytes = self.source.read_u32::<LittleEndian>()?;

        let _unknown = self.read_bytes(2)?;
        let session = self.read_string(64)?;
        let short_comment = self.read_string(64)?;
        let _unknown = self.read_bytes(126)?;

        Ok(Header {
            device_serial,
            device_type,
            device_version,
            pro_logging_bytes,
            num_channels,
            date_string,
            time_string,
            driver,
            vehicleid,
            venue,
            session,
            short_comment,
        })
    }

    pub fn read_event(&mut self) -> I2Result<Option<Event>> {
        Ok(match self.address_table()?.event {
            Some(addr) => {
                self.source.seek(addr.seek())?;

                let name = self.read_string(64)?;
                let session = self.read_string(64)?;
                let comment = self.read_string(1024)?;
                // let venue_addr = self.source.read_u16::<LittleEndian>()?;

                Some(Event {
                    name,
                    session,
                    comment,
                })
            }
            None => None,
        })
    }

    pub fn read_venue(&mut self) -> I2Result<Option<Venue>> {
        Ok(match self.address_table()?.venue {
            Some(addr) => {
                self.source.seek(addr.seek())?;

                let name = self.read_string(64)?;
                let _unknown = self.read_bytes(1034)?;
                // let vehicle_addr = self.source.read_u16::<LittleEndian>()?;

                Some(Venue { name })
            }
            None => None,
        })
    }

    pub fn read_vehicle(&mut self) -> I2Result<Option<Vehicle>> {
        Ok(match self.address_table()?.vehicle {
            Some(addr) => {
                self.source.seek(addr.seek())?;

                let id = self.read_string(64)?;
                let _unknown = self.read_bytes(128)?;
                let weight = self.source.read_u32::<LittleEndian>()?;
                let _type = self.read_string(32)?;
                let comment = self.read_string(32)?;

                Some(Vehicle {
                    id,
                    weight,
                    _type,
                    comment,
                })
            }
            None => None,
        })
    }

    /// Read the channel meta data blocks inside the ld file
    ///
    /// The channel metadata structs form a linked list with each metadata block pointing
    /// to the next block
    ///
    /// Calls [LDReader::read_header] if it hasn't been called before
    pub fn read_channels(&mut self) -> I2Result<Vec<ChannelMetadata>> {
        let channel_meta = self.address_table()?.channel_meta;

        let mut channels = vec![];

        let mut next_ptr = channel_meta;
        loop {
            // A 0 addr means we are done searching this list
            if next_ptr.is_zero() {
                return Ok(channels);
            }

            let channel = self.read_channel_metadata(next_ptr)?;
            next_ptr = channel.next_addr;
            channels.push(channel);
        }
    }

    /// Read the [ChannelMetadata] block at file offset `addr`
    fn read_channel_metadata(&mut self, addr: FileAddr) -> I2Result<ChannelMetadata> {
        self.source.seek(addr.seek())?;

        let prev_addr = FileAddr::from(self.source.read_u32::<LittleEndian>()?);
        let next_addr = FileAddr::from(self.source.read_u32::<LittleEndian>()?);
        let data_addr = FileAddr::from(self.source.read_u32::<LittleEndian>()?);
        let data_count = self.source.read_u32::<LittleEndian>()?;

        let _unknown = self.source.read_u16::<LittleEndian>()?;

        let datatype_type = self.source.read_u16::<LittleEndian>()?;
        let datatype_size = self.source.read_u16::<LittleEndian>()?;
        let datatype = Datatype::from_type_and_size(datatype_type, datatype_size)?;

        let sample_rate = self.source.read_u16::<LittleEndian>()?;

        let offset = self.source.read_u16::<LittleEndian>()?;
        let mul = self.source.read_u16::<LittleEndian>()?;
        let scale = self.source.read_u16::<LittleEndian>()?;
        let dec_places = self.source.read_i16::<LittleEndian>()?;

        let name = self.read_string(32)?;
        let short_name = self.read_string(8)?;
        let unit = self.read_string(12)?;
        let _unknown = self.read_bytes(40)?; // ? (40 bytes for ACC, 32 bytes for acti)

        Ok(ChannelMetadata {
            prev_addr,
            next_addr,
            data_addr,
            data_count,
            datatype,
            sample_rate,
            offset,
            mul,
            scale,
            dec_places,
            name,
            short_name,
            unit,
        })
    }

    // TODO: We should probably have a iterator over channel data

    /// Returns a iterator over the channel data
    pub fn channel_data(&mut self, channel: &ChannelMetadata) -> I2Result<Vec<Sample>> {
        self.source.seek(channel.data_addr.seek())?;

        // Data for a channel is stored in a contiguous manner at the addr ptr
        let data = (0..channel.data_count)
            .map(|_| {
                Ok({
                    match channel.datatype {
                        Datatype::Beacon16 | Datatype::I16 => {
                            Sample::I16(self.source.read_i16::<LittleEndian>()?)
                        }
                        Datatype::Beacon32 | Datatype::I32 => {
                            Sample::I32(self.source.read_i32::<LittleEndian>()?)
                        }

                        Datatype::F16 => unimplemented!("Reading f16 samples unimplemented"),
                        Datatype::F32 => Sample::F32(self.source.read_f32::<LittleEndian>()?),
                        Datatype::Invalid => panic!(
                            "Tried to read invalid datatype from channel: {}",
                            channel.name
                        ),
                    }
                })
            })
            .collect::<I2Result<Vec<_>>>()?;

        Ok(data)
    }

    fn read_bytes(&mut self, size: usize) -> io::Result<Vec<u8>> {
        let mut bytes: Vec<u8> = iter::repeat(0u8).take(size).collect();
        self.source.read_exact(&mut bytes[0..size])?;
        Ok(bytes)
    }

    /// Reads a string with a fixed size trimming null bytes
    fn read_string(&mut self, size: usize) -> I2Result<String> {
        let bytes = self.read_bytes(size)?;
        let str_size = bytes.iter().position(|c| *c == b'\0').unwrap_or(size);
        let str = ::std::str::from_utf8(&bytes[0..str_size])?;
        Ok(str.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::{AddressTable, FileAddr, LDReader};
    use crate::{ChannelMetadata, Datatype, Event, Header, Sample, Vehicle, Venue};
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn read_sample1_address_table() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let addr_tbl = reader.address_table().unwrap();
        assert_eq!(
            addr_tbl,
            AddressTable {
                channel_meta: FileAddr::from(0x3448u32),
                channel_data: FileAddr::from(0x5A10u32),
                event: Some(FileAddr::from(0x06E2u32)),
                venue: Some(FileAddr::from(0x1336u32)),
                vehicle: Some(FileAddr::from(0x1F54u32)),
            }
        );
    }

    #[test]
    fn read_sample1_header() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let header = reader.read_header().unwrap();
        assert_eq!(
            header,
            Header {
                device_serial: 0x2EE7,
                device_type: "ADL".to_string(),
                device_version: 0x01A4,
                num_channels: 0x4E,
                date_string: "23/11/2005".to_string(),
                time_string: "09:53:00".to_string(),
                driver: "".to_string(),
                vehicleid: "11A".to_string(),
                venue: "Calder".to_string(),
                session: "2".to_string(),
                short_comment: "second warmup".to_string(),
                pro_logging_bytes: 0xD20822,
            }
        );
    }

    #[test]
    fn read_sample1_channel_metadata() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let channels = reader.read_channels().unwrap();
        assert_eq!(channels.len(), 78);
        assert_eq!(
            channels[0],
            ChannelMetadata {
                prev_addr: FileAddr::from(0u32),
                next_addr: FileAddr::from(13508u32),
                data_addr: FileAddr::from(23056u32),
                data_count: 908,
                datatype: Datatype::I16,
                sample_rate: 2,
                offset: 0,
                mul: 1,
                scale: 1,
                dec_places: 1,
                name: "Air Temp Inlet".to_owned(),
                short_name: "Air Tem".to_owned(),
                unit: "C".to_owned(),
            }
        );

        assert_eq!(
            channels[1],
            ChannelMetadata {
                prev_addr: FileAddr::from(13384u32),
                next_addr: FileAddr::from(13632u32),
                data_addr: FileAddr::from(24872u32),
                data_count: 4540,
                datatype: Datatype::I16,
                sample_rate: 10,
                offset: 0,
                mul: 1,
                scale: 1,
                dec_places: 0,
                name: "Brake Temp FL".to_owned(),
                short_name: "Brake T".to_owned(),
                unit: "C".to_owned(),
            }
        );

        assert_eq!(
            channels[77],
            ChannelMetadata {
                prev_addr: FileAddr::from(22808u32),
                next_addr: FileAddr::from(0u32),
                data_addr: FileAddr::from(1189836u32),
                data_count: 9080,
                datatype: Datatype::I16,
                sample_rate: 20,
                offset: 0,
                mul: 1,
                scale: 1,
                dec_places: 1,
                name: "Steered Angle".to_owned(),
                short_name: "Steered".to_owned(),
                unit: "deg".to_owned(),
            }
        );
    }

    macro_rules! assert_delta {
        ($x:expr, $y:expr, $d:expr) => {
            if ($x - $y).abs() > $d {
                panic!();
            }
        };
    }

    #[test]
    fn read_sample1_channel_data() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let channels = reader.read_channels().unwrap();
        let channel = &channels[0];

        let data = reader.channel_data(channel).unwrap();
        let data: Vec<_> = data.into_iter().take(5).collect();

        assert_eq!(
            data,
            vec![
                Sample::I16(199),
                Sample::I16(199),
                Sample::I16(201),
                Sample::I16(199),
                Sample::I16(199),
            ]
        );

        assert_delta!(data[0].decode_f64(channel), 19.9, 0.000001);
        assert_delta!(data[1].decode_f64(channel), 19.9, 0.000001);
        assert_delta!(data[2].decode_f64(channel), 20.1, 0.000001);
        assert_delta!(data[3].decode_f64(channel), 19.9, 0.000001);
        assert_delta!(data[4].decode_f64(channel), 19.9, 0.000001);
    }

    #[test]
    fn read_sample1_event() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let event = reader.read_event().unwrap();

        assert_eq!(
            event,
            Some(Event {
                name: "i2 data day".to_string(),
                session: "2".to_string(),
                comment: "Calder Park, 23/11/05, fine sunny day".to_string(),
            })
        );
    }

    #[test]
    fn read_sample1_venue() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let venue = reader.read_venue().unwrap();

        assert_eq!(
            venue,
            Some(Venue {
                name: "Calder".to_string(),
            })
        );
    }

    #[test]
    fn read_sample1_vehicle() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let vehicle = reader.read_vehicle().unwrap();

        assert_eq!(
            vehicle,
            Some(Vehicle {
                id: "11A".to_string(),
                weight: 0,
                _type: "Car".to_string(),
                comment: "".to_string(),
            })
        );
    }
}
