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
extern crate lazy_static;
extern crate regex;
extern crate sonogram;
extern crate x3;

use std::fs::File;
use std::io::prelude::*;
use std::str;
use std::thread;
use std::time::Duration;

use regex::Regex;
use sonogram::{blackman_harris, SpecOptionsBuilder};

use gift_code::date_now;
use gift_code::errors::GiftError;

const MAX_HTTP_HEADER_LEN: usize = 1024;
const SERVER_SAVE_PATH: &str = "data";

fn find(haystack: &[u8], needle: &[u8]) -> Result<usize, GiftError> {
  haystack
    .windows(needle.len())
    .position(|window| window == needle)
    .ok_or(GiftError::HttpErrorOnFind)
}

fn to_str(buf: &[u8]) -> Result<&str, GiftError> {
  str::from_utf8(buf).map_err(GiftError::Str)
}

fn path_to_buoy_id(path: &str) -> Result<&str, GiftError> {
  lazy_static! {
    static ref RE: Regex = Regex::new(r"^/id/[0-9a-fA-F\-]{1,40}$").unwrap();
  }
  if RE.is_match(path) {
    Ok(&path[4..])
  } else {
    Err(GiftError::HttpInvalidPath)
  }
}

fn json_start(mut file: &File) -> Result<(), GiftError> {
  write!(file, "{{").map_err(GiftError::Io)
}

fn json_out(mut file: &File, property: &str, value: &str) -> Result<(), GiftError> {
  write!(file, "\"{}\": \"{}\"", property, value).map_err(GiftError::Io)
}

fn json_sep(mut file: &File) -> Result<(), GiftError> {
  write!(file, ",").map_err(GiftError::Io)
}

fn json_end(mut file: &File) -> Result<(), GiftError> {
  write!(file, "}}").map_err(GiftError::Io)
}

fn write_headers_to_file(
  req: &httparse::Request,
  buoy_id: &str,
  date: &str,
  num_errors: usize,
) -> Result<(), GiftError> {
  let filename = format!("{}/{}.{}.json", SERVER_SAVE_PATH, buoy_id, date);
  let meta_file = File::create(filename).map_err(GiftError::Io)?;
  json_start(&meta_file)?;

  for header in req.headers.iter() {
    // print!("{}:{}; ", header.name, to_str(header.value)?);
    json_out(&meta_file, header.name, to_str(header.value)?)?;
    json_sep(&meta_file)?;
  }
  // print out the number of errors
  json_out(&meta_file, "decode_errors", &format!("{}", num_errors))?;
  json_sep(&meta_file)?;

  // println!();

  json_out(&meta_file, "buoy_id", buoy_id)?;
  json_end(&meta_file)?;

  Ok(())
}

// Write the data to an .x3 file
fn write_raw_data_to_file(buf: &[u8], buoy_id: &str, date: &str) -> Result<(), GiftError> {
  let filename = format!("{}/{}.{}.bin", SERVER_SAVE_PATH, buoy_id, date);
  let mut meta_file = File::create(filename).map_err(GiftError::Io)?;

  // Find the body
  let idx = find(buf, gift_code::HTTP_HEADER_END.as_bytes())?;
  meta_file
    .write_all(&buf[(idx + gift_code::HTTP_HEADER_END.len())..])
    .map_err(GiftError::Io)
}

// Convert the .bin file that has already been saved to a .wav file
fn write_wav_data_to_file(buoy_id: &str, date: &str) -> Result<usize, GiftError> {
  let bin_file = format!("{}/{}.{}.bin", SERVER_SAVE_PATH, buoy_id, date);
  let wav_file = format!("{}/{}.{}.wav", SERVER_SAVE_PATH, buoy_id, date);

  match x3::decodefile::x3bin_to_wav(bin_file, wav_file) {
    Ok(num_errors) => Ok(num_errors),
    Err(e) => {
      eprintln!("Error parsing or saving x3bin file: {:?}", e);
      Err(GiftError::X3SaveIssue)
    }
  }
}

///
/// Get the date from the header, if it's not found, use the current date/time
///
fn date_from_header(req: &httparse::Request) -> String {
  for header in req.headers.iter() {
    if header.name == "Start-Time" {
      let date = to_str(header.value);
      if date.is_err() {
        // There was an error
        error!("date_from_header(): Error parsing date");
        break;
      }
      let date = date.unwrap();
      if &date[..4] == "1970" {
        // The FX30 has recently booted and we need to wait till it's got the Unix time
        break;
      }
      return String::from(date);
    }
  }

  date_now()
}

pub fn save_spectrogram_png(buoy_id: &str, date: &str) -> Result<(), GiftError> {
  let wav_file = format!("{}/{}.{}.wav", SERVER_SAVE_PATH, buoy_id, date);
  let png_file = format!("{}/{}.{}.png", SERVER_SAVE_PATH, buoy_id, date);

  let mut spectrograph = SpecOptionsBuilder::new(512, 128)
    .set_window_fn(blackman_harris)
    .load_data_from_file(&std::path::Path::new(&wav_file))?
    .downsample(2)
    .scale(60.0)
    .build();

  spectrograph.compute(1024, 0.6);
  spectrograph.save_as_png(&std::path::Path::new(&png_file), false)?;

  Ok(())
}

pub fn save_http_post(buf: &[u8]) -> Result<(), GiftError> {
  let mut headers = [httparse::EMPTY_HEADER; MAX_HTTP_HEADER_LEN];
  let mut req = httparse::Request::new(&mut headers);

  let parsed_req = req.parse(buf).map_err(GiftError::Parse)?;

  if parsed_req.is_complete() {
    let http_path = req.path.ok_or(GiftError::HttpInvalidPath)?;
    let buoy_id = path_to_buoy_id(http_path)?;
    let dt_str = date_from_header(&req);

    write_raw_data_to_file(buf, buoy_id, &dt_str)?;

    let num_errors;
    if buf.len() > gift_code::MIN_X3_FILE_SIZE * 2 {
      num_errors = write_wav_data_to_file(buoy_id, &dt_str)?;

      // Sleep a bit, because we need to read the wav file
      thread::sleep(Duration::from_millis(500));
      save_spectrogram_png(buoy_id, &dt_str)?;
    } else {
      num_errors = 0;
    }

    // Needs to happen last, we will trigger changes
    write_headers_to_file(&req, buoy_id, &dt_str, num_errors)?;

    Ok(())
  } else {
    Err(GiftError::HttpInvalidRequest)
  }
}
