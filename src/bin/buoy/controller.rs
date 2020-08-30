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

#[cfg(feature = "fx30")]
use std::fs;
#[cfg(feature = "fx30")]
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
#[cfg(feature = "fx30")]
use std::process::Stdio;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use time::now as utc_time_now;
use url::Url;

use crate::data_send::Transmit;
use crate::sensor_reader;
use crate::voltage;
use buoy_code::commands::handle_fx30_command;
use buoy_code::errors::GiftError;
use buoy_code::ControllerAction::{self, *};

#[cfg(feature = "fx30")]
fn read_gps(last_gps: Arc<Mutex<String>>) -> Result<(), GiftError> {
  // Start the actual connection
  let mut gps_cmd = Command::new(buoy_code::GPS_SCRIPT)
    .stdout(Stdio::piped())
    .spawn()
    .map_err(GiftError::Io)?;
  {
    let stdout = gps_cmd.stdout.as_mut().ok_or(GiftError::GPSIssue)?;
    let stdout_lines = BufReader::new(stdout).lines();

    for line in stdout_lines {
      let ln = line.unwrap();
      let mut gps = last_gps.lock().unwrap();
      *gps = ln;
      debug!("read_gps(): GPS => {}", *gps);
      // Any data means we have the result
      break;
    }
  }

  // Although the process exited, we still need to close it
  gps_cmd.wait()?;

  Ok(())
}

///
/// Read the GPS every buoy_code::GPS_ACQUISITION_PERIOD seconds.
///
/// This spawns a thread.
///
#[cfg(feature = "fx30")]
fn read_gps_thread(last_gps: Arc<Mutex<String>>) {
  thread::spawn(move || loop {
    thread::sleep(buoy_code::GPS_ACQUISITION_PERIOD);
    read_gps(Arc::clone(&last_gps)).unwrap();
  });
}

fn is_nz_daylight() -> bool {
  let utc_time = utc_time_now();
  utc_time.tm_hour > 19 || utc_time.tm_hour < 5
}

///
/// Blink the buoy navigation light for a short period, then wait for a longer
/// period.
///
/// Maritime NZ says 5 quick flashes every 20 seconds for a scientific buoy.
/// That also looks like what DTA were doing. Flash duration ~500ms  should be ok.
///
///
/// This spawns a thread.
///
#[cfg(feature = "fx30")]
fn blink_buoy_light() {
  thread::spawn(move || loop {
    thread::sleep(buoy_code::BUOY_NAV_LIGHT_LONG_INT);
    let is_daylight = is_nz_daylight();
    for _ in 0..buoy_code::BUOY_NAV_LIGHT_NUM_SHORT {
      // Turn ON - but only outside of bright daylight hours (7pm.. 5am UTC)
      if !is_daylight {
        match fs::write(buoy_code::BUOY_NAV_LIGHT_GPIO, b"1") {
          Err(_) => (),
          Ok(()) => (),
        }
      }
      thread::sleep(buoy_code::BUOY_NAV_LIGHT_BLINK_ON);

      // OFF
      match fs::write(buoy_code::BUOY_NAV_LIGHT_GPIO, b"0") {
        Err(_) => (),
        Ok(()) => (),
      }
      thread::sleep(buoy_code::BUOY_NAV_LIGHT_BLINK_OFF);
    }
  });
}

fn run_ulpm(sleeptime: usize) {
  let output = Command::new(buoy_code::ULPM_SCRIPT)
    .arg(sleeptime.to_string())
    .output()
    .expect("failed to execute process");
  error!(
    "run_ulpm done: stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

///
/// Power management - Check the voltage every 5 minutes.  The power management (PM)
/// states are as a follows:
///  FX30_PM_POWER_MEDIUM_THRESH - Any voltage above this value requires no PM.
///  FX30_PM_POWER_LOW_THRESH - Any voltage between this and *_MEDIUM_* requires ULPM .
///
/// This spawns a thread.
///
fn power_management() {
  thread::spawn(move || loop {
    thread::sleep(buoy_code::FX30_PM_TIME_AWAKE);

    let battery_voltage = voltage::get_voltage().unwrap();
    if battery_voltage < buoy_code::FX30_PM_POWER_LOW_THRESH {
      thread::sleep(Duration::from_secs(60));
      run_ulpm(buoy_code::FX30_PM_POWER_LOW_SLEEP_TIME_SEC);
    } else if battery_voltage < buoy_code::FX30_PM_POWER_MEDIUM_THRESH {
      thread::sleep(Duration::from_secs(60));
      run_ulpm(buoy_code::FX30_PM_POWER_MEDIUM_SLEEP_TIME_SEC);
    }
  });
}

fn transmit_data(
  url: Url,
  ca_path: PathBuf,
  action_tx: Sender<ControllerAction>,
  last_gps: &std::sync::Arc<std::sync::Mutex<std::string::String>>,
  mut data: buoy_code::BuoyData,
) {
  // Collect the data from the hydrophone buffer

  // Get the last gps position
  let gps = last_gps.lock().unwrap();
  let t_gps = (*gps).clone();

  thread::spawn(move || {
    // Connect to the server
    let result = Transmit::new(url, ca_path, &action_tx);
    let mut conn = match result {
      Ok(conn) => conn,
      Err(e) => {
        error!("Transmit::new() failed: {:?}", e);
        return;
      }
    };

    data.gps = t_gps;

    // Send the data to the cloud
    match conn.send(&data) {
      Ok(_) => (),
      Err(e) => {
        // Errors are handled in `conn.send()`
        // TODO: should count the number of errors here
        error!("conn.send() failed: {:?}", e)
      }
    };
  });
}

///
/// The main loop
///
pub fn controller(
  url: &Url,
  ca_path: &PathBuf,
  action_tx: Sender<ControllerAction>,
  action_rx: Receiver<ControllerAction>,
) -> Result<(), GiftError> {
  let last_gps = Arc::new(Mutex::new(String::new()));

  // Stuff for the fx30 only
  #[cfg(feature = "fx30")]
  {
    // Tell the GPS every once in a while to do a capture
    read_gps_thread(Arc::clone(&last_gps));

    // Create a thread the blinks the light every so many seconds
    blink_buoy_light();
  }

  // Create the Power management thread
  power_management();

  // The main loop
  loop {
    //
    // Wait
    //

    thread::sleep(buoy_code::FX30_SEND_INTERVAL);

    //
    // Gather all necessary data
    //

    loop {
      let action = action_rx.recv_timeout(buoy_code::FX30_NO_DATA_WAIT);

      // Handle the action
      match action {
        Ok(CtrlBuoyData(data)) => transmit_data(
          url.clone(),
          ca_path.clone(),
          Sender::clone(&action_tx),
          &last_gps,
          data,
        ),
        Ok(CtrlServerCmd(action)) => handle_fx30_command(action)?,
        Err(RecvTimeoutError::Timeout) => {
          debug!("Timed out waiting for hydrophone data");
          let data = sensor_reader::create_buoy_data(None, None)?;
          transmit_data(
            url.clone(),
            ca_path.clone(),
            Sender::clone(&action_tx),
            &last_gps,
            data,
          )
        }
        Err(e) => error!("controller(): Waiting for recv: {:?}", e),
      }
    }
  }
}
