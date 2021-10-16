use crate::full_header::FULL_HEADER;
use crate::{Header, I2Result, LD_HEADER_MARKER};
use byteorder::{LittleEndian, WriteBytesExt};
use core::iter;
use std::io::{Seek, SeekFrom, Write};

//C:/Users/Afonso/.cargo/bin/cargo.exe test --color=always --package motec-i2 --lib reader::tests::read_sample1_channel_data --no-fail-fast -- --format=json --exact -Z unstable-options --show-output
// Testing started at 16:39 ...
// warning: unused import: `crate::reader::LDReader`
//    --> src\writer.rs:102:9
//     |
// 102 |     use crate::reader::LDReader;
//     |         ^^^^^^^^^^^^^^^^^^^^^^^
//     |
//     = note: `#[warn(unused_imports)]` on by default
//
// warning: unused imports: `ChannelMetadata`, `Datatype`, `Header`, `Sample`
//    --> src\writer.rs:103:17
//     |
// 103 |     use crate::{ChannelMetadata, Datatype, Header, LDWriter, Sample};
//     |                 ^^^^^^^^^^^^^^^  ^^^^^^^^  ^^^^^^            ^^^^^^
//
// warning: unused import: `fs`
//    --> src\writer.rs:105:15
//     |
// 105 |     use std::{fs, iter};
//     |               ^^
//
// warning: variable does not need to be mutable
//    --> src\writer.rs:109:13
//     |
// 109 |         let mut bytes: Vec<u8> = iter::repeat(1u8).take(8).collect();
//     |             ----^^^^^
//     |             |
//     |             help: remove this `mut`
//     |
//     = note: `#[warn(unused_mut)]` on by default
//
// warning: variable does not need to be mutable
//    --> src\writer.rs:121:13
//     |
// 121 |         let mut bytes: Vec<u8> = iter::repeat(1u8).take(8).collect();
//     |             ----^^^^^
//     |             |
//     |             help: remove this `mut`
//
// warning: unused `Result` that must be used
//   --> src\writer.rs:72:9
//    |
// 72 |         self.sink.write_u16::<LittleEndian>(0u16);
//    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//    |
//    = note: `#[warn(unused_must_use)]` on by default
//    = note: this `Result` may be an `Err` variant, which should be handled
//
// warning: `motec-i2` (lib test) generated 6 warnings
//     Finished test [unoptimized + debuginfo] target(s) in 0.02s
//      Running unittests (target\debug\deps\motec_i2-b754b7a30fad338a.exe)
// Rip: Ok(
//     Header {
//         channel_meta_ptr: 13384,
//         channel_data_ptr: 23056,
//         event_ptr: 1762,
//         device_serial: 12007,
//         device_type: "ADL",
//         device_version: 420,
//         num_channels: 78,
//         date_string: "23/11/2005",
//         time_string: "09:53:00",
//         driver: "",
//         vehicleid: "11A",
//         venue: "Calder",
//         session: "2",
//         short_comment: "second warmup",
//         event: "i2 data day",
//         pro_logging_bytes: 13764642,
//     },
// )
//
// Process finished with exit code 0

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
    use crate::LDWriter;
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
}
