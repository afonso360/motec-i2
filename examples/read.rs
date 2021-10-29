use motec_i2::{I2Result, LDReader};
use std::env;
use std::fs::File;

fn main() -> I2Result<()> {
    let path = env::args()
        .skip(1)
        .next()
        .unwrap_or("./samples/Sample1.ld".into());
    println!("Reading file: {}", path);

    let mut file = File::open(path).expect("Failed to open file!");
    let mut reader = LDReader::new(&mut file);

    let header = reader.read_header()?;
    println!("Header: {:#?}", header);

    let event = reader.read_event()?;
    println!("Event: {:#?}", event);

    let venue = reader.read_venue()?;
    println!("Venue: {:#?}", venue);

    let vehicle = reader.read_vehicle()?;
    println!("Vehicle: {:#?}", vehicle);

    let channels = reader.read_channels()?;
    println!("File has {} channels", channels.len());

    let channel = &channels[0];
    println!(
        "Reading channel 0: {} ({} samples at {} Hz)",
        channel.channel.name, channel.samples, channel.channel.sample_rate
    );
    println!("Channle: {:#?}", channel);

    let data = reader.channel_data(channel)?;
    for i in 0..6 {
        let sample = &data[i];
        let value = sample.decode_f64(&channel.channel);
        println!("[{}]: {:.1} - (Raw Sample: {:?})", i, value, sample);
    }

    Ok(())
}
