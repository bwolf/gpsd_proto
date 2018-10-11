//! The `gpsd_proto` module contains types and functions to connect to
//! [gpsd](http://catb.org/gpsd/) to get GPS coordinates and satellite
//! information.
//!
//! `gpsd_proto` uses a plain TCP socket to connect to `gpsd`, reads
//! and writes JSON messages. The main motivation to create this crate
//! was independence from C libraries, like `libgps` (provided by
//! `gpsd`) to ease cross compiling.
//!
//! A example demo application is provided in the `example` sub
//! directory. Check the repository for up to date sample code.
//!
//! # Testing
//!
//! `gpsd_proto` has been tested against `gpsd` version 3.17 on macOS
//! with a GPS mice (Adopt SkyTraQ Venus 8) and the iOS app
//! [GPS2IP](http://www.capsicumdreams.com/iphone/gps2ip/).
//!
//! Feel free to report any other supported GPS by opening a GitHub
//! issue.
//!
//! # Reference documentation
//!
//! Important reference documentation of `gpsd` are the [JSON
//! protocol](http://www.catb.org/gpsd/gpsd_json.html) and the [client
//! HOWTO](http://catb.org/gpsd/client-howto.html).
//!
//! # Development notes
//!
//! Start `gpsd` with a real GPS device:
//!
//! ```sh
//! /usr/local/sbin/gpsd -N -D4 /dev/tty.SLAB_USBtoUART
//! ```
//!
//! Or start [gpsd](http://catb.org/gpsd/gpsd.html) with a TCP stream to a remote GPS:
//!
//! ```sh
//! /usr/local/sbin/gpsd -N -D2 tcp://192.168.177.147:11123
//! ```
//!
//! Test the connection to `gpsd` with `telnet localhost 2947` and send the string:
//!
//! ```text
//! ?WATCH={"enable":true,"json":true};
//! ```

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::io;
use std::io::Write;

use serde::de::*;
use serde::Deserializer;

/// Minimum supported version of `gpsd`.
pub const PROTO_MAJOR_MIN: u8 = 3;

/// Command to enable watch
pub const ENABLE_WATCH_CMD: &str = "?WATCH={\"enable\":true,\"json\":true};\r\n";

/// Simple device information as reported by `gpsd`
#[derive(Debug, Deserialize)]
pub struct DeviceInfo {
    /// Name the device for which the control bits are being reported,
    /// or for which they are to be applied. This attribute may be
    /// omitted only when there is exactly one subscribed channel.
    pub path: Option<String>,
    /// Time the device was activated as an ISO8601 timestamp. If the
    /// device is inactive this attribute is absent.
    pub activated: Option<String>,
}

/// Responses from `gpsd` during handshake
#[derive(Debug, Deserialize)]
#[serde(tag = "class")]
pub enum ResponseHandshake {
    /// `gpsd` ships a VERSION response to each client when the client
    /// first connects to it.
    #[serde(rename = "VERSION")]
    Version {
        /// Public release level.
        release: String,
        /// Internal revision-control level.
        rev: String,
        /// API major revision level.
        proto_major: u8,
        /// API minor revision level.
        proto_minor: u8,
        /// URL of the remote daemon reporting this version. If empty,
        /// this is the version of the local daemon.
        remote: Option<String>,
    },
    /// Device information (i.e. device enumeration)
    #[serde(rename = "DEVICES")]
    Devices { devices: Vec<DeviceInfo> },
    /// Watch response. Elicits a report of per-subscriber policy.
    #[serde(rename = "WATCH")]
    Watch {
        /// Enable (true) or disable (false) watcher mode. Default is
        /// true.
        enable: Option<bool>,
        /// Enable (true) or disable (false) dumping of JSON reports.
        /// Default is false.
        json: Option<bool>,
        /// Enable (true) or disable (false) dumping of binary packets
        /// as pseudo-NMEA. Default is false.
        nmea: Option<bool>,
        /// Controls 'raw' mode. When this attribute is set to 1 for a
        /// channel, gpsd reports the unprocessed NMEA or AIVDM data
        /// stream from whatever device is attached. Binary GPS
        /// packets are hex-dumped. RTCM2 and RTCM3 packets are not
        /// dumped in raw mode. When this attribute is set to 2 for a
        /// channel that processes binary data, gpsd reports the
        /// received data verbatim without hex-dumping.
        raw: Option<u8>,
        /// If true, apply scaling divisors to output before dumping;
        /// default is false.
        scaled: Option<bool>,
        /// undocumented
        timing: Option<bool>,
        /// If true, aggregate AIS type24 sentence parts. If false,
        /// report each part as a separate JSON object, leaving the
        /// client to match MMSIs and aggregate. Default is false.
        /// Applies only to AIS reports.
        split24: Option<bool>,
        /// If true, emit the TOFF JSON message on each cycle and a
        /// PPS JSON message when the device issues 1PPS. Default is
        /// false.
        pps: Option<bool>,
    },
}

