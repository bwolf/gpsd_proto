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

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use serde::de::*;
use serde::Deserializer;
#[cfg(feature = "serialize")]
use serde::{Serialize, Serializer};

use std::fmt;
use std::io;

/// Minimum supported version of `gpsd`.
pub const PROTO_MAJOR_MIN: u8 = 3;

/// Command to enable watch.
pub const ENABLE_WATCH_CMD: &str = "?WATCH={\"enable\":true,\"json\":true};\r\n";

/// `gpsd` ships a VERSION response to each client when the client
/// first connects to it.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Version {
    /// Public release level.
    pub release: String,
    /// Internal revision-control level.
    pub rev: String,
    /// API major revision level.
    pub proto_major: u8,
    /// API minor revision level.
    pub proto_minor: u8,
    /// URL of the remote daemon reporting this version. If empty,
    /// this is the version of the local daemon.
    pub remote: Option<String>,
}

/// Device information (i.e. device enumeration).
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Devices {
    pub devices: Vec<DeviceInfo>,
}

/// Single device information as reported by `gpsd`.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct DeviceInfo {
    /// Name the device for which the control bits are being reported,
    /// or for which they are to be applied. This attribute may be
    /// omitted only when there is exactly one subscribed channel.
    pub path: Option<String>,
    /// Time the device was activated as an ISO8601 timestamp. If the
    /// device is inactive this attribute is absent.
    pub activated: Option<String>,
}

/// Watch response. Elicits a report of per-subscriber policy.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Watch {
    /// Enable (true) or disable (false) watcher mode. Default is
    /// true.
    pub enable: Option<bool>,
    /// Enable (true) or disable (false) dumping of JSON reports.
    /// Default is false.
    pub json: Option<bool>,
    /// Enable (true) or disable (false) dumping of binary packets
    /// as pseudo-NMEA. Default is false.
    pub nmea: Option<bool>,
    /// Controls 'raw' mode. When this attribute is set to 1 for a
    /// channel, gpsd reports the unprocessed NMEA or AIVDM data
    /// stream from whatever device is attached. Binary GPS
    /// packets are hex-dumped. RTCM2 and RTCM3 packets are not
    /// dumped in raw mode. When this attribute is set to 2 for a
    /// channel that processes binary data, gpsd reports the
    /// received data verbatim without hex-dumping.
    pub raw: Option<u8>,
    /// If true, apply scaling divisors to output before dumping;
    /// default is false.
    pub scaled: Option<bool>,
    /// undocumented
    pub timing: Option<bool>,
    /// If true, aggregate AIS type24 sentence parts. If false,
    /// report each part as a separate JSON object, leaving the
    /// client to match MMSIs and aggregate. Default is false.
    /// Applies only to AIS reports.
    pub split24: Option<bool>,
    /// If true, emit the TOFF JSON message on each cycle and a
    /// PPS JSON message when the device issues 1PPS. Default is
    /// false.
    pub pps: Option<bool>,
}

/// Responses from `gpsd` during handshake..
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[serde(tag = "class")]
#[serde(rename_all = "UPPERCASE")]
pub enum ResponseHandshake {
    Version(Version),
    Devices(Devices),
    Watch(Watch),
}

/// Device information.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Device {
    /// Name the device for which the control bits are being
    /// reported, or for which they are to be applied. This
    /// attribute may be omitted only when there is exactly one
    /// subscribed channel.
    pub path: Option<String>,
    /// Time the device was activated as an ISO8601 timestamp. If
    /// the device is inactive this attribute is absent.
    pub activated: Option<String>,
    /// Bit vector of property flags. Currently defined flags are:
    /// describe packet types seen so far (GPS, RTCM2, RTCM3,
    /// AIS). Won't be reported if empty, e.g. before gpsd has
    /// seen identifiable packets from the device.
    pub flags: Option<i32>,
    /// GPSD's name for the device driver type. Won't be reported
    /// before gpsd has seen identifiable packets from the device.
    pub driver: Option<String>,
    /// Whatever version information the device returned.
    pub subtype: Option<String>,
    /// Device speed in bits per second.
    pub bps: Option<u16>,
    /// N, O or E for no parity, odd, or even.
    pub parity: Option<String>,
    /// Stop bits (1 or 2).
    pub stopbits: Option<u8>,
    /// 0 means NMEA mode and 1 means alternate mode (binary if it
    /// has one, for SiRF and Evermore chipsets in particular).
    /// Attempting to set this mode on a non-GPS device will yield
    /// an error.
    pub native: Option<u8>,
    /// Device cycle time in seconds.
    pub cycle: Option<f32>,
    /// Device minimum cycle time in seconds. Reported from
    /// ?DEVICE when (and only when) the rate is switchable. It is
    /// read-only and not settable.
    pub mincycle: Option<f32>,
}

