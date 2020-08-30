///****************************************************************************
///
///  Smart-Buoy - connects marine sounds to the cloud.
///  Copyright (C) 2020  Simon M. Werner (Anemoi Robotics Ltd)
///
///  This program is free software: you can redistribute it and/or modify
///  it under the terms of the GNU General Public License as published by
///  the Free Software Foundation, either version 3 of the License, or
///  (at your option) any later version.
///
///  This program is distributed in the hope that it will be useful,
///  but WITHOUT ANY WARRANTY; without even the implied warranty of
///  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
///  GNU General Public License for more details.
///
///  You should have received a copy of the GNU General Public License
///  along with this program.  If not, see <https://www.gnu.org/licenses/>.
///
///****************************************************************************
extern crate chrono;
extern crate failure;
extern crate futures;
extern crate httparse;
extern crate quinn;
extern crate rustls;
extern crate serialport;
extern crate tokio;
extern crate url;

#[macro_use]
extern crate log;
extern crate env_logger;

use chrono::Utc;
use core::time::Duration;

pub mod commands;
pub mod errors;

//
//                     ####### #     #  #####    ###
//                     #        #   #  #     #  #   #
//                     #         # #         # #     #
//                     #####      #     #####  #     #
//                     #         # #         # #     #
//                     #        #   #  #     #  #   #
//                     #       #     #  #####    ###
//

pub const BUOY_ID: &str = "1";

const SEND_INT: u64 = 60 * 5;
pub const FX30_SEND_INTERVAL: Duration = Duration::from_secs(SEND_INT);
pub const FX30_RECORD_LEN: u64 = SEND_INT / 8; // How long to record for. We will have this many simultanous upload connections.
pub const FX30_NO_DATA_WAIT: Duration = Duration::from_secs(1800); // How long to wait for hydrophone to send data before we ignore it.

// Power management stuff
pub const FX30_PM_TIME_AWAKE: Duration = Duration::from_secs(SEND_INT * 2 - 60);
pub const FX30_PM_POWER_MEDIUM_THRESH: f32 = 12.0;
pub const FX30_PM_POWER_LOW_THRESH: f32 = 11.0;
pub const FX30_PM_POWER_MEDIUM_SLEEP_TIME_SEC: usize = 30 * 60;
pub const FX30_PM_POWER_LOW_SLEEP_TIME_SEC: usize = 3 * 60 * 60;

// How long to transmit data before we timeout
pub const FX30_UPLOAD_SEND_TIMEOUT: Duration = Duration::from_secs(180);

// Timeout for the response sent back by the server
pub const FX30_UPLOAD_RESP_TIMEOUT: Duration = Duration::from_secs(25);

// How to wait for a connection
pub const FX30_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
pub const SW_VERSION: &str = env!("CARGO_PKG_VERSION"); // The version number

// This is the ADC GPIO on the FX30.  It's the Green wire.
pub const GPIO_ADC_PATH: &str = "/sys/class/hwmon/hwmon0/device/mpp_05";
pub const SERIAL_PATH: &str = "/dev/ttyUSB0";
pub const SERIAL_BAUD: u32 = 230_400; // or 460_800 (original 288_000)
pub const SERIAL_BUF_SIZE: usize = 16384;

pub const GPS_ACQUISITION_PERIOD: Duration = Duration::from_secs(15 * 60); // How often we should probe the GPS, in seconds
pub const GPS_SCRIPT: &str = "/home/root/gps.sh";
pub const ULPM_SCRIPT: &str = "/home/root/sms_scripts/ulpm.sh";
pub const FX30_BIN_NAME: &str = "buoy";

pub const BUOY_NAV_LIGHT_GPIO: &str = "/sys/class/gpio/gpio56/value";
pub const BUOY_NAV_LIGHT_LONG_INT: Duration = Duration::from_millis(16 * 1000); // 18 seconds
pub const BUOY_NAV_LIGHT_NUM_SHORT: u64 = 5;
pub const BUOY_NAV_LIGHT_BLINK_OFF: Duration = Duration::from_millis(400); // 4 seconds worth of flashing
pub const BUOY_NAV_LIGHT_BLINK_ON: Duration = Duration::from_millis(100);

pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-20", b"hq-22"];

//
//                #####
//               #     # ###### #####  #    # ###### #####
//               #       #      #    # #    # #      #    #
//                #####  #####  #    # #    # #####  #    #
//                     # #      #####  #    # #      #####
//               #     # #      #   #   #  #  #      #   #
//                #####  ###### #    #   ##   ###### #    #
//

pub const MIN_X3_FILE_SIZE: usize = 1024;

//
//                #####
//               #     #  ####  #    # #    #  ####  #    #
//               #       #    # ##  ## ##  ## #    # ##   #
//               #       #    # # ## # # ## # #    # # #  #
//               #       #    # #    # #    # #    # #  # #
//               #     # #    # #    # #    # #    # #   ##
//                #####   ####  #    # #    #  ####  #    #
//

pub const DOMAIN: &str = env!("DOMAIN");
pub const CA_CERT_PATH: &str = concat!("./certs/", env!("DOMAIN"), "/ca.der");
pub const CA_SERVER_RSA_PATH: &str = concat!("./certs/", env!("DOMAIN"), "/server.rsa");
pub const CA_SERVER_CHAIN_PATH: &str = concat!("./certs/", env!("DOMAIN"), "/server.chain");
pub const HOME_SERVER_URL: &str = concat!("https://", env!("DOMAIN"), ":4433");

// Protocol specific
pub const QUIC_PORT: u16 = 4433;
pub const END_BOUNDARY: &str = "------END!!!";
pub const HTTP_HEADER_END: &str = "\r\n\r\n";

#[derive(Clone)]
pub struct BuoyData {
  pub id: &'static str,      // The buoy id
  pub hydrophone: Vec<u8>,   // The raw hydrophone data
  pub voltage: f32,          // The voltage read from the battery voltage sensor
  pub dropped_blocks: usize, // The number of dropped blocks
  pub gps: String,           // The GPS location, if available
  pub start_time: String,    // The start time of the recording
  pub uptime: i64,           // The uptime of the buoy operating system
}

#[derive(Clone)]
pub enum ControllerAction {
  CtrlBuoyData(BuoyData),
  CtrlServerCmd(crate::commands::FX30Command),
}

pub fn date_now() -> String {
  Utc::now().format("%Y%m%dT%H%M%S.%3fZ").to_string()
}
