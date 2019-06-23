#[macro_use]
extern crate log;

use futures::prelude::*;
use futures::stream::Stream;
use gpsd_proto::UnifiedResponse;
use std::net::SocketAddr;
use tokio::codec::Framed;
use tokio::codec::LinesCodec;
use tokio::net::TcpStream;

fn main() {
    env_logger::init();
    info!("Starting");

    let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();

    let program = TcpStream::connect(&addr)
        .and_then(|sock| {
            let framed = Framed::new(sock, LinesCodec::new());
            framed
                .send(gpsd_proto::ENABLE_WATCH_CMD.to_string())
                .and_then(|framed| {
                    framed.for_each(|line| {
                        trace!("Raw {}", &line);
                        match serde_json::from_str(&line) {
                            Ok(rd) => match rd {
                                UnifiedResponse::Version(v) => {
                                    if v.proto_major < gpsd_proto::PROTO_MAJOR_MIN {
                                        panic!("Gpsd major version mismatch");
                                    }
                                    info!("Gpsd version {} connected", v.rev);
                                }
                                UnifiedResponse::Devices(_) => {}
                                UnifiedResponse::Watch(_) => {}
                                UnifiedResponse::Device(d) => debug!("Device {:?}", d),
                                UnifiedResponse::Tpv(t) => debug!("Tpv {:?}", t),
                                UnifiedResponse::Sky(s) => debug!("Sky {:?}", s),
                                UnifiedResponse::Pps(p) => debug!("PPS {:?}", p),
                            },
                            Err(e) => {
                                error!("Error decoding: {}", e);
                            }
                        };
                        Ok(())
                    })
                })
        })
        .map_err(|e| error!("Failure {:?}", e));

    tokio::run(program);
}