/// Type of GPS fix.
#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// No fix at all.
    NoFix,
    /// Two dimensional fix, 2D.
    Fix2d,
    /// Three dimensional fix, 3D (i.e. with altitude).
    Fix3d,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::NoFix => write!(f, "NoFix"),
            Mode::Fix2d => write!(f, "2d"),
            Mode::Fix3d => write!(f, "3d"),
        }
    }
}

#[cfg(feature = "serialize")]
impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Mode::NoFix => serializer.serialize_i32(1),
            Mode::Fix2d => serializer.serialize_i32(2),
            Mode::Fix3d => serializer.serialize_i32(3),
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

/// GPS position.
///
/// A TPV object is a time-position-velocity report. The "mode"
/// field will be emitted before optional fields that may be
/// absent when there is no fix. Error estimates will be emitted
/// after the fix components they're associated with. Others may
/// be reported or not depending on the fix quality.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Tpv {
    /// Name of the originating device.
    pub device: Option<String>,
    /// GPS fix status.
    pub status: Option<i32>,
    /// NMEA mode, see `Mode` enum.
    #[serde(deserialize_with = "mode_from_str")]
    pub mode: Mode,
    /// Time/date stamp in ISO8601 format, UTC. May have a
    /// fractional part of up to .001sec precision. May be absent
    /// if mode is not 2 or 3.
    pub time: Option<String>,
    /// Estimated timestamp error (%f, seconds, 95% confidence).
    /// Present if time is present.
    pub ept: Option<f32>,
    pub leapseconds: Option<i32>,
    /// MSL altitude in meters.
    #[serde(rename = "altMSL")]
    pub alt_msl: Option<f32>,
    /// Altitude height above ellipsoid (elipsoid is unspecified, but probably WGS48)
    #[serde(rename = "altHAE")]
    pub alt_hae: Option<f32>,
    /// Geoid separation between whatever geoid the device uses and WGS84, in metres
    #[serde(rename = "geoidSep")]
    pub geoid_sep: Option<f32>,
    /// Latitude in degrees: +/- signifies North/South. Present
    /// when mode is 2 or 3.
    pub lat: Option<f64>,
    /// Longitude in degrees: +/- signifies East/West. Present
    /// when mode is 2 or 3.
    pub lon: Option<f64>,
    /// Altitude in meters. Present if mode is 3.
    pub alt: Option<f32>,
    /// Longitude error estimate in meters, 95% confidence.
    /// Present if mode is 2 or 3 and DOPs can be calculated from
    /// the satellite view.
    pub epx: Option<f32>,
    /// Latitude error estimate in meters, 95% confidence. Present
    /// if mode is 2 or 3 and DOPs can be calculated from the
    /// satellite view.
    pub epy: Option<f32>,
    /// Estimated vertical error in meters, 95% confidence.
    /// Present if mode is 3 and DOPs can be calculated from the
    /// satellite view.
    pub epv: Option<f32>,
    /// Course over ground, degrees from true north.
    pub track: Option<f32>,
    /// Speed over ground, meters per second.
    pub speed: Option<f32>,
    /// Climb (positive) or sink (negative) rate, meters per
    /// second.
    pub climb: Option<f32>,
    /// Direction error estimate in degrees, 95% confidence.
    pub epd: Option<f32>,
    /// Speed error estinmate in meters/sec, 95% confidence.
    pub eps: Option<f32>,
    /// Climb/sink error estimate in meters/sec, 95% confidence.
    pub epc: Option<f32>,
    /// Horizontal 2D position error in meters.
    pub eph: Option<f32>,
}

