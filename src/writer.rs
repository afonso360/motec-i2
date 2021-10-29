use crate::full_header::FULL_HEADER;
use crate::{ChannelMetadata, Header, I2Result, Sample, LD_HEADER_MARKER};
use byteorder::{LittleEndian, WriteBytesExt};
use core::iter;
use std::io::{Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct LDWriter<'a, S: Write + Seek> {
    sink: &'a mut S,
    header: Header,
    channels: Vec<(ChannelMetadata, Vec<Sample>)>,
}

impl<'a, S: Write + Seek> LDWriter<'a, S> {
    pub fn new(sink: &'a mut S, header: Header) -> Self {
        Self {
            sink,
            header,
            channels: Vec::new(),
        }
    }

    pub fn with_channel(mut self, channel: ChannelMetadata, data: Vec<Sample>) -> Self {
        self.channels.push((channel, data));
        self
    }

    pub fn write(mut self) -> I2Result<()> {
        // TODO: Fix these clones
        self.write_header(&self.header.clone())?;
        self.write_channels(self.channels.clone())?;
        Ok(())
    }

    fn write_header(&mut self, hdr: &Header) -> I2Result<()> {
        // See comments on FULL_HEADER for an explanation on why we do this.
        self.sink.seek(SeekFrom::Start(0))?;
        self.sink.write(&FULL_HEADER[..])?;

        // Header is always at start
        self.sink.seek(SeekFrom::Start(0))?;

        self.sink.write_u32::<LittleEndian>(LD_HEADER_MARKER)?;

        // TODO: We don't know what this is, but Sample1.ld has it as 0
        self.sink.write_u32::<LittleEndian>(0x00000000)?;

        self.sink.write_u32::<LittleEndian>(hdr.channel_meta_ptr)?;
        self.sink.write_u32::<LittleEndian>(hdr.channel_data_ptr)?;

        // TODO: We don't know what this is, but Sample1.ld has it as 0
        self.sink.write(&[0u8; 20][..])?;

        self.sink.write_u32::<LittleEndian>(hdr.event_ptr)?;

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
        self.write_string(64, "")?;
        self.write_string(64, &hdr.venue)?;
        self.write_string(64, "")?;

        self.sink.write(&[0u8; 1024])?;

        // 0xD20822 for Sample1.ld
        // ProLogging related
        self.sink.write_u32::<LittleEndian>(0xD20822)?;
        self.sink.write_u16::<LittleEndian>(0u16)?;

        self.write_string(64, &hdr.session)?;
        self.write_string(64, &hdr.short_comment)?;

        self.sink.write(&[0u8; 8])?;
        self.sink.write_u8(99)?;
        self.sink.write(&[0u8; 117])?;

        // TODO: Write Event

        Ok(())
    }

    fn write_channels(&mut self, channels: Vec<(ChannelMetadata, Vec<Sample>)>) -> I2Result<()> {
        // TODO: Should not be hardcoded
        self.sink.seek(SeekFrom::Start(0x3448))?;

        for i in 0..channels.len() {
            let (meta, samples) = &channels[i];
            let mut meta = meta.clone();
            let i = i as u32;

            meta.prev_addr = i * ChannelMetadata::ENTRY_SIZE;
            meta.next_addr = ((i + 1) % (channels.len() as u32)) * ChannelMetadata::ENTRY_SIZE;
            meta.data_count = samples.len() as u32;

            let header_offset = 0x3448;
            let meta_offset = channels.len() as u32 * ChannelMetadata::ENTRY_SIZE;
            let data_offset: u32 = channels.iter().map(|(c, _)| c.data_size()).sum();
            meta.data_addr = header_offset + meta_offset + data_offset;

            self.write_channel_metadata(&meta)?;
            self.write_samples(meta.data_addr, samples)?;
        }

        Ok(())
    }

