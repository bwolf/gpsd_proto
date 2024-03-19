#[macro_use]
extern crate log;

use std::io;
use std::net::TcpStream;

use gpsd_proto::{get_data, handshake, GpsdError, ResponseData};
use itertools::Itertools;

pub fn demo_forever<R>(
    reader: &mut dyn io::BufRead,
    writer: &mut io::BufWriter<R>,
) -> Result<(), GpsdError>
where
    R: std::io::Write,
{
    handshake(reader, writer)?;

    loop {
        let msg = match get_data(reader) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error: {:?}", e);
                continue;
            }
        };
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
                let sats = sky.satellites.map_or_else(
                    || "(none)".to_owned(),
                    |sats| {
                        sats.iter()
                            .filter(|sat| sat.used)
                            .map(|sat| sat.prn.to_string())
                            .join(",")
                    },
                );
                println!(
                    "Sky xdop {:4.2} ydop {:4.2} vdop {:4.2}, satellites {}",
                    sky.xdop.unwrap_or(0.0),
                    sky.ydop.unwrap_or(0.0),
                    sky.vdop.unwrap_or(0.0),
                    sats
                );
            }
            ResponseData::Pps(p) => {
                println!(
                    "PPS {} real: {} s {} ns clock: {} s {} ns precision: {:?}",
                    p.device, p.real_sec, p.real_nsec, p.clock_sec, p.clock_nsec, p.precision,
                );
            }
            ResponseData::Gst(g) => {
                println!(
                    "GST {} time: {} rms: {} major: {} m minor: {} m orient: {}° lat: {} m lon: {} m alt: {} m",
                    g.device.unwrap_or("".to_string()), g.time.unwrap_or("".to_string()),
                    g.rms.unwrap_or(0.), g.major.unwrap_or(0.),
                    g.minor.unwrap_or(0.), g.orient.unwrap_or(0.),
                    g.lat.unwrap_or(0.), g.lon.unwrap_or(0.), g.alt.unwrap_or(0.),
                );
            }
            other => println!("Unexpected message {:#?}", other),
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