/// Detailed satellite information.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Satellite {
    /// PRN ID of the satellite. 1-63 are GNSS satellites, 64-96 are
    /// GLONASS satellites, 100-164 are SBAS satellites.
    #[serde(rename = "PRN")]
    pub prn: i16,
    /// Elevation in degrees.
    pub el: Option<f32>,
    /// Azimuth, degrees from true north.
    pub az: Option<f32>,
    /// Signal strength in dB.
    pub ss: Option<f32>,
    /// Used in current solution? (SBAS/WAAS/EGNOS satellites may be
    /// flagged used if the solution has corrections from them, but
    /// not all drivers make this information available.).
    pub used: bool,
    pub gnssid: Option<u8>,
    pub svid: Option<u16>,
    pub health: Option<u8>,
}

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
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Sky {
    /// Name of originating device.
    pub device: Option<String>,
    /// Longitudinal dilution of precision, a dimensionless factor
    /// which should be multiplied by a base UERE to get an error
    /// estimate.
    pub xdop: Option<f32>,
    /// Latitudinal dilution of precision, a dimensionless factor
    /// which should be multiplied by a base UERE to get an error
    /// estimate.
    pub ydop: Option<f32>,
    /// Altitude dilution of precision, a dimensionless factor
    /// which should be multiplied by a base UERE to get an error
    /// estimate.
    pub vdop: Option<f32>,
    /// Time dilution of precision, a dimensionless factor which
    /// should be multiplied by a base UERE to get an error
    /// estimate.
    pub tdop: Option<f32>,
    /// Horizontal dilution of precision, a dimensionless factor
    /// which should be multiplied by a base UERE to get a
    /// circular error estimate.
    pub hdop: Option<f32>,
    /// Hyperspherical dilution of precision, a dimensionless
    /// factor which should be multiplied by a base UERE to get an
    /// error estimate.
    pub gdop: Option<f32>,
    /// Spherical dilution of precision, a dimensionless factor
    /// which should be multiplied by a base UERE to get an error
    /// estimate.
    pub pdop: Option<f32>,
    /// List of satellite objects in skyview.
    pub satellites: Option<Vec<Satellite>>,
}

/// This message is emitted each time the daemon sees a valid PPS (Pulse Per
/// Second) strobe from a device.
///
/// This message exactly mirrors the TOFF message except for two details.
///
/// PPS emits the NTP precision. See the NTP documentation for their definition
/// of precision.
///
/// The TOFF message reports the GPS time as derived from the GPS serial data
/// stream. The PPS message reports the GPS time as derived from the GPS PPS
/// pulse.
///
/// There are various sources of error in the reported clock times. The speed of
/// the serial connection between the GPS and the system adds a delay to start
/// of cycle detection. An even bigger error is added by the variable
/// computation time inside the GPS. Taken together the time derived from the
/// start of the GPS cycle can have offsets of 10 millisecond to 700
/// milliseconds and combined jitter and wander of 100 to 300 millisecond.
///
/// This message is emitted once per second to watchers of a device emitting
/// PPS, and reports the time of the start of the GPS second (when the 1PPS
/// arrives) and seconds as reported by the system clock (which may be
/// NTP-corrected) at that moment.
///
/// The message contains two second/nanosecond pairs: real_sec and real_nsec
/// contain the time the GPS thinks it was at the PPS edge; clock_sec and
/// clock_nsec contain the time the system clock thinks it was at the PPS edge.
/// real_nsec is always to nanosecond precision. clock_nsec is nanosecond
/// precision on most systems.
///
/// There are various sources of error in the reported clock times. For PPS
/// delivered via a real serial-line strobe, serial-interrupt latency plus
/// processing time to the timer call should be bounded above by about 10
/// microseconds; that can be reduced to less than 1 microsecond if your kernel
/// supports RFC 2783. USB1.1-to-serial control-line emulation is limited to
/// about 1 millisecond.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Pps {
    /// Name of originating device.
    pub device: String,
    /// Seconds from the PPS source.
    pub real_sec: f32,
    /// Nanoseconds from the PPS source.
    pub real_nsec: f32,
    /// Seconds from the system clock.
    pub clock_sec: f32,
    /// Nanoseconds from the system clock.
    pub clock_nsec: f32,
    /// NTP style estimate of PPS precision.
    pub precision: f32,
}

/// Pseudorange noise report.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Gst {
    /// Name of originating device.
    pub device: Option<String>,
    /// Time/date stamp in ISO8601 format, UTC. May have a fractional part of up
    /// to .001 sec precision.
    pub time: Option<String>,
    /// Value of the standard deviation of the range inputs to the navigation
    /// process (range inputs include pseudoranges and DGPS corrections).
    pub rms: Option<f32>,
    /// Standard deviation of semi-major axis of error ellipse, in meters.
    pub major: Option<f32>,
    /// Standard deviation of semi-minor axis of error ellipse, in meters.
    pub minor: Option<f32>,
    /// Orientation of semi-major axis of error ellipse, in degrees from true
    /// north.
    pub orient: Option<f32>,
    /// Standard deviation of latitude error, in meters.
    pub lat: Option<f32>,
    /// Standard deviation of longitude error, in meters.
    pub lon: Option<f32>,
    /// Standard deviation of altitude error, in meters.
    pub alt: Option<f32>,
}

