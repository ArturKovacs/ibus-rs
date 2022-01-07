use std::{
    io::BufRead,
    path::{Path, PathBuf},
    sync::Arc,
};

use bitflags::bitflags;
use thiserror::Error;

mod input_context;
pub use input_context::*;

pub(crate) const REQ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

bitflags! {
    pub struct Capabilites: u32 {
        const PREEDIT_TEXT = 1 << 0;
        const AUXILIARY_TEXT = 1 << 1;
        const LOOKUP_TABLE = 1 << 2;
        const FOCUS = 1 << 3;
        const PROPERTY = 1 << 4;
        const SURROUNDING_TEXT = 1 << 5;
    }
}

#[derive(Debug, Error)]
pub enum Error {
    DBus(#[from] dbus::Error),
    Unknown { description: String },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Yeah Display is the same as Debug... I'm lazy
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AfterCallback {
    /// Returning this from a callback closure causes it to be removed from
    /// the 'listeners' and the closure won't be called again
    Remove,
    /// Returning this from a callback closure allows the closure to be called
    /// again the next time the signal is emited.
    Keep,
}
impl AfterCallback {
    fn to_bool(self) -> bool {
        match self {
            AfterCallback::Remove => false,
            AfterCallback::Keep => true,
        }
    }
}

pub struct Bus {
    conn: Arc<dbus::blocking::Connection>,
}

impl Bus {
    pub fn new() -> Result<Self, Error> {
        let addr = get_address().map_err(|e| Error::Unknown { description: e })?;
        let mut channel = dbus::channel::Channel::open_private(&addr)?;
        channel.register()?;
        Ok(Bus {
            conn: Arc::new(dbus::blocking::Connection::from(channel)),
        })
    }

    pub fn create_input_context(&self, name: &str) -> Result<InputContext, Error> {
        let ibus =
            self.conn
                .with_proxy("org.freedesktop.IBus", "/org/freedesktop/IBus", REQ_TIMEOUT);
        let (obj_path,): (dbus::strings::Path,) =
            ibus.method_call("org.freedesktop.IBus", "CreateInputContext", (name,))?;

        Ok(InputContext {
            conn: self.conn.clone(),
            obj_path,
        })
    }

    /// Returns:
    /// - `Ok(true)` if a new message was successfully processed
    /// - `Ok(false)` if there was no message in the queue
    /// - `Err(e)` if there was an error
    pub fn try_process(&self) -> Result<bool, Error> {
        let processed = self.conn.process(std::time::Duration::from_millis(0))?;
        Ok(processed)
    }
}

fn get_machine_id() -> Result<String, String> {
    if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
        return Ok(id.trim().to_owned());
    }
    if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
        return Ok(id.trim().to_owned());
    }
    Err("Could not get the machine id".into())
}

// Based on https://seoyoungjin.github.io/ibus/text%20input/IBus/
fn get_address() -> Result<String, String> {
    if let Ok(addr) = std::env::var("IBUS_ADDRESS") {
        return Ok(addr);
    }

    let display;
    if let Ok(disp) = std::env::var("DISPLAY") {
        display = disp;
    } else {
        display = ":0.0".into();
    }
    let mut split = display.split(":");
    let mut host = split.next().map_or_else(
        || Err(String::from("Failed to get host from display")),
        |x| Ok(x),
    )?;
    let disp_num = split.next().map_or_else(
        || {
            Err(String::from(
                "Failed to get display number from display (colon)",
            ))
        },
        |x| {
            x.split(".").next().map_or_else(
                || Err("Failed to get display number from display (period)".into()),
                |x| Ok(x),
            )
        },
    )?;
    if host.len() == 0 {
        host = "unix";
    }

    let config_home: PathBuf;
    if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME") {
        config_home = cfg_home.into();
    } else {
        if let Ok(home) = std::env::var("HOME") {
            config_home = Path::new(&home).join(".config");
        } else {
            return Err("Could not find the home config folder".into());
        }
    }

    let machine_id = get_machine_id()?;
    let mut addr_filename = config_home.clone();
    addr_filename = addr_filename.join("ibus/bus");
    addr_filename = addr_filename.join(format!("{}-{}-{}", machine_id, host, disp_num));

    let addr_file = std::fs::File::open(&addr_filename)
        .map_err(|e| format!("Couldn't open {:?}, err was: {}", addr_filename, e))?;
    let reader = std::io::BufReader::new(addr_file);
    let prefix = "IBUS_ADDRESS=";
    for line in reader.lines() {
        match line {
            Ok(line) => {
                let line = line.trim_start();
                if let Some(addr) = line.strip_prefix(prefix) {
                    return Ok(addr.to_owned());
                }
            }
            Err(e) => {
                return Err(format!(
                    "Failed to read line from the ibus address file: {}",
                    e
                ));
            }
        }
    }
    Err(format!("Failed to find {:?} in the address file", prefix))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
