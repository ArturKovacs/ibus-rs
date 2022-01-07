
## ibus

This crate implements the IBus protocol in pure Rust. The API provided is currently limited and only focuses on functionality that's important to clients.

The API more or less follows the structure of the IBus C API, but note that the two are not binary compatible. This means that it's *not* valid to pass a pointer to this crate's `Bus` object to the `ibus_bus_create_input_context` function. Luckily it shouldn't be needed either.