/// Responses from `gpsd` after handshake (i.e. the payload)
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[serde(tag = "class")]
#[serde(rename_all = "UPPERCASE")]
pub enum ResponseData {
    Device(Device),
    Tpv(Tpv),
    Sky(Sky),
    Pps(Pps),
    Gst(Gst),
}

/// All known `gpsd` responses (handshake + normal operation).
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[serde(tag = "class")]
#[serde(rename_all = "UPPERCASE")]
pub enum UnifiedResponse {
    Version(Version),
    Devices(Devices),
    Watch(Watch),
    Device(Device),
    Tpv(Tpv),
    Sky(Sky),
    Pps(Pps),
    Gst(Gst),
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

impl fmt::Display for GpsdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GpsdError::IoError(e) => write!(f, "IoError: {}", e),
            GpsdError::JsonError(e) => write!(f, "JsonError: {}", e),
            GpsdError::UnsupportedGpsdProtocolVersion => {
                write!(f, "UnsupportedGpsdProtocolVersion")
            }
            GpsdError::UnexpectedGpsdReply(e) => write!(f, "UnexpectedGpsdReply: {}", e),
            GpsdError::WatchFail(e) => write!(f, "WatchFail: {}", e),
        }
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
pub fn handshake(
    reader: &mut dyn io::BufRead,
    writer: &mut dyn io::Write,
) -> Result<(), GpsdError> {
    // Get VERSION
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    trace!("{}", String::from_utf8(data.clone()).unwrap());
    let msg: ResponseHandshake = serde_json::from_slice(&data)?;
    match msg {
        ResponseHandshake::Version(v) => {
            if v.proto_major < PROTO_MAJOR_MIN {
                return Err(GpsdError::UnsupportedGpsdProtocolVersion);
            }
        }
        _ => {
            return Err(GpsdError::UnexpectedGpsdReply(
                String::from_utf8(data).unwrap(),
            ))
        }
    }

    // Enable WATCH
    writer.write_all(ENABLE_WATCH_CMD.as_bytes())?;
    writer.flush()?;

    // Get DEVICES
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    trace!("{}", String::from_utf8(data.clone()).unwrap());
    let msg: ResponseHandshake = serde_json::from_slice(&data)?;
    match msg {
        ResponseHandshake::Devices(_) => {}
        _ => {
            return Err(GpsdError::UnexpectedGpsdReply(
                String::from_utf8(data).unwrap(),
            ))
        }
    }

    // Get WATCH
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    trace!("{}", String::from_utf8(data.clone()).unwrap());
    let msg: ResponseHandshake = serde_json::from_slice(&data)?;
    match msg {
        ResponseHandshake::Watch(w) => {
            if let (false, false, true) = (
                w.enable.unwrap_or(false),
                w.json.unwrap_or(false),
                w.nmea.unwrap_or(false),
            ) {
                return Err(GpsdError::WatchFail(
                    String::from_utf8(data).unwrap(),
                ));
            }
        }
        _ => {
            return Err(GpsdError::UnexpectedGpsdReply(
                String::from_utf8(data).unwrap(),
            ))
        }
    }

    Ok(())
}

