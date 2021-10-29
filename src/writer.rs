use crate::full_header::FULL_HEADER;
use crate::{Channel, FileAddr, FileChannel, Header, I2Result, Sample, LD_HEADER_MARKER};
use byteorder::{LittleEndian, WriteBytesExt};
use core::iter;
use std::io::{Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct ChannelId {
    id: usize,
    data_size: u32,
}

#[derive(Debug)]
pub struct LDWriter<'a, S: Write + Seek> {
    sink: &'a mut S,
    channels: Vec<FileAddr>,
    channel_data_blocks: Vec<FileAddr>,
    write_pos: FileAddr,
}
// TODO: Do something like this to ensure that the file cannot be written to wrongly
// pub enum LD<'a, S: Write + Seek> {
//     Header { sink: &'a mut S },
//     Channels { sink: &'a mut S },
//     Data { sink: &'a mut S },
// }

impl<'a, S: Write + Seek> LDWriter<'a, S> {
    pub fn new(sink: &'a mut S) -> Self {
        Self {
            sink,
            channels: Vec::new(),
            channel_data_blocks: Vec::new(),
            write_pos: FileAddr::zero(),
        }
    }

    pub fn write_header(&mut self, hdr: &Header) -> I2Result<()> {
        // See comments on FULL_HEADER for an explanation on why we do this.
        self.sink.seek(SeekFrom::Start(0))?;
        self.sink.write(&FULL_HEADER[..])?;
        // panic!();
        // Header is always at start
        self.sink.seek(SeekFrom::Start(0))?;

        self.sink.write_u32::<LittleEndian>(LD_HEADER_MARKER)?;

        // TODO: We don't know what this is, but Sample1.ld has it as 0
        self.sink.write_u32::<LittleEndian>(0x00000000)?;

        // This gets filled in later in the `finish` function
        self.sink.write_u32::<LittleEndian>(0)?; // hdr.channel_meta_ptr
        self.sink.write_u32::<LittleEndian>(0)?; // hdr.channel_data_ptr

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
        // ProLogging related
        self.sink.write_u32::<LittleEndian>(0xD20822)?;
        self.sink.write_u16::<LittleEndian>(0u16)?;

        self.write_string(64, &hdr.session)?;
        self.write_string(64, &hdr.short_comment)?;

        self.sink.write(&[0u8; 8])?;
        self.sink.write_u8(99)?;
        self.sink.write(&[0u8; 117])?;

        // TODO: Write Event

        // TODO: This should be based on what we write in the header...
        self.write_pos = FileAddr::from(0x3448u32);
        Ok(())
    }

    pub fn write_channel(&mut self, channel: &Channel, data: &Vec<Sample>) -> I2Result<ChannelId> {
        let channel_addr = self.write_pos.clone();
        let prev_addr = self.channels.last().copied().unwrap_or(FileAddr::zero());
        let next_addr = FileAddr::zero();
        let samples = data.len() as u32;
        // let data_addr = self.write_pos + FileChannel::ENTRY_SIZE;

        let file_channel = FileChannel {
            prev_addr,
            next_addr,
            data_addr: FileAddr::zero(),
            samples,
            channel: channel.clone(),
        };

        self.sink.seek(self.write_pos.seek())?;
        self.write_file_channel(&file_channel)?;

        // Update the previous channel to point to this one
        if !prev_addr.is_zero() {
            let prev_channel_next_addr = file_channel.prev_addr + FileChannel::NEXT_ADDR_OFFSET;
            self.sink.seek(prev_channel_next_addr.seek())?;
            self.sink.write_u32::<LittleEndian>(channel_addr.into())?;
        }

        let channel_id = ChannelId {
            id: self.channels.len(),
            data_size: file_channel.data_size(),
        };
        // Add this channel to the list of channels
        self.channels.push(channel_addr);

        // Update the current write head
        self.write_pos = channel_addr + FileChannel::ENTRY_SIZE;
        Ok(channel_id)
    }

