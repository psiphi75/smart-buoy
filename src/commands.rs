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

///
/// Commands that are sent from the server to FX30.
///
///
use std::sync::mpsc::Sender;

use crate::errors::GiftError;
use crate::ControllerAction::{self, CtrlServerCmd};

pub const RESPONSE_OK: &str = "HTTP/1.1 200 OK\r\n\r\n";

#[derive(Clone)]
pub enum FX30Command {
  Normal, // Normal operation.
}

pub fn handle_fx30_command(cmd: FX30Command) -> Result<(), GiftError> {
  match cmd {
    FX30Command::Normal => Ok(()),
  }
}

fn parse_server_response(resp: &str) -> Result<ControllerAction, GiftError> {
  debug!("parse_server_response(): {:?}", resp);

  if resp == RESPONSE_OK {
    Ok(CtrlServerCmd(FX30Command::Normal))
  } else {
    error!("handle_server_response(): error in response: {}", resp);
    Ok(CtrlServerCmd(FX30Command::Normal))
  }
}

pub fn handle_server_response(
  action_tx: Sender<ControllerAction>,
  resp: &[u8],
) -> Result<(), GiftError> {
  let s = String::from_utf8(resp.to_vec()).unwrap();
  let action = crate::commands::parse_server_response(&s).unwrap();
  action_tx.send(action).unwrap();
  Ok(())
}
