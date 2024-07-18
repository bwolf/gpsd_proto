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
//! ## Historic Links
//!
//! The GPSD documentation is only valid for the most recent version,
//! and does not reflect changes from previous versions. The links
//! below are convenience links to wayback machine entries for
//! specific versions of the gpsd_json documentation.
//!
//! - gpsd_json 3.17: https://web.archive.org/web/20171211092731/http://www.catb.org/gpsd/gpsd_json.html
//! - gpsd_json 3.20: https://web.archive.org/web/20200512073259/https://gpsd.gitlab.io/gpsd/gpsd_json.html
//!
//! (some amount of guesswork was required here, based on the tag
//! dates in the gpsd repository. For example 3.17 was released sept
//! 2017, 3.18 was oct 2018, and the wayback link is from between
//! those dates.)
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
//! Test the connection to `gpsd` with `telnet localhost 2947` and send the
//! string:
//!
//! ```text
//! ?WATCH={"enable":true,"json":true};
//! ```

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use std::{fmt, io};

use serde::de::*;
use serde::Deserializer;
#[cfg(feature = "serialize")]
use serde::{Serialize, Serializer};

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
    /// List of devices.
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
    /// device is inactive this attribute is absent. Some older versions
    /// of gpsd will sometimes give the integer 0 in this field, which
    /// this library maps to `None`
    #[serde(default, deserialize_with = "option_str_or_zero")]
    pub activated: Option<String>,
}

// This might look familiar: https://serde.rs/string-or-struct.html
fn option_str_or_zero<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionOrZero;

    impl<'de> Visitor<'de> for OptionOrZero {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("nothing, string or integer 0")
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<String>, E>
        where
            E: Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<String>, E>
        where
            E: Error,
        {
            if value == 0 {
                Ok(None)
            } else {
                Err(Error::invalid_value(Unexpected::Signed(value), &self))
            }
        }
        fn visit_u64<E>(self, value: u64) -> Result<Option<String>, E>
        where
            E: Error,
        {
            if value == 0 {
                Ok(None)
            } else {
                Err(Error::invalid_value(Unexpected::Unsigned(value), &self))
            }
        }
    }
    deserializer.deserialize_any(OptionOrZero)
}

/// Watch response. Elicits a report of per-subscriber policy.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
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
    /// If present, enable watching only of the specified device
    /// rather than all devices. Useful with raw and NMEA modes
    /// in which device responses aren’t tagged. Has no effect
    /// when used with enable:false.
    pub device: Option<String>,
}

/// The POLL command requests data from the last-seen fixes on all active GPS
/// devices. Devices must previously have been activated by ?WATCH to be
/// pollable.

/// Polling can lead to possibly surprising results when it is used on a device
/// such as an NMEA GPS for which a complete fix has to be accumulated from
/// several sentences. If you poll while those sentences are being emitted, the
/// response will contain only the fix data collected so far in the current
/// epoch. It may be as much as one cycle time (typically 1 second) stale.

/// The POLL response will contain a timestamped list of TPV objects describing
/// cached data, and a timestamped list of SKY objects describing satellite
/// configuration. If a device has not seen fixes, it will be reported with a
/// mode field of zero.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
pub struct Poll {
    /// Timestamp in ISO8601 format, UTC. May have a fractional part
    /// of up to .001sec precision.
    pub time: Option<String>,
    /// Count of active devices.
    pub active: u32,
    /// List of TPV Objects
    pub tpv: Vec<Tpv>,
    /// List of SKY Objects
    pub sky: Vec<Sky>,
}

/// Responses from `gpsd` during handshake..
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[serde(tag = "class")]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ResponseHandshake {
    Version(Version),
    Devices(Devices),
    Watch(Watch),
}