    fn write_file_channel(&mut self, fc: &FileChannel) -> I2Result<()> {
        self.sink.write_u32::<LittleEndian>(fc.prev_addr.into())?;
        self.sink.write_u32::<LittleEndian>(fc.next_addr.into())?;
        self.sink.write_u32::<LittleEndian>(fc.data_addr.into())?;
        self.sink.write_u32::<LittleEndian>(fc.samples)?;

        // TODO: Not sure what this is...
        self.sink.write_u16::<LittleEndian>(4u16)?;

        self.sink
            .write_u16::<LittleEndian>(fc.channel.datatype._type())?;
        self.sink
            .write_u16::<LittleEndian>(fc.channel.datatype.size())?;

        self.sink
            .write_u16::<LittleEndian>(fc.channel.sample_rate)?;

        self.sink.write_u16::<LittleEndian>(fc.channel.offset)?;
        self.sink.write_u16::<LittleEndian>(fc.channel.mul)?;
        self.sink.write_u16::<LittleEndian>(fc.channel.scale)?;
        self.sink.write_i16::<LittleEndian>(fc.channel.dec_places)?;

        self.write_string(32, &fc.channel.name)?;
        self.write_string(8, &fc.channel.short_name)?;
        self.write_string(12, &fc.channel.unit)?;

        // TODO: Not sure what this is...
        self.sink.write_u8(201)?;
        self.sink.write(&[0u8; 39])?;
        Ok(())
    }

    pub fn write_channel_data(
        &mut self,
        channel: ChannelId,
        samples: &Vec<Sample>,
    ) -> I2Result<()> {
        self.sink.seek(self.write_pos.seek())?;
        let data_addr = self.write_pos.clone();

        self.write_samples(&samples)?;

        // Update the channel with the correct data address
        let channel_addr = self.channels[channel.id];
        self.sink
            .seek((channel_addr + FileChannel::DATA_ADDR_OFFSET).seek())?;
        self.sink.write_u32::<LittleEndian>(data_addr.into())?;

        self.channel_data_blocks.push(data_addr);
        self.write_pos = data_addr + channel.data_size;
        Ok(())
    }

    fn write_samples(&mut self, samples: &Vec<Sample>) -> I2Result<()> {
        for s in samples {
            match s {
                Sample::I16(i) => self.sink.write_i16::<LittleEndian>(*i)?,
                Sample::I32(i) => self.sink.write_i32::<LittleEndian>(*i)?,
                Sample::F32(f) => self.sink.write_f32::<LittleEndian>(*f)?,
            }
        }
        Ok(())
    }

    /// Finishes writing a file
    pub fn finish(self) -> I2Result<()> {
        let channel_meta_addr = self.channels.iter().min();
        let channel_data_addr = self.channel_data_blocks.iter().min();
        dbg!(channel_meta_addr);
        dbg!(channel_data_addr);

        match (channel_data_addr, channel_meta_addr) {
            (Some(daddr), Some(maddr)) => {
                self.sink
                    .seek(FileAddr::from(Header::CHANNEL_DATA_OFFSET as u32).seek())?;
                self.sink.write_u32::<LittleEndian>(daddr.as_u32())?;

                self.sink
                    .seek(FileAddr::from(Header::CHANNEL_META_OFFSET as u32).seek())?;
                self.sink.write_u32::<LittleEndian>(maddr.as_u32())?;
            }
            // TODO: Make this a error
            _ => panic!("No data written"),
        }

        Ok(())
    }

    // pub fn write_channels(
    //     &mut self,
    //     channels: Vec<(ChannelMetadata, Vec<Sample>)>,
    // ) -> I2Result<()> {
    //     // TODO: Should not be hardcoded
    //     self.sink.seek(SeekFrom::Start(0x3448))?;
    //
    //     for i in 0..channels.len() {
    //         let (meta, samples) = &channels[i];
    //         let mut meta = meta.clone();
    //         let i = i as u32;
    //
    //         meta.prev_addr = FileAddr::from(i * ChannelMetadata::ENTRY_SIZE);
    //         meta.next_addr =
    //             FileAddr::from(((i + 1) % (channels.len() as u32)) * ChannelMetadata::ENTRY_SIZE);
    //         meta.samples = samples.len() as u32;
    //
    //         let header_offset = 0x3448;
    //         let meta_offset = channels.len() as u32 * ChannelMetadata::ENTRY_SIZE;
    //         let data_offset: u32 = channels.iter().map(|(c, _)| c.data_size()).sum();
    //         meta.data_addr = FileAddr::from(header_offset + meta_offset + data_offset);
    //
    //         self.write_channel_metadata(&meta)?;
    //         self.write_samples(meta.data_addr, samples)?;
    //     }
    //
    //     Ok(())
    // }

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
    use crate::{Channel, Datatype, LDWriter, Sample};
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
    fn test_write_channel() {
        let bytes: Vec<u8> = iter::repeat(0u8).take(132).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor);

