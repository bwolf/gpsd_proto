#[macro_use]
extern crate log;

use std::error::Error;
use std::net::SocketAddr;

use futures::future::ready;
use futures::prelude::*;
use gpsd_proto::UnifiedResponse;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting");

    let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();

    let stream = TcpStream::connect(&addr).await?;
    let mut framed = Framed::new(stream, LinesCodec::new());

    framed.send(gpsd_proto::ENABLE_WATCH_CMD).await?;
    framed
        .try_for_each(|line| {
            trace!("Raw {line}");

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
                    UnifiedResponse::Device(d) => debug!("Device {d:?}"),
                    UnifiedResponse::Tpv(t) => debug!("Tpv {t:?}"),
                    UnifiedResponse::Sky(s) => debug!("Sky {s:?}"),
                    UnifiedResponse::Pps(p) => debug!("PPS {p:?}"),
                    UnifiedResponse::Gst(g) => debug!("GST {g:?}"),
                    other => debug!("Unexpected message {other:?}"),
                },
                Err(e) => {
                    error!("Error decoding: {e}");
                }
            };

            ready(Ok(()))
        })
        .await?;

    Ok(())
}