/// Device information.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
pub struct Device {
    /// Name the device for which the control bits are being
    /// reported, or for which they are to be applied. This
    /// attribute may be omitted only when there is exactly one
    /// subscribed channel.
    pub path: Option<String>,
    /// Time the device was activated as an ISO8601 timestamp. If
    /// the device is inactive this attribute is absent. Some
    /// older versions of gpsd will sometimes give the integer 0
    /// in this field, which this library maps to `None`
    #[serde(default, deserialize_with = "option_str_or_zero")]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// Altitude height above ellipsoid (elipsoid is unspecified, but probably
    /// WGS48)
    #[serde(rename = "altHAE")]
    pub alt_hae: Option<f32>,
    /// Geoid separation between whatever geoid the device uses and WGS84, in
    /// metres
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
    /// Current Datum. Hopefully WGS84.
    pub datum: Option<String>,
    /// Depth in meters.
    pub depth: Option<f32>,
    /// Age of DGPS Data in seconds
    #[serde(rename = "dgpsAge")]
    pub dgps_age: Option<f32>,
    /// ID of DGPS station
    #[serde(rename = "dgpsSta")]
    pub dgps_sta: Option<i32>,
    /// Course over ground, degrees magnetic.
    pub magtrack: Option<f32>,
    /// Magnetic variation, degrees. Also known as the magnetic
    /// declination (the direction of the horizontal component
    /// of the magnetic field measured clockwise from north)
    /// in degrees, Positive is West variation. Negative is
    /// East variation.
    pub magvar: Option<f32>,
    /// ECEF X Position in meters.
    pub ecefx: Option<f32>,
    /// ECEF Y Position in meters.
    pub ecefy: Option<f32>,
    /// ECEF Z Position in meters.
    pub ecefz: Option<f32>,
    /// ECEF Position error in meters.
    #[serde(rename = "ecefpAcc")]
    pub ecef_p_acc: Option<f32>,
    /// ECEF X velocity in meters per second.
    pub ecefvx: Option<f32>,
    /// ECEF Y velocity in meters per second.
    pub ecefvy: Option<f32>,
    /// ECEF Z velocity in meters per second.
    pub ecefvz: Option<f32>,
    /// ECEF velocity error in meters per second.
    #[serde(rename = "ecefvAcc")]
    pub ecef_v_acc: Option<f32>,
    /// Estimated Spherical (3D) Position Error in meters.
    pub sep: Option<f32>,
    /// Down component of relative position vector in meters.
    #[serde(rename = "relD")]
    pub rel_d: Option<f32>,
    /// East component of relative position vector in meters.
    #[serde(rename = "relE")]
    pub rel_e: Option<f32>,
    /// North component of relative position vector in meters.
    #[serde(rename = "relN")]
    pub rel_n: Option<f32>,
    /// Down velocity component in meters.
    #[serde(rename = "velD")]
    pub vel_d: Option<f32>,
    /// East velocity component in meters.
    #[serde(rename = "velE")]
    pub vel_e: Option<f32>,
    /// North velocity component in meters.
    #[serde(rename = "velN")]
    pub vel_n: Option<f32>,
    /// Wind angle magnetic in degrees.
    pub wanglem: Option<f32>,
    /// Wind angle relative in degrees.
    pub wangler: Option<f32>,
    /// Wind angle true in degrees.
    pub wanglet: Option<f32>,
    /// Wind speed relative in meters per second.
    pub wspeedr: Option<f32>,
    /// Wind speed true in meters per second.
    pub wspeedt: Option<f32>,
    /// Water temperature in degrees Celsius.
    pub wtemp: Option<f32>,
}

/// Detailed satellite information.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
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
    /// The GNSS ID, as defined by u-blox, not NMEA. 0=GPS, 2=Galileo,
    /// 3=Beidou, 5=QZSS, 6-GLONASS.
    pub gnssid: Option<u8>,
    /// The satellite ID within its constellation. As defined by
    /// u-blox, not NMEA).
    pub svid: Option<u16>,
    /// The signal ID of this signal. As defined by u-blox,
    /// not NMEA. See u-blox doc for details.
    pub sigid: Option<u16>,
    /// For GLONASS satellites only: the frequency ID of the
    /// signal. As defined by u-blox, range 0 to 13. The freqid
    /// is the frequency slot plus 7.
    pub freqid: Option<u16>,
    /// The health of this satellite. 0 is unknown, 1 is OK, and 2 is unhealthy.
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
#[non_exhaustive]
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
    /// Number of satellites in "satellites" array
    #[serde(rename = "nSat")]
    pub n_sat: Option<u32>,
    /// Pseudorange Residue in meters.
    #[serde(rename = "prRes")]
    pub pr_res: Option<f32>,
    /// Quality indicator
    pub qual: Option<u8>,
    /// Time/date stamp in ISO8601 format, UTC. May have a
    /// fractional part of up to .001sec precision.
    pub time: Option<String>,
    /// Number of satellites used in navigation solution.
    #[serde(rename = "uSat")]
    pub u_sat: Option<u32>,
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
#[non_exhaustive]
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
    pub precision: Option<f32>,
    /// shm key of this PPS
    pub shm: Option<String>,
    /// Quantization error of the pps, in picoseconds. Sometimes called the
    /// "sawtooth" error
    #[serde(rename = "qErr")]
    pub q_err: Option<f32>,
}

