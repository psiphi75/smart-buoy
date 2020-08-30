#![allow(unused_imports)]
#![allow(dead_code)]

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
use regex::Regex;
use std::fs::File;
use std::io::prelude::*;

use buoy_code::errors::GiftError;
use buoy_code::GPIO_ADC_PATH;

// Calibration
// These two values are calculated using least squares approximation of
// the equation y = CAL_A * x + CAL_B.  Where x is the raw voltage
// and y is the actual voltage.
//
// The following link works well to calculate these values:
//    http://neoprogrammics.com/linear_least_squares_regression/index.php
//
// These are the some example values used - the "Raw"
// 0.0V => Result:10499 Raw:24812
// 1.280V => Result:453493 Raw:29327
// 5.19V => Result:1796016 Raw:43010
const CAL_A: f32 = 0.001_029_309;
const CAL_B: f32 = -26.655_219;

fn parse_voltage(buffer: String) -> Result<f32, GiftError> {
  lazy_static! {
    static ref RE: Regex = Regex::new(r"^Result:\d+ Raw:(?P<raw_v>\d+)").unwrap();
  }

  let re = RE.captures(&buffer);
  if re.is_none() {
    return Err(GiftError::ParseVoltage);
  }
  let caps = re.unwrap();

  let raw_v_str = &caps["raw_v"];
  let raw_v: f32 = raw_v_str.parse::<f32>()?;
  let v = CAL_A * raw_v + CAL_B;

  Ok(v)
}

/// Get the voltage of the ADC covertor.
#[cfg(feature = "fx30")]
pub fn get_voltage() -> Result<f32, GiftError> {
  let mut f = File::open(GPIO_ADC_PATH)?;
  let mut buffer = String::new();
  f.read_to_string(&mut buffer)?;

  parse_voltage(buffer)
}

// We just use this for testing
#[cfg(not(feature = "fx30"))]
pub fn get_voltage() -> Result<f32, GiftError> {
  Ok(1.0)
}

#[cfg(test)]
mod tests {
  use crate::voltage::parse_voltage;

  #[test]
  fn test_parse_voltage() {
    let v = parse_voltage(String::from("Result:10499 Raw:24812")).unwrap();

    assert_eq!(-31.96656, v);
  }

  #[test]
  fn test_parse_invalid_voltage() {
    let v = parse_voltage(String::from("Result:10499 Raw:asdf"));

    assert!(v.is_err());
  }
}
