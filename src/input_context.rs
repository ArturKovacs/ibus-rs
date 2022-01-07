use std::{collections::VecDeque, sync::Arc};

use dbus::{
    arg::{RefArg, Variant},
    blocking::{Connection, Proxy},
    channel::Token,
    Message,
};

use crate::{AfterCallback, Capabilites, Error, REQ_TIMEOUT};

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
            text: arg[2].as_str().unwrap_or("").to_owned(),
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

pub struct InputContext {
    pub(crate) conn: Arc<dbus::blocking::Connection>,
    pub(crate) obj_path: dbus::strings::Path<'static>,
}
impl InputContext {
    pub fn set_capabilities(&self, caps: Capabilites) {
        self.with_proxy(|p| {
            let caps = caps.bits();
            let () = p
                .method_call(
                    "org.freedesktop.IBus.InputContext",
                    "SetCapabilities",
                    (caps,),
                )
                .unwrap();
        })
    }

    pub fn on_commit_text<F>(&self, mut callback: F) -> Result<Token, Error>
    where
        F: FnMut(CommitTextSignal, &Connection, &Message) -> AfterCallback + Send + 'static,
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
        F: FnMut(UpdatePreeditTextSignal, &Connection, &Message) -> AfterCallback + Send + 'static,
    {
        let token = self.with_proxy(|p| {
            p.match_signal(
                move |a: UpdatePreeditTextSignal, b: &Connection, c: &Message| {
                    (callback)(a, b, c).to_bool()
                },
            )
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
            let (handled,): (bool,) = p.method_call(
                "org.freedesktop.IBus.InputContext",
                "ProcessKeyEvent",
                key_args,
            )?;
            Ok(handled)
        })
    }

    fn with_proxy<R, F: FnOnce(Proxy<&Connection>) -> R>(&self, f: F) -> R {
        let proxy = self
            .conn
            .with_proxy("org.freedesktop.IBus", &self.obj_path, REQ_TIMEOUT);
        f(proxy)
    }
}
