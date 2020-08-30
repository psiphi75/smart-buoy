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
extern crate quinn;
extern crate tokio;
#[macro_use]
extern crate failure;
extern crate buoy_code;
extern crate futures;
extern crate rustls;
extern crate tokio_current_thread;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::thread;

use std::ascii;
use std::fmt;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str;

use failure::{Fail, ResultExt};
use futures::{Future, Stream};
use tokio::runtime::current_thread::Runtime;

use failure::Error;

pub mod save_post;
use save_post::save_http_post;

type Result<T> = std::result::Result<T, Error>;

pub struct PrettyErr<'a>(&'a dyn Fail);
impl<'a> fmt::Display for PrettyErr<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    fmt::Display::fmt(&self.0, f)?;
    let mut x: &dyn Fail = self.0;
    while let Some(cause) = x.cause() {
      f.write_str(": ")?;
      fmt::Display::fmt(&cause, f)?;
      x = cause;
    }
    Ok(())
  }
}

pub trait ErrorExt {
  fn pretty(&self) -> PrettyErr;
}

impl ErrorExt for Error {
  fn pretty(&self) -> PrettyErr {
    PrettyErr(self.as_fail())
  }
}

struct Opt {
  key_path: PathBuf,
  cert_path: PathBuf,
  listen: SocketAddr,
}

fn main() {
  println!("Running");
  let opt = Opt {
    key_path: PathBuf::from(buoy_code::CA_SERVER_RSA_PATH),
    cert_path: PathBuf::from(buoy_code::CA_SERVER_CHAIN_PATH),
    listen: SocketAddr::from(([0, 0, 0, 0], buoy_code::QUIC_PORT)),
  };
  let code = {
    if let Err(e) = run(opt) {
      println!("ERROR: {}", e.pretty());
      1
    } else {
      0
    }
  };
  ::std::process::exit(code);
}

fn run(options: Opt) -> Result<()> {
  let mut server_config = quinn::ServerConfigBuilder::default();
  server_config.protocols(buoy_code::ALPN_QUIC_HTTP);
  server_config.use_stateless_retry(true);

  let key = fs::read(&options.key_path).context("failed to read private key")?;
  let key = if options.key_path.extension().map_or(false, |x| x == "der") {
    quinn::PrivateKey::from_der(&key)?
  } else {
    quinn::PrivateKey::from_pem(&key)?
  };
  let cert_chain = fs::read(&options.cert_path).context("failed to read certificate chain")?;
  let cert_chain = if options.cert_path.extension().map_or(false, |x| x == "der") {
    quinn::CertificateChain::from_certs(quinn::Certificate::from_der(&cert_chain))
  } else {
    quinn::CertificateChain::from_pem(&cert_chain)?
  };
  server_config.certificate(cert_chain, key)?;

  let mut endpoint = quinn::Endpoint::builder();
  endpoint.listen(server_config.build());

  let (endpoint_driver, incoming) = {
    let (driver, endpoint, incoming) = endpoint.bind(options.listen)?;
    info!("listening on {}", endpoint.local_addr()?);
    (driver, incoming)
  };

  let mut runtime = Runtime::new()?;
  runtime.spawn(incoming.for_each(move |conn| {
    handle_connection(conn);
    Ok(())
  }));
  runtime.block_on(endpoint_driver)?;

  Ok(())
}