/// Pseudorange noise report.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
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

/// An ATT object is a vehicle-attitude report. It is returned by
/// digital-compass and gyroscope sensors; depending on device, it may include:
/// heading, pitch, roll, yaw, gyroscope, and magnetic-field readings. Because
/// such sensors are often bundled as part of marine-navigation systems, the ATT
/// response may also include water depth.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
pub struct Att {
    /// Name of originating device.
    pub device: Option<String>,
    /// Time/date stamp in ISO8601 format, UTC. May have a fractional part of up
    /// to .001 sec precision.
    pub time: Option<String>,
    /// Arbitrary time tag of measurement
    #[serde(rename = "timeTag")]
    pub time_tag: Option<String>,
    /// Heading, degrees from true north.
    pub heading: Option<f32>,
    /// Magnetometer status
    pub mag_st: Option<String>,
    /// Heading, degrees from magnetic north.
    pub mheading: Option<f32>,
    /// Pitch, in degrees.
    pub pitch: Option<f32>,
    /// Pitch sensor status
    pub pitch_st: Option<String>,
    /// Rate of turn in degrees per minute.
    pub rot: Option<f32>,
    /// Yaw, in degrees.
    pub yaw: Option<f32>,
    /// Yaw sensor status
    pub yaw_st: Option<String>,
    /// Roll, in degrees.
    pub roll: Option<f32>,
    /// Roll sensor status
    pub roll_st: Option<String>,
    /// Local magnetic inclination, degrees, positive when the magnetic field
    /// points downward (into the Earth).
    pub dip: Option<f32>,
    /// Scalar magnetic field strength.
    pub mag_len: Option<f32>,
    /// X component of magnetic field strength.
    pub mag_x: Option<f32>,
    /// Y component of magnetic field strength.
    pub mag_y: Option<f32>,
    /// Z component of magnetic field strength.
    pub mag_z: Option<f32>,
    /// Scalar acceleration
    pub acc_len: Option<f32>,
    /// X component of acceleration (m/s^2)
    pub acc_x: Option<f32>,
    /// Y component of acceleration
    pub acc_y: Option<f32>,
    /// Z component of acceleration
    pub acc_z: Option<f32>,
    /// X component of angular rate (deg/s)
    pub gyro_x: Option<f32>,
    /// Y component of angular rate
    pub gyro_y: Option<f32>,
    /// Z component of angular rate
    pub gyro_z: Option<f32>,
    /// Water depth, in meters.
    pub depth: Option<f32>,
    /// Temperature at the sensor, degrees centigrade.
    pub temp: Option<f32>,
}

/// This message reports the status of a GPS-disciplined oscillator (GPSDO).
/// The GPS PPS output (which has excellent long-term stability) is
/// typically used to discipline a local oscillator with much better
/// short-term stability (such as a rubidium atomic clock).
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[non_exhaustive]
pub struct Osc {
    /// Name of originating device.
    pub device: Option<String>,
    /// If true, the oscillator is currently running.
    pub running: bool,
    /// If true, the oscillator is receiving a GPS PPS Signal
    pub reference: bool,
    /// If true, the GPS PPS signal is sufficiently stable and is being
    /// used to discipline the local oscillator.
    pub disciplined: bool,
    /// The time difference (in nanoseconds) between the GPS-disciplined
    /// oscillator PPS output pulse and the most recent GPS PPS input pulse.
    pub delta: u32,
}