/// Responses from `gpsd` after handshake (i.e. the payload)
#[derive(Debug, Deserialize)]
#[serde(tag = "class")]
pub enum ResponseData {
    /// Device information
    #[serde(rename = "DEVICE")]
    Device {
        /// Name the device for which the control bits are being
        /// reported, or for which they are to be applied. This
        /// attribute may be omitted only when there is exactly one
        /// subscribed channel.
        path: Option<String>,
        /// Time the device was activated as an ISO8601 timestamp. If
        /// the device is inactive this attribute is absent.
        activated: Option<String>,
        /// Bit vector of property flags. Currently defined flags are:
        /// describe packet types seen so far (GPS, RTCM2, RTCM3,
        /// AIS). Won't be reported if empty, e.g. before gpsd has
        /// seen identifiable packets from the device.
        flags: Option<i32>,
        /// GPSD's name for the device driver type. Won't be reported
        /// before gpsd has seen identifiable packets from the device.
        driver: Option<String>,
        /// Whatever version information the device returned.
        subtype: Option<String>,
        /// Device speed in bits per second.
        bps: Option<u16>,
        /// N, O or E for no parity, odd, or even.
        parity: Option<String>,
        /// Stop bits (1 or 2).
        stopbits: Option<u8>,
        /// 0 means NMEA mode and 1 means alternate mode (binary if it
        /// has one, for SiRF and Evermore chipsets in particular).
        /// Attempting to set this mode on a non-GPS device will yield
        /// an error.
        native: Option<u8>,
        /// Device cycle time in seconds.
        cycle: Option<f32>,
        /// Device minimum cycle time in seconds. Reported from
        /// ?DEVICE when (and only when) the rate is switchable. It is
        /// read-only and not settable.
        mincycle: Option<f32>,
    },
    /// GPS position.
    ///
    /// A TPV object is a time-position-velocity report. The "mode"
    /// field will be emitted before optional fields that may be
    /// absent when there is no fix. Error estimates will be emitted
    /// after the fix components they're associated with. Others may
    /// be reported or not depending on the fix quality.
    #[serde(rename = "TPV")]
    Tpv {
        /// Name of the originating device.
        device: Option<String>,
        /// NMEA mode, see `Mode` enum.
        #[serde(deserialize_with = "mode_from_str")]
        mode: Mode,
        /// Time/date stamp in ISO8601 format, UTC. May have a
        /// fractional part of up to .001sec precision. May be absent
        /// if mode is not 2 or 3.
        time: Option<String>,
        /// Estimated timestamp error (%f, seconds, 95% confidence).
        /// Present if time is present.
        ept: Option<f32>,
        /// Latitude in degrees: +/- signifies North/South. Present
        /// when mode is 2 or 3.
        lat: Option<f64>,
        /// Longitude in degrees: +/- signifies East/West. Present
        /// when mode is 2 or 3.
        lon: Option<f64>,
        /// Altitude in meters. Present if mode is 3.
        alt: Option<f32>,
        /// Longitude error estimate in meters, 95% confidence.
        /// Present if mode is 2 or 3 and DOPs can be calculated from
        /// the satellite view.
        epx: Option<f32>,
        /// Latitude error estimate in meters, 95% confidence. Present
        /// if mode is 2 or 3 and DOPs can be calculated from the
        /// satellite view.
        epy: Option<f32>,
        /// Estimated vertical error in meters, 95% confidence.
        /// Present if mode is 3 and DOPs can be calculated from the
        /// satellite view.
        epv: Option<f32>,
        /// Course over ground, degrees from true north.
        track: Option<f32>,
        /// Speed over ground, meters per second.
        speed: Option<f32>,
        /// Climb (positive) or sink (negative) rate, meters per
        /// second.
        climb: Option<f32>,
        /// Direction error estimate in degrees, 95% confidence.
        epd: Option<f32>,
        /// Speed error estinmate in meters/sec, 95% confidence.
        eps: Option<f32>,
        /// Climb/sink error estimate in meters/sec, 95% confidence.
        epc: Option<f32>,
    },
    /// Satellites information.
    ///
    /// A SKY object reports a sky view of the GPS satellite
    /// positions. If there is no GPS device available, or no skyview
    /// has been reported yet.
    ///
    /// Many devices compute dilution of precision factors but do not
    /// include them in their reports. Many that do report DOPs report
    /// only HDOP, two-dimensional circular error. gpsd always passes
    /// through whatever the device actually reports, then attempts to
    /// fill in other DOPs by calculating the appropriate determinants
    /// in a covariance matrix based on the satellite view. DOPs may
    /// be missing if some of these determinants are singular. It can
    /// even happen that the device reports an error estimate in
    /// meters when the corresponding DOP is unavailable; some devices
    /// use more sophisticated error modeling than the covariance
    /// calculation.
    #[serde(rename = "SKY")]
    Sky {
        /// Name of originating device.
        device: Option<String>,
        /// Longitudinal dilution of precision, a dimensionless factor
        /// which should be multiplied by a base UERE to get an error
        /// estimate.
        xdop: Option<f32>,
        /// Latitudinal dilution of precision, a dimensionless factor
        /// which should be multiplied by a base UERE to get an error
        /// estimate.
        ydop: Option<f32>,
        /// Altitude dilution of precision, a dimensionless factor
        /// which should be multiplied by a base UERE to get an error
        /// estimate.
        vdop: Option<f32>,
        /// Time dilution of precision, a dimensionless factor which
        /// should be multiplied by a base UERE to get an error
        /// estimate.
        tdop: Option<f32>,
        /// Horizontal dilution of precision, a dimensionless factor
        /// which should be multiplied by a base UERE to get a
        /// circular error estimate.
        hdop: Option<f32>,
        /// Hyperspherical dilution of precision, a dimensionless
        /// factor which should be multiplied by a base UERE to get an
        /// error estimate.
        gdop: Option<f32>,
        /// Spherical dilution of precision, a dimensionless factor
        /// which should be multiplied by a base UERE to get an error
        /// estimate.
        pdop: Option<f32>,
        /// List of satellite objects in skyview.
        satellites: Vec<Satellite>,
    },
}

