## pulseaudio-rs

![tests](https://github.com/colinmarc/pulseaudio-rs/actions/workflows/tests.yaml/badge.svg) [![docs](https://img.shields.io/docsrs/pulseaudio)](https://docs.rs/pulseaudio/latest/pulseaudio/)

This is a native rust implementation of the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) protocol, suitable for writing clients and servers.

Currently implemented:

 - Low-level serialization and deserialization of the wire format (called "tagstructs")

Not yet implemented (but contributions welcome!)

 - A higher level `async`-friendly API
 - `memfd`/`shm` shenanigans for zero-copy streaming

 Examples:

 - [Listing sinks](examples/list-sinks.rs)
 - [Subscribing to server events](examples/subscribe.rs)
 - [Playing an audio file](examples/playback.rs))
 - [Recording audio](examples/record.rs)
 - [Acting as a sound server](examples/server.rs)