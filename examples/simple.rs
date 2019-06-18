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
            ResponseData::Device(d) => {
                debug!(
                    "DEVICE {} {} {}",
                    d.path.unwrap_or("".to_string()),
                    d.driver.unwrap_or("".to_string()),
                    d.activated.unwrap_or("".to_string()),
                );
            }
            ResponseData::Tpv(t) => {
                println!(
                    "{:3} {:8.5} {:8.5} {:6.1} m {:5.1} ° {:6.3} m/s",
                    t.mode.to_string(),
                    t.lat.unwrap_or(0.0),
                    t.lon.unwrap_or(0.0),
                    t.alt.unwrap_or(0.0),
                    t.track.unwrap_or(0.0),
                    t.speed.unwrap_or(0.0),
                );
            }
            ResponseData::Sky(sky) => {
                let sats = sky
                    .satellites
                    .iter()
                    .filter(|sat| sat.used)
                    .map(|sat| sat.prn.to_string())
                    .join(",");
                println!(
                    "Sky xdop {:4.2} ydop {:4.2} vdop {:4.2}, satellites {}",
                    sky.xdop.unwrap_or(0.0),
                    sky.ydop.unwrap_or(0.0),
                    sky.vdop.unwrap_or(0.0),
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
