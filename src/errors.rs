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
use crate::ControllerAction;
use httparse;
use quinn;
use serialport;
use sonogram;
use std::io;
use std::str;
use std::sync::mpsc;

// We derive `Debug` because all types should probably derive `Debug`.
// This gives us a reasonable human readable description of `CliError` values.
#[derive(Debug)]
pub enum GiftError {
  Io(io::Error),
  Parse(httparse::Error),
  ParseFloat(std::num::ParseFloatError),
  ParseInt(std::num::ParseIntError),
  ParseUrl(url::ParseError),
  Str(str::Utf8Error),
  Serialport(serialport::Error),
  SerialportKind(serialport::ErrorKind),
  Sonogram(sonogram::SonogramError),
  QuinnRead(quinn::ReadError),
  QuinnWrite(quinn::WriteError),
  QuinnEndpoint(quinn::EndpointError),
  QuinnConnect(quinn::ConnectError),
  QuinnParse(quinn::crypto::rustls::ParseError),
  StdFailure(failure::Error),
  MPSC(mpsc::SendError<ControllerAction>),

  // Custom Http Errors
  HttpInvalidRequest,
  HttpInvalidPath,
  HttpErrorOnFind,
  HttpInvalidMethod,

  // Custom FX30 Errors
  DataConnection,        // Issue with the data connection, or it's process
  DataConnectionTimeout, // The connection timed out
  GPSIssue,              // There was an issue with the GPS command
  SerialPortArgMissing,  // Serial port is missing
  RemoteUrlError,        // Bad remote URL
  X3SaveIssue,           // x3bin to wav save error
  ParseVoltage,          // Error parsing voltage
}

impl From<io::Error> for GiftError {
  fn from(err: io::Error) -> GiftError {
    GiftError::Io(err)
  }
}

impl From<httparse::Error> for GiftError {
  fn from(err: httparse::Error) -> GiftError {
    GiftError::Parse(err)
  }
}

impl From<str::Utf8Error> for GiftError {
  fn from(err: str::Utf8Error) -> GiftError {
    GiftError::Str(err)
  }
}

impl From<std::num::ParseFloatError> for GiftError {
  fn from(err: std::num::ParseFloatError) -> GiftError {
    GiftError::ParseFloat(err)
  }
}

impl From<std::num::ParseIntError> for GiftError {
  fn from(err: std::num::ParseIntError) -> GiftError {
    GiftError::ParseInt(err)
  }
}

impl From<url::ParseError> for GiftError {
  fn from(err: url::ParseError) -> GiftError {
    GiftError::ParseUrl(err)
  }
}

impl From<serialport::Error> for GiftError {
  fn from(err: serialport::Error) -> GiftError {
    GiftError::Serialport(err)
  }
}

impl From<serialport::ErrorKind> for GiftError {
  fn from(err: serialport::ErrorKind) -> GiftError {
    GiftError::SerialportKind(err)
  }
}

impl From<quinn::ReadError> for GiftError {
  fn from(err: quinn::ReadError) -> GiftError {
    GiftError::QuinnRead(err)
  }
}

impl From<quinn::WriteError> for GiftError {
  fn from(err: quinn::WriteError) -> GiftError {
    GiftError::QuinnWrite(err)
  }
}

impl From<quinn::EndpointError> for GiftError {
  fn from(err: quinn::EndpointError) -> GiftError {
    GiftError::QuinnEndpoint(err)
  }
}

impl From<quinn::crypto::rustls::ParseError> for GiftError {
  fn from(err: quinn::crypto::rustls::ParseError) -> GiftError {
    GiftError::QuinnParse(err)
  }
}

impl From<quinn::ConnectError> for GiftError {
  fn from(err: quinn::ConnectError) -> GiftError {
    GiftError::QuinnConnect(err)
  }
}

impl From<failure::Error> for GiftError {
  fn from(err: failure::Error) -> GiftError {
    GiftError::StdFailure(err)
  }
}

impl From<mpsc::SendError<ControllerAction>> for GiftError {
  fn from(err: mpsc::SendError<ControllerAction>) -> GiftError {
    GiftError::MPSC(err)
  }
}

impl From<sonogram::SonogramError> for GiftError {
  fn from(err: sonogram::SonogramError) -> GiftError {
    GiftError::Sonogram(err)
  }
}
