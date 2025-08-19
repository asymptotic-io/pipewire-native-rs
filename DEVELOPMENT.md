## Code structure

At the top-level, we have a native implementation implementation of the
[PipeWire native
protocol](https://docs.pipewire.org/devel/page_native_protocol.html). This is
then exposed via the API in `pipewire/`, the entrypoint for this crate.

Similar to the C version, the `spa/` crate implements low-level primitives for
the PipeWire library.

There is a native implementation for some primitives, such as pod, for data
serialisation/deserialisation. There are also associated traits and macros (in
`macros/`) to reduce boilerplate.

A hybrid strategy is used for SPA plugins (which provide basic features such as
logging, event loops and a system call API). The SPA interfaces are exposed as
Rust interfaces, for use by Rust code. The underlying implementations use the C
plugin under the hood, with the option to be replaced by a Rust implementation
in the future if desired.
