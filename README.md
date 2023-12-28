## pulseaudio-rs
![tests](https://github.com/github/docs/actions/workflows/tests.yml/badge.svg)

This is a native rust implementation of the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) protocol, suitable for writing clients and servers.

Currently implemented:

 - Types for server introspection ([example](examples/list-sinks.rs)), for listing sinks, sources, clients, etc.
 - Types for basic playback ([example](examples/playback.rs)) and record streams

Not yet implemented (but contributions welcome!)

 - A higher level `async`-friendly API
 - `memfd`/`shm` shenanigans for zero-copy streaming
 - Most other functionality
 