    fn write_channel_metadata(&mut self, channel: &ChannelMetadata) -> I2Result<()> {
        self.sink.write_u32::<LittleEndian>(channel.prev_addr)?;
        self.sink.write_u32::<LittleEndian>(channel.next_addr)?;
        self.sink.write_u32::<LittleEndian>(channel.data_addr)?;
        self.sink.write_u32::<LittleEndian>(channel.data_count)?;

        // TODO: Not sure what this is...
        self.sink.write_u16::<LittleEndian>(4u16)?;

        self.sink
            .write_u16::<LittleEndian>(channel.datatype._type())?;
        self.sink
            .write_u16::<LittleEndian>(channel.datatype.size())?;

        self.sink.write_u16::<LittleEndian>(channel.sample_rate)?;

        self.sink.write_u16::<LittleEndian>(channel.shift)?;
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

    fn write_samples(&mut self, addr: u32, sample: &Vec<Sample>) -> I2Result<()> {
        self.sink.seek(SeekFrom::Start(addr as u64))?;

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
    use crate::{ChannelMetadata, Datatype, Header, LDWriter, Sample};
    use std::io::Cursor;
    use std::iter;

    fn sample_header() -> Header {
        Header {
            channel_meta_ptr: 13384,
            channel_data_ptr: 23056,
            event_ptr: 1762,
            device_serial: 12007,
            device_type: "ADL".to_string(),
            device_version: 420,
            num_channels: 1,
            date_string: "23/11/2005".to_string(),
            time_string: "09:53:00".to_string(),
            driver: "".to_string(),
            vehicleid: "11A".to_string(),
            venue: "Calder".to_string(),
            session: "2".to_string(),
            short_comment: "second warmup".to_string(),
        }
    }

    #[test]
    fn test_write_string() {
        let bytes: Vec<u8> = iter::repeat(1u8).take(8).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor, sample_header());

        writer.write_string(8, "OK").unwrap();

        let bytes = cursor.into_inner();
        assert_eq!(bytes, [79, 75, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_write_string_max_len() {
        let bytes: Vec<u8> = iter::repeat(1u8).take(8).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor, sample_header());

        writer.write_string(8, "test123456").unwrap();

        let bytes = cursor.into_inner();
        assert_eq!(bytes, [116, 101, 115, 116, 49, 50, 51, 52]);
    }

    #[test]
    fn test_write_single_channel() {
        let total_size = 13384 + 132; // header + 1 channel + samples
        let bytes: Vec<u8> = iter::repeat(0u8).take(total_size).collect();
        let mut cursor = Cursor::new(bytes);

        let channel = ChannelMetadata {
            prev_addr: 0,
            next_addr: 0,
            data_addr: 0,
            data_count: 0,
            datatype: Datatype::I16,
            sample_rate: 2,
            shift: 0,
            // offset: 0,
            mul: 1,
            scale: 1,
            dec_places: 1,
            name: "Air Temp Inlet".to_string(),
            short_name: "Air Tem".to_string(),
            unit: "C".to_string(),
        };

        let samples = vec![
            Sample::I16(0),
            Sample::I16(1),
            Sample::I16(2),
            Sample::I16(3),
        ];

        LDWriter::new(&mut cursor, sample_header())
            .with_channel(channel, samples)
            .write()
            .unwrap();

        const EXPECTED: [u8; 132] = [
            0x00, 0x00, 0x00, 0x00, // prev_addr
            0x00, 0x00, 0x00, 0x00, // next_addr
            0xC4, 0x34, 0x00, 0x00, // data_addr
            0x04, 0x00, 0x00, 0x00, // samples
            // Channel
            0x04, 0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x70, 0x20, 0x49, 0x6E, 0x6C,
            0x65, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x00,
            0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC9, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Channel end
            // Samples
            0x00, 0x00, // Sample 1
            0x01, 0x00, // Sample 2
            0x02, 0x00, // Sample 3
            0x03, 0x00, // Sample 4
        ];

        let channel_data = cursor.into_inner();
        assert_eq!(channel_data[13384..], EXPECTED);
    }

    //
    // #[test]
    // fn test_write_channel() {

    // }
    //
    // /// When writing multiple channels we have to go back and update the previous channels
    // #[test]
    // fn test_write_multi_channel() {
    //     let bytes: Vec<u8> = iter::repeat(0u8).take(248).collect();
    //     let mut cursor = Cursor::new(bytes);
    //     let mut writer = LDWriter::new(&mut cursor);
    //
    //     let channel0 = Channel {
    //         datatype: Datatype::I16,
    //         sample_rate: 2,
    //         offset: 0,
    //         mul: 1,
    //         scale: 1,
    //         dec_places: 1,
    //         name: "Air Temp Inlet".to_string(),
    //         short_name: "Air Tem".to_string(),
    //         unit: "C".to_string(),
    //     };
    //     let channel0_samples = vec![];
    //
    //     let channel1 = Channel {
    //         datatype: Datatype::I32,
    //         sample_rate: 10,
    //         offset: 1,
    //         mul: 2,
    //         scale: 2,
    //         dec_places: 2,
    //         name: "Engine temp".to_string(),
    //         short_name: "EngTemp".to_string(),
    //         unit: "C".to_string(),
    //     };
    //     let channel1_samples = vec![];
    //
    //     let c0id = writer.write_channel(&channel0, &channel0_samples).unwrap();
    //     let c1id = writer.write_channel(&channel1, &channel1_samples).unwrap();
    //
    //     writer.write_channel_data(c0id, &channel0_samples).unwrap();
    //     writer.write_channel_data(c1id, &channel1_samples).unwrap();
    //
    //     const EXPECTED: [u8; 248] = [
    //         // Channel 1
    //         0x00, 0x00, 0x00, 0x00, // prev_addr
    //         0x7C, 0x00, 0x00, 0x00, // next_addr
    //         0xF8, 0x00, 0x00, 0x00, // data_addr
    //         0x00, 0x00, 0x00, 0x00, // samples
    //         // Channel
    //         0x04, 0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
    //         0x01, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x70, 0x20, 0x49, 0x6E, 0x6C,
    //         0x65, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x00,
    //         0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC9, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Channel end
    //         // Channel 2
    //         0x00, 0x00, 0x00, 0x00, // prev_addr
    //         0x00, 0x00, 0x00, 0x00, // next_addr
    //         0xF8, 0x00, 0x00, 0x00, // data_addr
    //         0x00, 0x00, 0x00, 0x00, // samples
    //         // Channel
    //         0x04, 0x00, // unk
    //         0x03, 0x00, // datatype._type
    //         0x04, 0x00, // datatype.size
    //         0x0A, 0x00, // sample rate
    //         0x01, 0x00, // offset
    //         0x02, 0x00, // mul
    //         0x02, 0x00, // scale
    //         0x02, 0x00, // dec_places
    //         0x45, 0x6E, 0x67, 0x69, 0x6E, 0x65, 0x20, 0x74, 0x65, 0x6D, 0x70, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, // channel name
    //         0x45, 0x6E, 0x67, 0x54, 0x65, 0x6D, 0x70, 0x00, // short name
    //         0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // unit
    //         // Rest
    //         0xC9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x00, // Channel end
    //     ];
    //
    //     let bytes = cursor.into_inner();
    //     assert_eq!(bytes, EXPECTED);
    // }
}
