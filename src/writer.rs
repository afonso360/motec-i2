use crate::full_header::FULL_HEADER;
use crate::{ChannelMetadata, FileAddr, Header, I2Result, Sample, LD_HEADER_MARKER};
use byteorder::{LittleEndian, WriteBytesExt};
use core::iter;
use std::io::{Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct LDWriter<'a, S: Write + Seek> {
    sink: &'a mut S,
    // header: Option<Header>,
}

impl<'a, S: Write + Seek> LDWriter<'a, S> {
    pub fn new(sink: &'a mut S) -> Self {
        Self {
            sink,
            // header: None,
        }
    }

    pub fn write_header(&mut self, hdr: &Header) -> I2Result<()> {
        // See comments on FULL_HEADER for an explanation on why we do this.
        self.sink.seek(SeekFrom::Start(0))?;
        self.sink.write(&FULL_HEADER[..])?;

        // Header is always at start
        self.sink.seek(SeekFrom::Start(0))?;

        self.sink.write_u32::<LittleEndian>(LD_HEADER_MARKER)?;

        // TODO: We don't know what this is, but Sample1.ld has it as 0
        self.sink.write_u32::<LittleEndian>(0x00000000)?;

        // TODO: Write this
        // self.sink.write_u32::<LittleEndian>(hdr.channel_meta_ptr)?;
        // self.sink.write_u32::<LittleEndian>(hdr.channel_data_ptr)?;

        // TODO: We don't know what this is, but Sample1.ld has it as 0
        self.sink.write(&[0u8; 20][..])?;

        // TODO: Write this out
        // self.sink.write_u32::<LittleEndian>(hdr.event_ptr)?;

        // TODO: We don't know what this is, but Sample1.ld has it as 0
        // 20160903-0051401.ld has this as a different value
        self.sink.write(&[0u8; 24][..])?;

        // TODO: We don't know what these are...
        self.sink.write_u16::<LittleEndian>(0x0000)?;
        self.sink.write_u16::<LittleEndian>(0x4240)?;
        self.sink.write_u16::<LittleEndian>(0x000F)?;

        self.sink.write_u32::<LittleEndian>(hdr.device_serial)?;
        self.write_string(8, &hdr.device_type)?;
        self.sink.write_u16::<LittleEndian>(hdr.device_version)?;

        // TODO: We don't know what this is, but Sample1.ld has it as this const
        self.sink.write_u16::<LittleEndian>(0x0080)?;

        self.sink.write_u32::<LittleEndian>(hdr.num_channels)?;
        // TODO: We don't know what this is, but Sample1.ld has it as this const
        self.sink.write_u32::<LittleEndian>(0x0001_0064)?;

        self.write_string(16, &hdr.date_string)?;
        self.write_string(16, "")?; // TODO: Not sure what these are
        self.write_string(16, &hdr.time_string)?;
        self.write_string(16, "")?; // TODO: Not sure what these are

        self.write_string(64, &hdr.driver)?;
        self.write_string(64, &hdr.vehicleid)?;
        self.write_string(64, "")?; // These strings don't seem to show up in i2
        self.write_string(64, &hdr.venue)?;
        self.write_string(64, "")?; // These strings don't seem to show up in i2

        // Not a string, i2 fails to read data if this is a string
        self.sink.write(&[0u8; 1024])?;

        // 0xD20822 for Sample1.ld
        self.sink.write_u32::<LittleEndian>(hdr.pro_logging_bytes)?;
        self.sink.write_u16::<LittleEndian>(0u16)?;

        self.write_string(64, &hdr.session)?;
        self.write_string(64, &hdr.short_comment)?;

        self.sink.write(&[0u8; 8])?;
        self.sink.write_u8(99)?;
        self.sink.write(&[0u8; 117])?;

        // TODO: Write Event

        Ok(())
    }

    pub fn write_channels(
        &mut self,
        channels: Vec<(ChannelMetadata, Vec<Sample>)>,
    ) -> I2Result<()> {
        // TODO: Should not be hardcoded
        self.sink.seek(SeekFrom::Start(0x3448))?;

        for i in 0..channels.len() {
            let (meta, samples) = &channels[i];
            let mut meta = meta.clone();
            let i = i as u32;

            meta.prev_addr = FileAddr::from(i * ChannelMetadata::ENTRY_SIZE);
            meta.next_addr =
                FileAddr::from(((i + 1) % (channels.len() as u32)) * ChannelMetadata::ENTRY_SIZE);
            meta.data_count = samples.len() as u32;

            let header_offset = 0x3448;
            let meta_offset = channels.len() as u32 * ChannelMetadata::ENTRY_SIZE;
            let data_offset: u32 = channels.iter().map(|(c, _)| c.data_size()).sum();
            meta.data_addr = FileAddr::from(header_offset + meta_offset + data_offset);

            self.write_channel_metadata(&meta)?;
            self.write_samples(meta.data_addr, samples)?;
        }

        Ok(())
    }

