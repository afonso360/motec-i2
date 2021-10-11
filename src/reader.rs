use crate::{ChannelMetadata, Datatype, Header, I2Error, I2Result, ProLogging};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use std::{io, iter};

const LD_HEADER_MARKER: u32 = 64;

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
        assert_eq!(_unknown, [0u8; 20]);

        let event_ptr = self.source.read_u32::<LittleEndian>()?;

        let mut _unknown = self.read_bytes(24)?;
        assert_eq!(_unknown, [0u8; 24]);

        // TODO: These may not actually be const...
        let unknown_const_1 = self.source.read_u16::<LittleEndian>()?;
        assert_eq!(unknown_const_1, 0x0000);
        let unknown_const_2 = self.source.read_u16::<LittleEndian>()?;
        assert_eq!(unknown_const_2, 0x4240);
        let unknown_const_3 = self.source.read_u16::<LittleEndian>()?;
        assert_eq!(unknown_const_3, 0x000F);

        let device_serial = self.source.read_u32::<LittleEndian>()?;
        let device_type = self.read_string(8)?;
        let device_version = self.source.read_u16::<LittleEndian>()?;

        // TODO: This may not actually be const...
        let unknown_const_4 = self.source.read_u16::<LittleEndian>()?;
        assert_eq!(unknown_const_4, 0x0080);

        let num_channels = self.source.read_u32::<LittleEndian>()?;
        let _unknown = self.source.read_u32::<LittleEndian>()?;

        let date_string = self.read_string(16)?;
        let _unknown = self.read_bytes(16)?;
        let time_string = self.read_string(16)?;
        let _unknown = self.read_bytes(16)?;

        let driver = self.read_string_space(64)?;
        let vehicleid = self.read_string_space(64)?;
        let _unknown = self.read_bytes(64)?;
        let venue = self.read_string_space(64)?;
        let _unknown = self.read_bytes(64)?;

        let _unknown = self.read_bytes(1024)?;

        let pro_logging_magic_bytes = self.source.read_u32::<LittleEndian>()?;
        // TODO: Check if this is how pro logging is enabled / disabled
        let pro_logging = if pro_logging_magic_bytes != 0 {
            ProLogging::Enabled
        } else {
            ProLogging::Disabled
        };

        let _unknown = self.read_bytes(2)?;
        let session = self.read_string(64)?;
        let short_comment = self.read_string_space(64)?;
        let _unknown = self.read_bytes(126)?; // Probably long_comment? + some 2byte

        let event = self.read_string(64)?;
        let _session2 = self.read_string(64)?; // ??????

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
            event,
            pro_logging,
        };
        self.header = Some(header.clone());
        Ok(header)
    }

    /// Read the channel meta data blocks inside the ld file
    ///
    /// The channel metadata structs form a linked list with each metadata block pointing
    /// to the next block
    ///
    /// Calls [LDReader::read_header] if it hasn't been called before
    pub fn read_channels(&mut self) -> I2Result<Vec<ChannelMetadata>> {
        if self.header.is_none() {
            self.header = Some(self.read_header()?);
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
    pub fn read_channel_metadata(&mut self, addr: u32) -> I2Result<ChannelMetadata> {
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

        let shift = self.source.read_u16::<LittleEndian>()?;
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
            shift,
            mul,
            scale,
            dec_places,
            name,
            short_name,
            unit,
        })
    }

    /// Returns a iterator over the channel data
    // pub fn channel_iter(&mut self, channel: &ChannelMetadata) -> I2Result<ChannelIter> {}

    fn read_bytes(&mut self, size: usize) -> io::Result<Vec<u8>> {
        let mut bytes: Vec<u8> = iter::repeat(0u8).take(size).collect();
        self.source.read_exact(&mut bytes[0..size])?;
        Ok(bytes)
    }

    /// Reads a string with a fixed size trimming null bytes
    fn read_string(&mut self, size: usize) -> io::Result<String> {
        let bytes = self.read_bytes(size)?;
        let string = String::from_utf8(bytes).unwrap().replace('\0', ""); // TODO: this should be ?
        Ok(string)
    }

    /// Reads a string with a fixed size trimming null bytes, and trailing space character
    fn read_string_space(&mut self, size: usize) -> io::Result<String> {
        Ok(self.read_string(size)?.trim_end_matches(' ').to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::LDReader;
    use crate::{ChannelMetadata, Datatype, Header, ProLogging};
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn read_sample1_header() {
        let bytes = fs::read("./samples/Sample1.ld").unwrap();
        let mut cursor = Cursor::new(bytes);
        let mut reader = LDReader::new(&mut cursor);

        assert_eq!(
            reader.read_header().unwrap(),
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
                event: "i2 data day".to_string(),
                pro_logging: ProLogging::Enabled, // 0xD20822,
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
                shift: 0,
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
                shift: 0,
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
                shift: 0,
                mul: 1,
                scale: 1,
                dec_places: 1,
                name: "Steered Angle".to_owned(),
                short_name: "Steered".to_owned(),
                unit: "deg".to_owned(),
            }
        );
    }
}
