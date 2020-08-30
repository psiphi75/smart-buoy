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
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;
extern crate gift_code;
extern crate quinn;
extern crate rustls;
extern crate time;
extern crate tokio;
extern crate url;

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

pub mod controller;
pub mod data_send;
pub mod sensor_reader;
pub mod voltage;

use url::Url;

use gift_code::errors::GiftError;
use gift_code::{CA_CERT_PATH, HOME_SERVER_URL, SW_VERSION};

use crate::controller::controller;
use crate::sensor_reader::sensor_reader;

#[allow(unreachable_code)]
pub fn handle_error(err: GiftError) {
  error!("ERROR: {:?}", err);
  panic!("Help");
  ::std::process::exit(1);
}

fn main() {
  // Set up the logger
  env_logger::init();
  println!(
    "Starting buoy runtime.\n\tVersion:{}\n\tRemote: {}",
    SW_VERSION, HOME_SERVER_URL
  );

  // The channel for sending actions to controller.
  let (action_tx1, action_rx) = mpsc::channel();
  let action_tx2 = mpsc::Sender::clone(&action_tx1);

  // Get the hydrophone data
  let serial_port = PathBuf::from(gift_code::SERIAL_PATH);
  thread::spawn(move || {
    sensor_reader(&action_tx1, &serial_port, gift_code::SERIAL_BAUD)
      .map_err(handle_error)
      .unwrap();
  });

  // Main controller
  let url = Url::parse(HOME_SERVER_URL).unwrap();
  let ca = PathBuf::from(CA_CERT_PATH);
  controller(&url, &ca, action_tx2, action_rx)
    .map_err(handle_error)
    .unwrap();
}
