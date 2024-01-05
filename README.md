## pulseaudio-rs

![tests](https://github.com/colinmarc/pulseaudio-rs/actions/workflows/tests.yaml/badge.svg) [![docs](https://img.shields.io/docsrs/pulseaudio)](https://docs.rs/pulseaudio/latest/pulseaudio/)

This is a native rust implementation of the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) protocol, suitable for writing clients and servers.

Currently implemented:

 - Types for server introspection ([example](examples/list-sinks.rs)), for listing sinks, sources, clients, etc.
 - Types for basic playback ([example](examples/playback.rs)) and record streams

Not yet implemented (but contributions welcome!)

 - A higher level `async`-friendly API
 - `memfd`/`shm` shenanigans for zero-copy streaming
 - Most other functionality
 