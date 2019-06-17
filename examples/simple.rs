#[macro_use]
extern crate log;

use gpsd_proto::{get_data, handshake, GpsdError, ResponseData};
use itertools::Itertools;
use std::io;
use std::net::TcpStream;

pub fn demo_forever<R>(
    reader: &mut io::BufRead,
    writer: &mut io::BufWriter<R>,
) -> Result<(), GpsdError>
where
    R: std::io::Write,
{
    handshake(reader, writer)?;

    loop {
        let msg = get_data(reader)?;
        match msg {
            ResponseData::Device {
                path,
                driver,
                activated,
                ..
            } => {
                debug!(
                    "DEVICE {} {} {}",
                    path.unwrap_or("".to_string()),
                    driver.unwrap_or("".to_string()),
                    activated.unwrap_or("".to_string()),
                );
            }
            ResponseData::Tpv {
                device: _,
                mode,
                time: _,
                ept: _,
                lat,
                lon,
                alt,
                epx: _,
                epy: _,
                epv: _,
                track,
                speed,
                ..
            } => {
                println!(
                    "{:3} {:8.5} {:8.5} {:6.1} m {:5.1} ° {:6.3} m/s",
                    mode.to_string(),
                    lat.unwrap_or(0.0),
                    lon.unwrap_or(0.0),
                    alt.unwrap_or(0.0),
                    track.unwrap_or(0.0),
                    speed.unwrap_or(0.0),
                );
            }
            ResponseData::Sky {
                device: _,
                xdop,
                ydop,
                vdop,
                tdop: _,
                hdop: _,
                gdop: _,
                pdop: _,
                satellites,
            } => {
                let sats = satellites
                    .iter()
                    .filter(|sat| sat.used)
                    .map(|sat| sat.prn.to_string())
                    .join(",");
                println!(
                    "Sky xdop {:4.2} ydop {:4.2} vdop {:4.2}, satellites {}",
                    xdop.unwrap_or(0.0),
                    ydop.unwrap_or(0.0),
                    vdop.unwrap_or(0.0),
                    sats
                );
            }
        }
    }
}

fn main() {
    env_logger::init();
    if let Ok(stream) = TcpStream::connect("127.0.0.1:2947") {
        info!("Connected to gpsd");
        let mut reader = io::BufReader::new(&stream);
        let mut writer = io::BufWriter::new(&stream);
        demo_forever(&mut reader, &mut writer).unwrap();
    } else {
        panic!("Couldn't connect to gpsd...");
    }
}
