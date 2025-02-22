## pulseaudio-rs

![tests](https://github.com/colinmarc/pulseaudio-rs/actions/workflows/tests.yaml/badge.svg) [![docs](https://img.shields.io/docsrs/pulseaudio)](https://docs.rs/pulseaudio/latest/pulseaudio/)

This is a native rust implementation of the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) protocol, suitable for writing clients and servers.

Currently implemented:

 - Low-level serialization and deserialization of the wire format (called "tagstructs")
 - A higher level `async`-friendly API

Not yet implemented (but contributions welcome!)

 - `memfd`/`shm` shenanigans for zero-copy streaming

 Examples:

 - [Listing sinks](examples/list-sinks.rs)
 - [Subscribing to server events](examples/subscribe.rs)
 - [Playing an audio file](examples/playback.rs) and the [async version](examples/playback_async.rs)
 - [Recording audio](examples/record.rs) and the [async version](examples/record_async.rs)
 - [Acting as a sound server](examples/server.rs)
