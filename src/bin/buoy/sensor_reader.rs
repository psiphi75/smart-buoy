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
extern crate x3;

use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time;

use serialport::prelude::*;

use crate::voltage::get_voltage;

use gift_code::date_now;
use gift_code::errors::GiftError;
use gift_code::BuoyData;
use gift_code::ControllerAction;

fn find_first(search_buf: &[u8], target_buf: &[u8]) -> Option<usize> {
  if target_buf.len() > search_buf.len() {
    return None;
  }

  for i in 0..(search_buf.len() - target_buf.len()) {
    let mut found = true;
    for j in 0..target_buf.len() {
      if search_buf[i + j] != target_buf[j] {
        found = false;
      }
    }
    if found {
      return Some(i);
    }
  }

  None
}

fn find_last(search_buf: &[u8], target_buf: &[u8]) -> Option<usize> {
  if target_buf.len() > search_buf.len() {
    return None;
  }

  for i in (0..(search_buf.len() - target_buf.len())).rev() {
    let mut found = true;
    for j in 0..target_buf.len() {
      if search_buf[i + j] != target_buf[j] {
        found = false;
      }
    }
    if found {
      return Some(i);
    }
  }

  None
}

//
// Clean the x3 data by dropping frames that are invalid.  The most likely
// point of corruption is through the RS485 connection.  If we drop the
// the frames on the buoy it means less data is transmitted.
//
fn clean_x3_data(buf: &[u8], header: &[u8]) -> (Vec<u8>, Vec<u8>) {
  //
  // Find the beginning and the end of the frames
  //

  let start = match find_first(buf, &header) {
    Some(p) => p,
    None => {
      info!("clean_x3_data: no start frame");
      return (buf.to_vec(), Vec::new());
    }
  };

  let end = match find_last(buf, &header) {
    Some(p) => p,
    None => {
      info!("clean_x3_data: no end frame");
      return (buf.to_vec(), Vec::new());
    }
  };

  if end <= start {
    info!("clean_x3_data: last frame is less than start frame");
    return (buf.to_vec(), Vec::new());
  }

  //
  // Now we clean up buf and return the data we want.
  //
  let result_buf = buf[start..end].to_vec();
  let remainder_buf = buf[end..].to_vec();

  (result_buf, remainder_buf)
}

///
/// Return the OS uptime.  Return 0 if there was an error
fn get_os_uptime() -> i64 {
  match uptime_lib::get() {
    Ok(uptime) => uptime.num_milliseconds() / 1000,
    Err(err) => {
      error!("ERROR: get_os_uptime(): {}", err);
      0
    }
  }
}

pub fn create_buoy_data(
  hydrophone: Option<Vec<u8>>,
  start_time: Option<String>,
) -> Result<BuoyData, GiftError> {
  Ok(BuoyData {
    id: gift_code::BUOY_ID,
    hydrophone: if hydrophone.is_none() {
      vec![]
    } else {
      hydrophone.unwrap()
    },
    voltage: get_voltage()?,
    dropped_blocks: 0,
    gps: String::from(""),
    start_time: if start_time.is_none() {
      date_now()
    } else {
      start_time.unwrap()
    },
    uptime: get_os_uptime(),
  })
}

fn read_loop(
  mut port: Box<dyn SerialPort>,
  data_tx: &Sender<ControllerAction>,
) -> Result<(), GiftError> {
  let mut serial_buf: Vec<u8> = vec![0; gift_code::SERIAL_BUF_SIZE];
  let mut send_buf = Vec::new();
  let mut rec_time = time::Instant::now(); // How long we've been recoding for
  let mut start_time = date_now();

  // This is what the header looks like.
  let header_start: [u8; 4] = [b'S', b'T', 0x00, 0x01];

  loop {
    // Read data from UART
    match port.read(serial_buf.as_mut_slice()) {
      Ok(bytes_read) => {
        send_buf.extend_from_slice(&serial_buf[0..bytes_read]);
      }
      Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (), // This is okay.
      Err(e) => {
        error!("Serial error: {:?}", e);
        return Err(GiftError::Io(e));
      }
    }

    // Send data when required
    if rec_time.elapsed().as_secs() > gift_code::FX30_RECORD_LEN && !send_buf.is_empty() {
      let (buf, remainder_buf) = clean_x3_data(&send_buf, &header_start);
      send_buf = remainder_buf;
      info!(
        "Collected hydrophone data: {} - {} bytes",
        start_time,
        buf.len()
      );
      data_tx.send(ControllerAction::CtrlBuoyData(create_buoy_data(
        Some(buf),
        Some(start_time),
      )?))?;

      // Restart the collection timers
      rec_time = time::Instant::now();
      start_time = date_now();
    }
  }
}

pub fn sensor_reader(
  data_tx: &Sender<ControllerAction>,
  port_name: &PathBuf,
  port_baud: u32,
) -> Result<(), GiftError> {
  let settings = SerialPortSettings {
    baud_rate: port_baud,
    data_bits: DataBits::Eight,
    flow_control: FlowControl::None,
    parity: Parity::None,
    stop_bits: StopBits::One,
    timeout: time::Duration::from_millis(200),
  };

  info!(
    "Receiving data from hydrophone on {:?} at {} baud:",
    &port_name, port_baud
  );

  match serialport::open_with_settings(&port_name, &settings) {
    Ok(port) => read_loop(port, data_tx),
    Err(e) => {
      error!(
        "sensor_reader(): An error occurred reading serial port: {:?}",
        GiftError::Serialport(e)
      );
      Ok(())
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::sensor_reader::*;

  const TARGET_BUF: &[u8; 4] = &[b'S', b'T', 0x00, 0x01];
  const BUF: &[u8; 30] = &[
    b'S', b'T', 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, b'S',
    b'T', 0x00, 0x01, 0x01, 0x00, 0x01, b'S', b'T', 0x00, 0x01, 0x01, b'S', b'T', 0x00,
  ];
  #[test]
  fn test_findfirst() {
    assert_eq!(0, find_first(BUF, TARGET_BUF).unwrap());
    assert_eq!(14, find_first(&BUF[1..], TARGET_BUF).unwrap());
    assert_eq!(None, find_first(&BUF[28..], TARGET_BUF));
    assert_eq!(None, find_first(BUF, &[0xff, 0xfe, 0xf1]));
  }

  #[test]
  fn test_findlast() {
    assert_eq!(22, find_last(BUF, TARGET_BUF).unwrap());
    assert_eq!(0, find_last(&BUF[22..], TARGET_BUF).unwrap());
    assert_eq!(None, find_last(&BUF[23..], TARGET_BUF));
    assert_eq!(None, find_last(&BUF[28..], TARGET_BUF));
    assert_eq!(None, find_last(BUF, &[0xff, 0xfe, 0xf1]));
  }

  #[test]
  fn test_clean_x3_data() {
    let exp_clean_buf = vec![
      b'S', b'T', 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00,
      b'S', b'T', 0x00, 0x01, 0x01, 0x00, 0x01,
    ];
    let exp_rem_buf = vec![b'S', b'T', 0x00, 0x01, 0x01, b'S', b'T', 0x00];

    assert_eq!(
      (exp_clean_buf, exp_rem_buf),
      clean_x3_data(&BUF.to_vec(), TARGET_BUF)
    );
  }
}
