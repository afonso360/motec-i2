use crate::{ChannelMetadata, Datatype, Event, Header, I2Error, I2Result, Sample, Vehicle, Venue};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use std::{io, iter};

pub(crate) const LD_HEADER_MARKER: u32 = 64;

#[derive(Debug)]
pub struct LDReader<'a, S: Read + Seek> {
    source: &'a mut S,
    header: Option<Header>,
}

impl<'a, S: Read + Seek> LDReader<'a, S> {
    pub fn new(source: &'a mut S) -> Self {
        Self {
            source,
            header: None,
        }
    }

    // TODO: Remove asserts and change into a proper error type
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

        let channel_meta_ptr = self.source.read_u32::<LittleEndian>()?;
        let channel_data_ptr = self.source.read_u32::<LittleEndian>()?;

        let mut _unknown = self.read_bytes(20)?;
        // assert_eq!(_unknown, [0u8; 20]);

        // Sample1.ld has this at addr 0x6E2, that is probably the length of the header????
        let event_ptr = self.source.read_u32::<LittleEndian>()?;

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

        let _pro_logging_bytes = self.source.read_u32::<LittleEndian>()?;

        let _unknown = self.read_bytes(2)?;
        let session = self.read_string(64)?;
        let short_comment = self.read_string(64)?;
        let _unknown = self.read_bytes(126)?; // Probably long_comment? + some 2byte

        //let long_comment = self.read_string(??);

        let header = Header {
            channel_meta_ptr,
            channel_data_ptr,
            event_ptr,
            device_serial,
            device_type,
            device_version,
            num_channels,
            date_string,
            time_string,
            driver,
            vehicleid,
            venue,
            session,
            short_comment,
        };
        self.header = Some(header.clone());
        Ok(header)
    }

    pub fn read_event(&mut self) -> I2Result<Option<Event>> {
        if self.header.is_none() {
            self.read_header()?;
        }

        let event_ptr = self.header.as_ref().unwrap().event_ptr;
        if event_ptr == 0 {
            return Ok(None);
        }

        self.source.seek(SeekFrom::Start(event_ptr as u64))?;

        let name = self.read_string(64)?;
        let session = self.read_string(64)?;
        let comment = self.read_string(1024)?;
        let venue_addr = self.source.read_u16::<LittleEndian>()?;

        Ok(Some(Event {
            name,
            session,
            comment,
            venue_addr,
        }))
    }

    pub fn read_venue(&mut self) -> I2Result<Option<Venue>> {
        Ok(match self.read_event()? {
            Some(event) => {
                if event.venue_addr == 0 {
                    return Ok(None);
                }

                self.source.seek(SeekFrom::Start(event.venue_addr as u64))?;

                let name = self.read_string(64)?;
                let _unknown = self.read_bytes(1034)?;
                let vehicle_addr = self.source.read_u16::<LittleEndian>()?;

                Some(Venue { name, vehicle_addr })
            }
            None => None,
        })
    }

    pub fn read_vehicle(&mut self) -> I2Result<Option<Vehicle>> {
        Ok(match self.read_venue()? {
            Some(venue) => {
                if venue.vehicle_addr == 0 {
                    return Ok(None);
                }

                self.source
                    .seek(SeekFrom::Start(venue.vehicle_addr as u64))?;

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
        if self.header.is_none() {
            self.read_header()?;
        }

        let mut channels = vec![];

        let mut next_ptr = self.header.as_ref().unwrap().channel_meta_ptr;
        loop {
            // A 0 addr means we are done searching this list
            if next_ptr == 0 {
                return Ok(channels);
            }

            let channel = self.read_channel_metadata(next_ptr)?;
            next_ptr = channel.next_addr;
            channels.push(channel);
        }
    }

    /// Read the [ChannelMetadata] block at file offset `addr`
    fn read_channel_metadata(&mut self, addr: u32) -> I2Result<ChannelMetadata> {
        self.source.seek(SeekFrom::Start(addr as u64))?;

        let prev_addr = self.source.read_u32::<LittleEndian>()?;
        let next_addr = self.source.read_u32::<LittleEndian>()?;
        let data_addr = self.source.read_u32::<LittleEndian>()?;
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
        self.source
            .seek(SeekFrom::Start(channel.data_addr as u64))?;

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
    use crate::reader::LDReader;
    use crate::{ChannelMetadata, Datatype, Event, Header, Sample, Vehicle, Venue};
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn read_sample1_header() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        let header = reader.read_header().unwrap();
        assert_eq!(
            header,
            Header {
                channel_meta_ptr: 0x3448,
                channel_data_ptr: 0x5A10,
                event_ptr: 0x06E2,
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
                prev_addr: 0,
                next_addr: 13508,
                data_addr: 23056,
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
                prev_addr: 13384,
                next_addr: 13632,
                data_addr: 24872,
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
                prev_addr: 22808,
                next_addr: 0,
                data_addr: 1189836,
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
                venue_addr: 0x1336,
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
                vehicle_addr: 0x1F54,
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
