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
use std::fs;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use futures::Future;
use quinn_proto;
use tokio::prelude::*;
use tokio::runtime::current_thread::Runtime;
use url::Url;

use buoy_code::errors::GiftError;
use buoy_code::BuoyData;
use buoy_code::ControllerAction;
use buoy_code::{END_BOUNDARY, SW_VERSION};

pub struct Transmit<'a> {
  remote: std::net::SocketAddr,
  client_config: quinn_proto::ClientConfig,
  // endpoint: quinn::Endpoint,
  host: String,
  action_tx: &'a Sender<ControllerAction>,
}

impl<'a> Transmit<'a> {
  pub fn new(
    url: Url,
    ca_path: PathBuf,
    action_tx: &'a Sender<ControllerAction>,
  ) -> Result<Self, GiftError> {
    let remote = url
      .with_default_port(|_| Ok(4433))?
      .to_socket_addrs()?
      .next()
      .ok_or(GiftError::RemoteUrlError)?;

    let mut config_builder = quinn::ClientConfigBuilder::default();
    config_builder.protocols(buoy_code::ALPN_QUIC_HTTP);

    info!("Loading cert authority: {:?}", ca_path);
    config_builder
      .add_certificate_authority(quinn::Certificate::from_der(&fs::read(&ca_path)?)?)?;
    let client_config = config_builder.build();

    Ok(Self {
      remote,
      client_config,
      // endpoint,
      host: String::from(
        url
          .host_str()
          .ok_or_else(|| format_err!("URL missing host"))?,
      ),
      action_tx,
    })
  }

  pub fn send(&mut self, buoy: &BuoyData) -> Result<(), GiftError> {
    info!("Sending request");

    //
    // Build the runtime used to send the data
    //
    let request = build_http_post(buoy)?;
    let action_tx = Sender::clone(&self.action_tx);
    let start = Instant::now();

    let mut endpoint = quinn::Endpoint::builder();
    endpoint.default_client_config(self.client_config.clone());
    let (endpoint_driver, endpoint, _) = endpoint.bind("0.0.0.0:0")?;

    let mut runtime = Runtime::new()?;
    runtime.spawn(endpoint_driver.map_err(|e| error!("IO error: {}", e)));

    //
    // The send-data thread
    //

    runtime.block_on(
      endpoint
        .connect(&self.remote, &self.host)?
        .timeout(buoy_code::FX30_CONNECT_TIMEOUT)
        .map_err(|e| format_err!("failed to connect: {}", e))
        .and_then(move |new_conn| {
          tokio_current_thread::spawn(
            new_conn
              .driver
              .map_err(|e| eprintln!("connection lost: {}", e)),
          );
          let conn = new_conn.connection;
          let stream = conn.open_bi();
          stream
            .map_err(|e| format_err!("failed to open stream: {}", e))
            .and_then(move |(send, recv)| {
              // Send the request
              tokio::io::write_all(send, request.to_owned())
                .timeout(buoy_code::FX30_UPLOAD_SEND_TIMEOUT)
                .map_err(|e| format_err!("failed to send request: {}", e))
                .and_then(|(send, _)| {
                  send
                    .finish()
                    .map_err(|e| format_err!("failed to shutdown stream: {}", e))
                })
                .and_then(move |_| {
                  recv
                    .read_to_end(usize::max_value())
                    .timeout(buoy_code::FX30_UPLOAD_RESP_TIMEOUT)
                    .map_err(|e| format_err!("failed to read response: {}", e))
                    .map(move |resp| {
                      buoy_code::commands::handle_server_response(action_tx, &resp).unwrap();
                    })
                })
            })
            .map(move |_| {
              let seconds = duration_secs(&start.elapsed());
              let kb = buoy.hydrophone.len() as f32 / 1024.0;
              info!("uploaded: {:0.1} kB at {:0.2} kB/s", kb, kb / seconds);
              conn.close(0u32.into(), b"done");
            })
        }),
    )?;

    //
    // Wrap up
    //

    // Allow the endpoint driver to automatically shut down - if not, then the
    // the call to run() below will permanently block the thread.
    drop(endpoint);

    // Let the connection to finish closing gracefully
    runtime.run().unwrap(); // FIXME: Need to handle this

    Ok(())
  }
}

fn build_http_post(buoy: &BuoyData) -> Result<Vec<u8>, GiftError> {
  let header = format!(
    "POST /id/{} HTTP/1.1\r\n\
     Host: /id/{}\r\n\
     Content-Type: multipart/form-data\r\n\
     Battery-Voltage: {}\r\n\
     Dropped-Blocks: {}\r\n\
     GPS: {}\r\n\
     Start-Time: {}\r\n\
     Uptime: {}\r\n\
     sw-version: {}\r\n\
     length: {}{}",
    buoy.id,
    buoy.id,
    buoy.voltage,
    buoy.dropped_blocks,
    buoy.gps,
    buoy.start_time,
    buoy.uptime,
    SW_VERSION,
    buoy.hydrophone.len(),
    buoy_code::HTTP_HEADER_END
  )
  .into_bytes();
  let end = END_BOUNDARY.as_bytes();
  let mut post = Vec::new();

  post.extend_from_slice(&header);
  post.extend_from_slice(&buoy.hydrophone);
  post.extend_from_slice(&end);

  Ok(post)
}

fn duration_secs(x: &Duration) -> f32 {
  x.as_secs() as f32 + x.subsec_nanos() as f32 * 1e-9
}
