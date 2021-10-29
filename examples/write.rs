use motec_i2::{Channel, Datatype, FileAddr, Header, I2Result, LDWriter, Sample};
use std::fs::File;

fn main() -> I2Result<()> {
    let filename = "test_write.ld";
    println!("Writing file: {}", filename);

    let mut file = File::create(filename).expect("Failed to open file!");
    let mut writer = LDWriter::new(&mut file);

    writer.write_header(&Header {
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
    })?;

    let channel0 = Channel {
        datatype: Datatype::I16,
        sample_rate: 2,
        offset: 2,
        mul: 1,
        scale: 1,
        dec_places: 1,
        name: "Air Temp Inlet".to_string(),
        short_name: "Air Tem".to_string(),
        unit: "C".to_string(),
    };
    let channel0_data = vec![
        Sample::I16(190),
        Sample::I16(190),
        Sample::I16(190),
        Sample::I16(190),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(200),
        Sample::I16(190),
        Sample::I16(190),
        Sample::I16(190),
    ];

    let id = writer.write_channel(&channel0, &channel0_data)?;
    writer.write_channel_data(id, &channel0_data)?;

    writer.finish()?;
    Ok(())
}