/// Type of GPS fix
#[derive(Debug)]
pub enum Mode {
    /// No fix at all.
    NoFix,
    /// Two dimensional fix, 2D.
    Fix2d,
    /// Three dimensional fix, 3D (i.e. with altitude).
    Fix3d,
}

impl ToString for Mode {
    fn to_string(&self) -> String {
        match self {
            Mode::NoFix => String::from("NoFix"),
            Mode::Fix2d => String::from("2d"),
            Mode::Fix3d => String::from("3d"),
        }
    }
}

fn mode_from_str<'de, D>(deserializer: D) -> Result<Mode, D::Error>
where
    D: Deserializer<'de>,
{
    let s = u8::deserialize(deserializer)?;
    match s {
        2 => Ok(Mode::Fix2d),
        3 => Ok(Mode::Fix3d),
        _ => Ok(Mode::NoFix),
    }
}

/// Detailed satellite information.
#[derive(Debug, Deserialize)]
pub struct Satellite {
    /// PRN ID of the satellite. 1-63 are GNSS satellites, 64-96 are
    /// GLONASS satellites, 100-164 are SBAS satellites.
    #[serde(rename = "PRN")]
    pub prn: u16,
    /// Elevation in degrees.
    pub el: u16,
    /// Azimuth, degrees from true north.
    pub az: u16,
    /// Signal strength in dB.
    pub ss: u16,
    /// Used in current solution? (SBAS/WAAS/EGNOS satellites may be
    /// flagged used if the solution has corrections from them, but
    /// not all drivers make this information available.).
    pub used: bool,
}