        let channel = Channel {
            datatype: Datatype::I16,
            sample_rate: 2,
            offset: 0,
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

        let cid = writer.write_channel(&channel, &samples).unwrap();
        writer.write_channel_data(cid, &samples).unwrap();

        const EXPECTED: [u8; 132] = [
            0x00, 0x00, 0x00, 0x00, // prev_addr
            0x00, 0x00, 0x00, 0x00, // next_addr
            0x7C, 0x00, 0x00, 0x00, // data_addr
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

        let bytes = cursor.into_inner();
        assert_eq!(bytes, EXPECTED);
    }

    /// When writing multiple channels we have to go back and update the previous channels
    #[test]
    fn test_write_multi_channel() {
        let bytes: Vec<u8> = iter::repeat(0u8).take(248).collect();
        let mut cursor = Cursor::new(bytes);
        let mut writer = LDWriter::new(&mut cursor);

        let channel0 = Channel {
            datatype: Datatype::I16,
            sample_rate: 2,
            offset: 0,
            mul: 1,
            scale: 1,
            dec_places: 1,
            name: "Air Temp Inlet".to_string(),
            short_name: "Air Tem".to_string(),
            unit: "C".to_string(),
        };
        let channel0_samples = vec![];

        let channel1 = Channel {
            datatype: Datatype::I32,
            sample_rate: 10,
            offset: 1,
            mul: 2,
            scale: 2,
            dec_places: 2,
            name: "Engine temp".to_string(),
            short_name: "EngTemp".to_string(),
            unit: "C".to_string(),
        };
        let channel1_samples = vec![];

        let c0id = writer.write_channel(&channel0, &channel0_samples).unwrap();
        let c1id = writer.write_channel(&channel1, &channel1_samples).unwrap();

        writer.write_channel_data(c0id, &channel0_samples).unwrap();
        writer.write_channel_data(c1id, &channel1_samples).unwrap();

        const EXPECTED: [u8; 248] = [
            // Channel 1
            0x00, 0x00, 0x00, 0x00, // prev_addr
            0x7C, 0x00, 0x00, 0x00, // next_addr
            0xF8, 0x00, 0x00, 0x00, // data_addr
            0x00, 0x00, 0x00, 0x00, // samples
            // Channel
            0x04, 0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x70, 0x20, 0x49, 0x6E, 0x6C,
            0x65, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x69, 0x72, 0x20, 0x54, 0x65, 0x6D, 0x00,
            0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC9, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Channel end
            // Channel 2
            0x00, 0x00, 0x00, 0x00, // prev_addr
            0x00, 0x00, 0x00, 0x00, // next_addr
            0xF8, 0x00, 0x00, 0x00, // data_addr
            0x00, 0x00, 0x00, 0x00, // samples
            // Channel
            0x04, 0x00, // unk
            0x03, 0x00, // datatype._type
            0x04, 0x00, // datatype.size
            0x0A, 0x00, // sample rate
            0x01, 0x00, // offset
            0x02, 0x00, // mul
            0x02, 0x00, // scale
            0x02, 0x00, // dec_places
            0x45, 0x6E, 0x67, 0x69, 0x6E, 0x65, 0x20, 0x74, 0x65, 0x6D, 0x70, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, // channel name
            0x45, 0x6E, 0x67, 0x54, 0x65, 0x6D, 0x70, 0x00, // short name
            0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // unit
            // Rest
            0xC9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, // Channel end
        ];

        let bytes = cursor.into_inner();
        assert_eq!(bytes, EXPECTED);
    }
}
