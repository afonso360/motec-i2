use motec_i2::{ChannelMetadata, Datatype, Header, I2Result, LDReader, LDWriter, Sample};
use std::env;
use std::fs::File;

fn main() -> I2Result<()> {
    let filename = "test_write.ld";
    println!("Writing file: {}", filename);

    let mut file = File::create(filename).expect("Failed to open file!");
    let mut writer = LDWriter::new(&mut file);

    writer.write_header(&Header {
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
        pro_logging_bytes: 13764642,
    });

    // Event: Some(
    //     Event {
    //     name: "i2 data day",
    //     session: "2",
    //     comment: "Calder Park, 23/11/05, fine sunny day",
    //     venue_addr: 4918,
    // },
    // )
    // Venue: Some(
    //     Venue {
    //     name: "Calder",
    //     vehicle_addr: 8020,
    // },
    // )
    // Vehicle: Some(
    //     Vehicle {
    //     id: "11A",
    //     weight: 0,
    //     _type: "Car",
    //     comment: "",
    // },
    // )

    let channel0_meta = ChannelMetadata {
        prev_addr: 0,
        next_addr: 0,
        data_addr: 0,
        data_count: 0,
        datatype: Datatype::I16,
        sample_rate: 2,
        shift: 0,
        mul: 1,
        scale: 1,
        dec_places: 1,
        name: "Air Temp Inlet".to_string(),
        short_name: "Air Tem".to_string(),
        unit: "C".to_string(),
    };
    let channel0_samples = vec![
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
    let channels = vec![(channel0_meta, channel0_samples)];
    writer.write_channels(channels)?;
    Ok(())
}