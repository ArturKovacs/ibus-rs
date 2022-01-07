
use std::{path::{PathBuf, Path}, io::BufRead, collections::VecDeque, sync::Arc};

use dbus::{blocking::{Connection, Proxy}, arg::{RefArg, Variant, PropMap}, Message, Signature, channel::Token};
use bitflags::bitflags;
use thiserror::Error;

// mod bus;
// mod object;

// pub use bus::*;
// pub use object::*;

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
        |x| Ok(x)
    )?;
    let disp_num = split.next().map_or_else(
        || Err(String::from("Failed to get display number from display (colon)")),
        |x| {
            x.split(".").next().map_or_else(
                || Err("Failed to get display number from display (period)".into()), 
                |x| Ok(x)
            )
        }
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
    
    let addr_file = std::fs::File::open(&addr_filename).map_err(|e| {
        format!("Couldn't open {:?}, err was: {}", addr_filename, e)
    })?;
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
                return Err(format!("Failed to read line from the ibus address file: {}", e));
            }
        }
    }
    Err(format!("Failed to find {:?} in the address file", prefix))
}


const REQ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

// ////////////////////////////////////////////////////////////
// Commit text
#[derive(Debug)]
pub struct CommitTextSignal {
    pub text: String,
}
impl dbus::arg::ReadAll for CommitTextSignal {
    fn read(i: &mut dbus::arg::Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
        let text_var: Variant<Box<dyn RefArg>> = i.read()?;
        // Structs are represented internally as `VecDeque<Box<RefArg>>`.
        // According to:
        // https://github.com/diwic/dbus-rs/blob/174e8d55b0e17fb6fbd9112e5c1c6119fe8b431b/dbus/examples/argument_guide.md
        let arg: &VecDeque<Box<dyn RefArg>> = dbus::arg::cast(&text_var.0).unwrap();
        Ok(CommitTextSignal {
            text: arg[2].as_str().unwrap_or("").to_owned()
        })
    }
}
impl dbus::message::SignalArgs for CommitTextSignal {
    const NAME: &'static str = "CommitText";
    const INTERFACE: &'static str = "org.freedesktop.IBus.InputContext";
}
// ////////////////////////////////////////////////////////////