fn handle_connection(conn: quinn::Connecting) {
  // We ignore errors from the driver because they'll be reported by the `incoming` handler anyway.
  tokio_current_thread::spawn(
    conn
      .map_err({
        move |e| {
          error!(
            "incoming handshake failed: {reason}",
            reason = e.to_string()
          );
        }
      })
      .and_then(move |new_conn| {
        let conn = new_conn.connection;
        println!(
          "connection established: remote_id: {}; address: {}; protocol: {};",
          conn.remote_id(),
          conn.remote_address(),
          conn.protocol().map_or_else(
            || "<none>".into(),
            |x| String::from_utf8_lossy(&x).into_owned()
          )
        );

        // Each stream initiated by the client constitutes a new request.
        tokio_current_thread::spawn(
          new_conn
            .streams
            .map_err(move |e| info!("connection terminated: reason: {}", e))
            .for_each(move |stream| {
              handle_request(stream);
              Ok(())
            }),
        );

        // We ignore errors from the driver because they'll be reported by the `incoming` handler anyway.
        new_conn.driver.map_err(|_| ())
      }),
  );
}

const MAX_STREAM_SIZE: usize = 50 * 1024 * 1024;

fn handle_request(stream: quinn::NewStream) {
  let (send, recv) = stream.unwrap_bi();

  tokio_current_thread::spawn(
    recv
      .read_to_end(MAX_STREAM_SIZE) // Read the request, which must be at most 64KiB
      .map_err(|e| format_err!("failed reading request: {}", e))
      .and_then(move |req| {
        let mut escaped = String::new();
        for &x in &req[..] {
          let part = ascii::escape_default(x).collect::<Vec<_>>();
          escaped.push_str(str::from_utf8(&part).unwrap());
        }
        info!("got request: content {}", &escaped[0..20]);

        let http_type = request_type(&req);
        if http_type.is_err() {
          error!("Invalid HTTP Type");
        }

        // Execute the request
        let process = match http_type.unwrap() {
          HTTPRequestType::POST => process_post,
          _ => process_error,
        };
        let resp = process(req).unwrap_or_else(move |e| {
          error!("failed to process request: reason: {}", e.pretty());
          format!("failed to process request: {}\n", e.pretty())
            .into_bytes()
            .into()
        });

        // Write the response
        tokio::io::write_all(send, resp).map_err(|e| format_err!("failed to send response: {}", e))
      })
      // Gracefully terminate the stream
      .and_then(|(send, _)| {
        send
          .finish()
          .map_err(|e| format_err!("failed to shutdown stream: {}", e))
      })
      .map_err(move |e| error!("request failed: reason: {}", e.pretty())),
  )
}

#[allow(dead_code)]
enum HTTPRequestType {
  GET,
  HEAD,
  POST,
  PUT,
  DELETE,
  CONNECT,
  OPTIONS,
  TRACE,
  PATCH,
}

fn first_token(buf: &[u8]) -> &[u8] {
  for (i, &item) in buf.iter().enumerate() {
    if item == b' ' {
      return &buf[0..=i];
    }
  }

  &buf
}

fn request_type(buf: &[u8]) -> Result<HTTPRequestType> {
  match first_token(buf) {
    b"GET " => Ok(HTTPRequestType::GET),
    b"POST " => Ok(HTTPRequestType::POST),
    _ => {
      error!("'{}'", str::from_utf8(first_token(buf)).unwrap());
      bail!("unhandled http request type");
    }
  }
}

fn process_post(buf: Vec<u8>) -> Result<Box<[u8]>> {
  if buf.len() < 5 || &buf[0..5] != b"POST " {
    bail!("missing POST");
  }
  if !buf.ends_with(buoy_code::END_BOUNDARY.as_bytes()) {
    bail!("missing END_BOUNDARY");
  }

  // All good.  Let's move the heavy processing to a thread
  thread::spawn(move || {
    let payload_len = buf.len() - buoy_code::END_BOUNDARY.len();
    let payload = &buf[0..payload_len];
    save_http_post(payload).unwrap();
  });

  // TODO: Add specific FX30 command response here
  Ok(Box::new(*b"HTTP/1.1 200 OK\r\n\r\n"))
}

fn process_error(_buf: Vec<u8>) -> Result<Box<[u8]>> {
  error!("Unhandled request");
  Ok(Box::new(*b"HTTP/1.1 501 Not Implemented\r\n\r\n"))
}
