# pipewire-native-rs

This is a native implementation of the [PipeWire](https://pipewire.org) client
library in Rust. The primary objective is to provide a safe, idiomatic API for
PipeWire clients, with a secondary goal of providing a C wrapper for clients in
other languages to benefit from the safety guarantees in the longer term.

This project and draws inspiration from other efforts like the current Rust
bindings in [pipewire-rs](https://gitlab.freedesktop.org/pipewire/pipewire-rs)
and the
[pipewire-native-protocol](https://github.com/Troels51/pipewire-native-protocol)
implementation. The goal is for these bindings to eventually be the official
PipeWire Rust API.

Being a work-in-progress, the API will likely change as we iterate. When things
are more stable, a release will be made to [crates.io](https://crates.io) and
[docs.rs](https://docs.rs).