/// Get one payload entry from `gpsd`.
///
/// # Arguments
///
/// * `reader` - reader to fetch data from `gpsd`
/// * `writer` - write to send data to `gpsd`
pub fn get_data(reader: &mut dyn io::BufRead) -> Result<ResponseData, GpsdError> {
    let mut data = Vec::new();
    reader.read_until(b'\n', &mut data)?;
    trace!("{}", String::from_utf8(data.clone()).unwrap());
    let msg: ResponseData = serde_json::from_slice(&data)?;
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::{get_data, handshake, GpsdError, Mode, ResponseData, ENABLE_WATCH_CMD};
    use std::io::BufWriter;

    #[test]
    fn handshake_ok() {
        // Note: linefeeds (0x0a) are added implicit; each line ends with 0x0d 0x0a.
        let mut reader: &[u8] = b"{\"class\":\"VERSION\",\"release\":\"blah\",\"rev\":\"blurp\",\"proto_major\":3,\"proto_minor\":12}\x0d
{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\",\"activated\":\"true\"}]}
{\"class\":\"WATCH\",\"enable\":true,\"json\":true,\"nmea\":false}
";
        let mut writer = BufWriter::new(Vec::<u8>::new());
        let r = handshake(&mut reader, &mut writer);
        assert!(r.is_ok());
        assert_eq!(writer.get_mut().as_slice(), ENABLE_WATCH_CMD.as_bytes());
    }

    #[test]
    fn handshake_unsupported_protocol_version() {
        let mut reader: &[u8] = b"{\"class\":\"VERSION\",\"release\":\"blah\",\"rev\":\"blurp\",\"proto_major\":2,\"proto_minor\":17}\x0d
";
        let mut writer = BufWriter::new(Vec::<u8>::new());
        let err = match handshake(&mut reader, &mut writer) {
            Err(GpsdError::UnsupportedGpsdProtocolVersion) => Ok(()),
            _ => Err(()),
        };
        assert_eq!(err, Ok(()));
        let empty: &[u8] = &[];
        assert_eq!(writer.get_mut().as_slice(), empty);
    }

    #[test]
    fn handshake_unexpected_gpsd_reply() {
        // A possible response, but in the wrong order; At the begin
        // of the handshake, a VERSION reply is expected.
        let mut reader: &[u8] =
            b"{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\",\"activated\":\"true\"}]}
";
        let mut writer = BufWriter::new(Vec::<u8>::new());
        let err = match handshake(&mut reader, &mut writer) {
            Err(GpsdError::UnexpectedGpsdReply(_)) => Ok(()),
            _ => Err(()),
        };
        assert_eq!(err, Ok(()));
        let empty: &[u8] = &[];
        assert_eq!(writer.get_mut().as_slice(), empty);
    }

    #[test]
    fn handshake_json_error() {
        let mut reader: &[u8] = b"{\"class\":broken";
        let mut writer = BufWriter::new(Vec::<u8>::new());
        let err = match handshake(&mut reader, &mut writer) {
            Err(GpsdError::JsonError(_)) => Ok(()),
            _ => Err(()),
        };
        assert_eq!(err, Ok(()));
        let empty: &[u8] = &[];
        assert_eq!(writer.get_mut().as_slice(), empty);
    }

    #[test]
    fn get_data_tpv() {
        let mut reader: &[u8] = b"{\"class\":\"TPV\",\"mode\":3,\"lat\":66.123}\x0d\x0a";
        let r = get_data(&mut reader).unwrap();
        let test = match r {
            ResponseData::Tpv(tpv) => {
                assert!(match tpv.mode {
                    Mode::Fix3d => true,
                    _ => false,
                });
                assert_eq!(tpv.lat.unwrap(), 66.123);
                Ok(())
            }
            _ => Err(()),
        };
        assert_eq!(test, Ok(()));
    }

    #[test]
    fn get_data_sky() {
        let mut reader: &[u8] = b"{\"class\":\"SKY\",\"device\":\"aDevice\",\"satellites\":[{\"PRN\":123,\"el\":1.0,\"az\":2.0,\"ss\":3.0,\"used\":true,\"gnssid\":1,\"svid\":271,\"health\":1}]}\x0d\x0a";

        let r = get_data(&mut reader).unwrap();
        let test = match r {
            ResponseData::Sky(sky) => {
                assert_eq!(sky.device.unwrap(), "aDevice");
                let actual = &sky.satellites.unwrap()[0];
                assert_eq!(actual.prn, 123);
                assert_eq!(actual.el, Some(1.));
                assert_eq!(actual.az, Some(2.));
                assert_eq!(actual.ss, Some(3.));
                assert_eq!(actual.used, true);
                assert_eq!(actual.gnssid, Some(1));
                assert_eq!(actual.svid, Some(271));
                assert_eq!(actual.health, Some(1));
                Ok(())
            }
            _ => Err(()),
        };
        assert_eq!(test, Ok(()));
    }

    #[test]
    fn mode_to_string() {
        assert_eq!("NoFix", Mode::NoFix.to_string());
        assert_eq!("2d", Mode::Fix2d.to_string());
        assert_eq!("3d", Mode::Fix3d.to_string());
    }
}