// ////////////////////////////////////////////////////////////
// UpdatePreeditText
#[derive(Debug)]
pub struct UpdatePreeditTextSignal {
    pub text: String,
    pub cursor_pos: u32,
    pub visible: bool,
}
impl dbus::arg::ReadAll for UpdatePreeditTextSignal {
    fn read(i: &mut dbus::arg::Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
        let text_var: Variant<Box<dyn RefArg>> = i.read()?;
        // Structs are represented internally as `VecDeque<Box<RefArg>>`.
        // According to:
        // https://github.com/diwic/dbus-rs/blob/174e8d55b0e17fb6fbd9112e5c1c6119fe8b431b/dbus/examples/argument_guide.md
        let text_struct: &VecDeque<Box<dyn RefArg>> = dbus::arg::cast(&text_var.0).unwrap();
        let text = text_struct[2].as_str().unwrap_or("").to_owned();
        let cursor_pos = i.read()?;
        let visible = i.read()?;
        Ok(UpdatePreeditTextSignal {
            text,
            cursor_pos,
            visible,
        })
    }
}
impl dbus::message::SignalArgs for UpdatePreeditTextSignal {
    const NAME: &'static str = "UpdatePreeditText";
    const INTERFACE: &'static str = "org.freedesktop.IBus.InputContext";
}
// ////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum Error {
    DBus(#[from] dbus::Error),
    Unknown {
        description: String
    }
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
        let addr = get_address().map_err(|e| Error::Unknown {description: e})?;
        let mut channel = dbus::channel::Channel::open_private(&addr)?;
        channel.register()?;
        Ok(Bus {
            conn: Arc::new(dbus::blocking::Connection::from(channel))
        })
    }

    pub fn create_input_context(&self, name: &str) -> Result<InputContext, Error> {
        let ibus = self.conn.with_proxy("org.freedesktop.IBus", "/org/freedesktop/IBus", REQ_TIMEOUT);
        let (obj_path,): (dbus::strings::Path,) = ibus.method_call("org.freedesktop.IBus", "CreateInputContext", (name,))?;

        Ok(InputContext {
            conn: self.conn.clone(),
            obj_path
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

pub struct InputContext {
    conn: Arc<dbus::blocking::Connection>,
    obj_path: dbus::strings::Path<'static>,
}
impl InputContext {
    pub fn set_capabilities(&self, caps: Capabilites) {
        self.with_proxy(|p| {
            let caps = caps.bits();
            let () = p.method_call("org.freedesktop.IBus.InputContext", "SetCapabilities", (caps,)).unwrap();
        })
    }

    pub fn on_commit_text<F>(&self, mut callback: F) -> Result<Token, Error>
    where
        F: FnMut(CommitTextSignal, &Connection, &Message) -> AfterCallback + Send + 'static
    {
        let token = self.with_proxy(|p| {
            p.match_signal(move |a: CommitTextSignal, b: &Connection, c: &Message| {
                (callback)(a, b, c).to_bool()
            })
        })?;
        Ok(token)
    }

    pub fn on_update_preedit_text<F>(&self, mut callback: F) -> Result<Token, Error>
    where
        F: FnMut(UpdatePreeditTextSignal, &Connection, &Message) -> AfterCallback + Send + 'static
    {
        let token = self.with_proxy(|p| {
            p.match_signal(move |a: UpdatePreeditTextSignal, b: &Connection, c: &Message| {
                (callback)(a, b, c).to_bool()
            })
        })?;
        Ok(token)
    }

    /// Returns:
    /// - `Ok(true)` if the call was handled succesfully
    /// - `Ok(false)` if the call was executed but it wasn't handled (this can for example happen when the capabilities aren't set correctly)
    /// - `Err(e)` if an error occured
    pub fn process_key_event(&self, sym: u32, code: u32, modifiers: u32) -> Result<bool, Error> {
        self.with_proxy(|p| {
            let key_args = (sym, code, modifiers);
            let (handled,): (bool,) = p.method_call("org.freedesktop.IBus.InputContext", "ProcessKeyEvent", key_args)?;
            Ok(handled)
        })
    }

    fn with_proxy<R, F: FnOnce(Proxy<&Connection>) -> R>(&self, f: F) -> R {
        let proxy = self.conn.with_proxy("org.freedesktop.IBus", &self.obj_path, REQ_TIMEOUT);
        f(proxy)
    }
}

pub fn asdf() {
    
    // dbus::Signature::new(s)
    
    // let (obj_path,): (dbus::strings::Path,) = ibus.method_call("org.freedesktop.IBus", "CreateInputContext", ("My Input Context".to_owned(),)).unwrap();
    // println!("obj path {:?}", obj_path);
    
    let caps = (Capabilites::PREEDIT_TEXT | Capabilites::FOCUS).bits();
    

    // let _ = ic_proxy.match_signal(|s: CommitTextSignal, _: &Connection, _: &Message| {
    //     println!("Received commited text: {}", s.text);
    //     true
    // }).unwrap();
    // let _ = ic_proxy.match_signal(|s: UpdatePreeditTextSignal, _: &Connection, _: &Message| {
    //     println!("Received preedit update: {}", s.text);
    //     true
    // }).unwrap();

    // let key_args: (u32, u32, u32) = (109, 50, 0);
    // let (_handled,): (bool,) = ic_proxy.method_call("org.freedesktop.IBus.InputContext", "ProcessKeyEvent", key_args).unwrap();
    // // println!("handled: {}", handled);
    // let key_args: (u32, u32, u32) = (117, 22, 0);
    // let (_handled,): (bool,) = ic_proxy.method_call("org.freedesktop.IBus.InputContext", "ProcessKeyEvent", key_args).unwrap();
    // // println!("handled: {}", handled);
    // let key_args: (u32, u32, u32) = (65293, 28, 0);
    // let (_handled,): (bool,) = ic_proxy.method_call("org.freedesktop.IBus.InputContext", "ProcessKeyEvent", key_args).unwrap();
    // // println!("handled: {}", handled);

    

    // let channel = conn.channel();
    // let watcher = conn.channel().watch();

    // println!("introspected\n{}", ibus.introspect().unwrap());
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        
    }
}
