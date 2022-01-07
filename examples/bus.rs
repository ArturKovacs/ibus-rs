
use ibus::{Bus, Capabilites};

fn bus_test() {
    let bus = Bus::new();
    let ic = bus.create_input_context().unwrap();
    ic.set_capabilities(Capabilites::PREEDIT_TEXT | Capabilites::FOCUS);
}

fn main() {
    ibus::asdf();
}