/// Responses from `gpsd` after handshake (i.e. the payload)
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[serde(tag = "class")]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ResponseData {
    Device(Device),
    Tpv(Tpv),
    Sky(Sky),
    Pps(Pps),
    Gst(Gst),
    Att(Att),
    /// The IMU object is asynchronous to the GNSS epoch. It is
    /// reported with arbitrary, even out of order, time scales.
    /// The ATT and IMU objects have the same fields, but IMU
    /// objects are output as soon as possible.
    Imu(Att),
    /// This message is emitted on each cycle and reports the
    /// offset between the host’s clock time and the GPS time
    /// at top of the second (actually, when the first data
    /// for the reporting cycle is received).
    ///
    /// This message exactly mirrors the PPS message.
    ///
    /// The TOFF message reports the GPS time as derived from
    /// the GPS serial data stream. The PPS message reports
    /// the GPS time as derived from the GPS PPS pulse.
    Toff(Pps),
    Osc(Osc),
    Poll(Poll),
}

/// All known `gpsd` responses (handshake + normal operation).
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[serde(tag = "class")]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum UnifiedResponse {
    Version(Version),
    Devices(Devices),
    Watch(Watch),
    Device(Device),
    Tpv(Tpv),
    Sky(Sky),
    Pps(Pps),
    Gst(Gst),
    Att(Att),
    /// The IMU object is asynchronous to the GNSS epoch. It is
    /// reported with arbitrary, even out of order, time scales.
    /// The ATT and IMU objects have the same fields, but IMU
    /// objects are output as soon as possible.
    Imu(Att),
    /// This message is emitted on each cycle and reports the
    /// offset between the host’s clock time and the GPS time
    /// at top of the second (actually, when the first data
    /// for the reporting cycle is received).
    ///
    /// This message exactly mirrors the PPS message.
    ///
    /// The TOFF message reports the GPS time as derived from
    /// the GPS serial data stream. The PPS message reports
    /// the GPS time as derived from the GPS PPS pulse.
    Toff(Pps),
    Osc(Osc),
    Poll(Poll),
    /// The SUBFRAME message is essentially arbitrary data which can vary based on your choice of GPS
    Subframe(serde_json::Value),
}

/// Errors during handshake or data acquisition.
#[derive(Debug)]
#[non_exhaustive]
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
                return Err(GpsdError::WatchFail(String::from_utf8(data).unwrap()));
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
    use std::io::BufWriter;

    use super::{
        get_data, handshake, GpsdError, Mode, ResponseData, UnifiedResponse, ENABLE_WATCH_CMD,
    };

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
                assert!(matches!(tpv.mode, Mode::Fix3d));
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
                assert!(actual.used);
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

    fn unwrap_device(data: UnifiedResponse) -> crate::Devices {
        match data {
            UnifiedResponse::Devices(d) => d,
            _ => panic!("Unexpected response"),
        }
    }

    #[test]
    fn test_device_activated_zero_value() {
        let data: &[u8] =
            b"{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\",\"activated\":0}]}
{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\",\"activated\":\"2024-01-10T11:36:48.480Z\"}]}
{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\"}]}
{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\",\"activated\":1}]}
{\"class\":\"DEVICES\",\"devices\":[{\"path\":\"/dev/gps\",\"activated\":false}]}";
        let mut rdr = data.split(|b| *b == b'\n');

        let ok_zero = unwrap_device(serde_json::from_slice(rdr.next().unwrap()).unwrap());
        assert_eq!(ok_zero.devices[0].activated, None);

        let ok_timestamp = unwrap_device(serde_json::from_reader(rdr.next().unwrap()).unwrap());
        assert_eq!(
            ok_timestamp.devices[0].activated,
            Some("2024-01-10T11:36:48.480Z".to_string())
        );

        let ok_not_present = unwrap_device(serde_json::from_reader(rdr.next().unwrap()).unwrap());
        assert_eq!(ok_not_present.devices[0].activated, None);

        assert!(serde_json::from_reader::<_, UnifiedResponse>(rdr.next().unwrap()).is_err());

        assert!(serde_json::from_reader::<_, UnifiedResponse>(rdr.next().unwrap()).is_err());
    }
}
