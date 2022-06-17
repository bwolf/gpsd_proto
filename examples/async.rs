// TODO use futures::prelude::*;
// use futures::stream::Stream;

use futures::TryFutureExt;
use gpsd_proto::UnifiedResponse;
use std::net::SocketAddr;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;
use tokio::net::TcpStream;
use log::{info, trace};
use std::error::Error;
use futures::sink::SinkExt;
use futures::stream::Stream;
use futures::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting");

    // let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();

    let mut stream = TcpStream::connect("127.0.0.1:2947").await?;
    let mut framed = Framed::new(stream, LinesCodec::new());

    let xx = framed.feed(gpsd_proto::ENABLE_WATCH_CMD.to_string()).await?;

    while let Some(Ok(line)) = framed.next().await {
        match line.as_str() {
            "READY" => {},
            _ => println!("{}", line),
        }
    }

    // xx.and_then(|framed| {
        // framed.for_each
        // Ok(())
    // });

        // .and_then(|framed| {
            // Ok(())
        // });

    // let program = TcpStream::connect(&addr)
    //     .and_then(|sock| {
    //         let framed = Framed::new(sock, LinesCodec::new());
    //         framed
    //             .send(gpsd_proto::ENABLE_WATCH_CMD.to_string())
    //             .and_then(|framed| {
    //                 framed.for_each(|line| {
    //                     trace!("Raw {}", &line);
                        // match serde_json::from_str(&line) {
                        //     Ok(rd) => match rd {
                        //         UnifiedResponse::Version(v) => {
                        //             if v.proto_major < gpsd_proto::PROTO_MAJOR_MIN {
                        //                 panic!("Gpsd major version mismatch");
                        //             }
                        //             info!("Gpsd version {} connected", v.rev);
                        //         }
                        //         UnifiedResponse::Devices(_) => {}
                        //         UnifiedResponse::Watch(_) => {}
                        //         UnifiedResponse::Device(d) => debug!("Device {:?}", d),
                        //         UnifiedResponse::Tpv(t) => debug!("Tpv {:?}", t),
                        //         UnifiedResponse::Sky(s) => debug!("Sky {:?}", s),
                        //         UnifiedResponse::Pps(p) => debug!("PPS {:?}", p),
                        //         UnifiedResponse::Gst(g) => debug!("GST {:?}", g),
                        //     },
                        //     Err(e) => {
                        //         error!("Error decoding: {}", e);
                        //     }
                        // };
        //                 Ok(())
        //             })
        //         })
        // })
        // .map_err(|e| error!("Failure {:?}", e));

    // tokio::run(program);

    Ok(())
}