/// Errors during handshake or data acquisition.
#[derive(Debug)]
pub enum GpsdError {
    /// Generic I/O error.
    IoError(io::Error),
    /// JSON error.
    JsonError(serde_json::Error),
    /// The protocol version reported by `gpsd` is smaller `PROTO_MAJOR_MIN`.
    UnsupportedGpsdProtocolVersion,
    /// Unexpected reply of `gpsd`.
    UnexpectedGpsdReply(String),
    /// Failed to enable watch.
    WatchFail(String),
}

impl From<io::Error> for GpsdError {
    fn from(err: io::Error) -> GpsdError {
        GpsdError::IoError(err)
    }
}

impl From<serde_json::Error> for GpsdError {
    fn from(err: serde_json::Error) -> GpsdError {
        GpsdError::JsonError(err)
    }
}

/// Performs the initial handshake with `gpsd`.
///
/// The following sequence of messages is expected: get VERSION, set
/// WATCH, get DEVICES, get WATCH.
///
/// # Arguments
///
/// * `debug` - enable debug printing of raw JSON data received
/// * `reader` - reader to fetch data from `gpsd`
/// * `writer` - write to send data to `gpsd`
///
/// # Errors
///
/// If the handshake fails, this functions returns an error that
/// indicates the type of error.
pub fn handshake<R>(
    debug: bool,
    reader: &mut io::BufRead,
    writer: &mut io::BufWriter<R>,
) -> Result<(), GpsdError>
where
    R: std::io::Write,
{
    // Get VERSION
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    if debug {
        println!("DEBUG {}", String::from_utf8(data.clone()).unwrap());
    }
    let msg: ResponseHandshake = serde_json::from_slice(&mut data)?;
    match msg {
        ResponseHandshake::Version {
            release: _,
            rev: _,
            proto_major,
            ..
        } => {
            if proto_major < PROTO_MAJOR_MIN {
                return Err(GpsdError::UnsupportedGpsdProtocolVersion);
            }
        }
        _ => {
            return Err(GpsdError::UnexpectedGpsdReply(
                String::from_utf8(data.clone()).unwrap(),
            ))
        }
    }

    // Enable WATCH
    writer.write(ENABLE_WATCH_CMD.as_bytes())?;
    writer.flush()?;

    // Get DEVICES
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    if debug {
        println!("DEBUG {}", String::from_utf8(data.clone()).unwrap());
    }
    let msg: ResponseHandshake = serde_json::from_slice(&mut data)?;
    match msg {
        ResponseHandshake::Devices { devices: _ } => {}
        _ => {
            return Err(GpsdError::UnexpectedGpsdReply(
                String::from_utf8(data.clone()).unwrap(),
            ))
        }
    }

    // Get WATCH
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    if debug {
        println!("DEBUG {}", String::from_utf8(data.clone()).unwrap());
    }
    let msg: ResponseHandshake = serde_json::from_slice(&mut data)?;
    match msg {
        ResponseHandshake::Watch {
            enable, json, nmea, ..
        } => {
            if let (false, false, true) = (
                enable.unwrap_or(false),
                json.unwrap_or(false),
                nmea.unwrap_or(false),
            ) {
                return Err(GpsdError::WatchFail(
                    String::from_utf8(data.clone()).unwrap(),
                ));
            }
        }
        _ => {
            return Err(GpsdError::UnexpectedGpsdReply(
                String::from_utf8(data.clone()).unwrap(),
            ))
        }
    }

    Ok(())
}

/// Get one payload entry from `gpsd`.
///
/// # Arguments
///
/// * `debug` - enable debug printing of raw JSON data received
/// * `reader` - reader to fetch data from `gpsd`
/// * `writer` - write to send data to `gpsd`
pub fn get_data(debug: bool, reader: &mut io::BufRead) -> Result<ResponseData, GpsdError> {
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    if debug {
        println!("DEBUG {}", String::from_utf8(data.clone()).unwrap());
    }
    let msg: ResponseData = serde_json::from_slice(&mut data)?;
    Ok(msg)
}
