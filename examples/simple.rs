
use ibus::{Bus, Capabilites, AfterCallback};

fn main() {
    // This program sends three fake keypresses to the IME server: M, U, Return
    // When executing this with the Mozc Katakana layout active, it should print the following:
    // 
    // preedit: UpdatePreeditTextSignal { text: "", cursor_pos: 0, visible: false }
    // preedit: UpdatePreeditTextSignal { text: "ｍ", cursor_pos: 1, visible: true }
    // preedit: UpdatePreeditTextSignal { text: "ム", cursor_pos: 1, visible: true }
    // commit: CommitTextSignal { text: "ム" }

    let bus = Bus::new().unwrap();
    let ctx = bus.create_input_context("input ctx lel").unwrap();
    ctx.set_capabilities(Capabilites::PREEDIT_TEXT | Capabilites::FOCUS);

    ctx.on_update_preedit_text(|s, _, _| {
        println!("preedit: {:?}", s);
        AfterCallback::Keep
    }).unwrap();
    ctx.on_commit_text(|s, _, _| {
        println!("commit: {:?}", s);
        AfterCallback::Keep
    }).unwrap();

    // The `M` key
    ctx.process_key_event(109, 50, 0).unwrap();
    // The `U` key
    ctx.process_key_event(117, 22, 0).unwrap();
    // The `Return` key
    ctx.process_key_event(65293, 28, 0).unwrap();

    loop {
        match bus.try_process() {
            Ok(true) => {}
            _ => break,
        }
    }
}
