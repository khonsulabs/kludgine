# Kludgine (Redux)

This branch is a rewrite of Kludgine. See the [v0.5.0 tag][v0.5] for the
currently released source.

Kludgine aims to be a lightweight, efficient 2d rendering framework powered by
[wgpu][wgpu]. Its name Kludgine is named in a way to hopefully be ironic in
nature, but it's being designed and written by a developer that is fairly new to
modern graphics programming and rust. Thus, it is probably a [kludge][kludge].

Without the `app` feature enabled, Kludgine provides an API inspired by wgpu's
[Encapsulating Graphics Work][encapsulating] article.

TODO create an embedded wgpu example

With the `app` feature enabled, Kludgine provides an easy-to-use API for running
multi-window applications.

The API is still a work in progress. The [`examples`][examples] folder contains many
examples that highlight a specific feature.

[v0.5]: https://github.com/khonsulabs/kludgine/tree/v0.5.0
[wgpu]: https://github.com/gfx-rs/wgpu
[kludge]: https://en.wikipedia.org/wiki/Kludge
[encapsulating]: https://github.com/gfx-rs/wgpu/wiki/Encapsulating-Graphics-Work
[examples]: https://github.com/khonsulabs/kludgine/tree/main/examples