    fn write_channel_metadata(&mut self, channel: &ChannelMetadata) -> I2Result<()> {
        self.sink
            .write_u32::<LittleEndian>(channel.prev_addr.into())?;
        self.sink
            .write_u32::<LittleEndian>(channel.next_addr.into())?;
        self.sink
            .write_u32::<LittleEndian>(channel.data_addr.into())?;
        self.sink.write_u32::<LittleEndian>(channel.data_count)?;

        // TODO: Not sure what this is...
        self.sink.write_u16::<LittleEndian>(4u16)?;

        self.sink
            .write_u16::<LittleEndian>(channel.datatype._type())?;
        self.sink
            .write_u16::<LittleEndian>(channel.datatype.size())?;

        self.sink.write_u16::<LittleEndian>(channel.sample_rate)?;

        self.sink.write_u16::<LittleEndian>(channel.offset)?;
        self.sink.write_u16::<LittleEndian>(channel.mul)?;
        self.sink.write_u16::<LittleEndian>(channel.scale)?;
        self.sink.write_i16::<LittleEndian>(channel.dec_places)?;

        self.write_string(32, &channel.name)?;
        self.write_string(8, &channel.short_name)?;
        self.write_string(12, &channel.unit)?;

        // TODO: Not sure what this is...
        self.sink.write_u8(201)?;
        self.sink.write(&[0u8; 39])?;
        Ok(())
    }

    fn write_samples(&mut self, addr: FileAddr, sample: &Vec<Sample>) -> I2Result<()> {
        self.sink.seek(addr.seek())?;

        for s in sample {
            match s {
                Sample::I16(i) => self.sink.write_i16::<LittleEndian>(*i)?,
                Sample::I32(i) => self.sink.write_i32::<LittleEndian>(*i)?,
                Sample::F32(f) => self.sink.write_f32::<LittleEndian>(*f)?,
            }
        }

        Ok(())
    }

    /// Writes a string in a field up to `max_len`
    ///
    /// The I2 format (as far as we understand) stores strings as utf8 bytes with 0 bytes for padding
    pub(crate) fn write_string(&mut self, max_len: usize, string: &str) -> I2Result<()> {
        let bytes: Vec<u8> = string.bytes().take(max_len).collect();
        self.sink.write(&bytes[..])?;
        let zeros: Vec<u8> = iter::repeat(0).take(max_len - bytes.len()).collect();
        self.sink.write(&zeros[..])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ChannelMetadata, Datatype, FileAddr, LDWriter};
    use std::io::Cursor;
    use std::iter;

    #[test]
    fn test_write_string() {
        let bytes: Vec<u8> = iter::repeat(1u8).take(8).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor);

        writer.write_string(8, "OK").unwrap();

        let bytes = cursor.into_inner();
        assert_eq!(bytes, [79, 75, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_write_string_max_len() {
        let bytes: Vec<u8> = iter::repeat(1u8).take(8).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor);

        writer.write_string(8, "test123456").unwrap();

        let bytes = cursor.into_inner();
        assert_eq!(bytes, [116, 101, 115, 116, 49, 50, 51, 52]);
    }

    #[test]
    fn test_write_channel_metadata() {
        let bytes: Vec<u8> = iter::repeat(0u8).take(124).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor);

        writer
            .write_channel_metadata(&ChannelMetadata {
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
                name: "Air Temp Inlet".to_string(),
                short_name: "Air Tem".to_string(),
                unit: "C".to_string(),
            })
            .unwrap();

        const EXPECTED: [u8; 124] = [
            0x00, 0x00, 0x00, 0x00, 0xC4, 0x34, 0x00, 0x00, 0x10, 0x5A, 0x00, 0x00, 0x8C, 0x03,
            0x00, 0x00, 0x04, 0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x70, 0x20, 0x49,
            0x6E, 0x6C, 0x65, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65,
            0x6D, 0x00, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xC9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let bytes = cursor.into_inner();
        assert_eq!(bytes, EXPECTED);
    }
}
