# gpsd_proto &emsp; [![Build Status]][travis] [![Latest Version]][crates.io] [![Latest Docs]][docs.rs] [![Coverage Status]][codecov.io]

[Build Status]: https://travis-ci.org/bwolf/gpsd_proto.svg?branch=master
[travis]: https://travis-ci.org/bwolf/gpsd_proto
[Latest Version]: https://meritbadge.herokuapp.com/gpsd_proto
[crates.io]: https://crates.io/crates/gpsd_proto
[Latest Docs]: https://docs.rs/gpsd_proto/badge.svg
[docs.rs]: https://docs.rs/gpsd_proto/
[Coverage Status]: https://codecov.io/gh/bwolf/gpsd_proto/branch/master/graph/badge.svg
[codecov.io]: https://codecov.io/gh/bwolf/gpsd_proto

<!--- Module documentation of src/lib.rs follows --->

The `gpsd_proto` module contains types and functions to connect to
[gpsd](http://catb.org/gpsd/) to get GPS coordinates and satellite
information.

`gpsd_proto` uses a plain TCP socket to connect to `gpsd`, reads and writes JSON messages. The main motivation to create this crate was independence from C libraries, like `libgps` (provided by `gpsd`) to ease cross compiling.

See the `examples` subdirectory for runnable demos.

# Testing

`gpsd_proto` has been tested against `gpsd` version 3.17 on macOS and Linux with these devices:

- [Adafruit Ultimate GPS Breakout - PA1616S](https://www.adafruit.com/product/746)
- [Quectel EC25 Mini](https://www.quectel.com/product/ec25minipcie.htm) PCIe 4G/LTE Module
- [u-blox MAX-M8Q](https://www.u-blox.com/en/product/max-m8-series)
- GPS mice (Adopt SkyTraQ Venus 8)
- iOS app [GPS2IP](http://www.capsicumdreams.com/iphone/gps2ip/).
- Android app [GPSD Relay](https://github.com/project-kaat/gpsdRelay)
- ... and many more :)

Note regarding the mobile apps: These apps typically provide the raw GPS data (NMEA), which gpsd can interpret. The setup requires running the app on the mobile and to run gpsd on the PC to interpret that data, and to provide it to clients like gpsd_proto.

``` text
  +--------------+   TCP   +--------+   TCP   +-------------------+
  |  gpsd_proto  | ------> |  gpsd  | ------> |  GPS relay (App)  |
  +--------------+         +--------+         +-------------------+
                               |  RS232/USB   +-------------------+
                               `------------> |    Hardware GPS   |
                                              +-------------------+
```

In the figure above gpsd_proto could be an application that uses gpsd_proto as a library to connect to gpsd, or it may be an example from this project like `examples/async.rs`.


# Reference documentation

Important reference documentation of `gpsd` are the [JSON
protocol](http://www.catb.org/gpsd/gpsd_json.html) and the [client
HOWTO](http://catb.org/gpsd/client-howto.html).

# Development notes

Start `gpsd` with a real GPS device:

```sh
/usr/local/sbin/gpsd -N -D4 /dev/tty.SLAB_USBtoUART
```

Or start [gpsd](http://catb.org/gpsd/gpsd.html) with a TCP stream to a remote GPS:

```sh
/usr/local/sbin/gpsd -N -D2 tcp://<IP>:<PORT>
```

Test the connection to `gpsd` with `telnet localhost 2947` and send the string:

```text
?WATCH={"enable":true,"json":true};
```
