# Kludgine

![Kludgine is considered alpha and unsupported](https://img.shields.io/badge/status-alpha-orange)
[![crate version](https://img.shields.io/crates/v/kludgine.svg)](https://crates.io/crates/kludgine)
[![Documentation for `main` branch](https://img.shields.io/badge/docs-main-informational)](https://khonsulabs.github.io/kludgine/main/kludgine)

Kludgine aims to be a lightweight, efficient 2d rendering framework powered by
[wgpu][wgpu]. Its name Kludgine is named in a way to hopefully be ironic in
nature, but it's being designed and written by a developer that was fairly new
to modern graphics programming and Rust. Thus, it is probably a
[kludge][kludge].

Without the `app` feature enabled, Kludgine provides an API inspired by wgpu's
[Encapsulating Graphics Work][encapsulating] article.

With the `app` feature enabled, Kludgine provides an easy-to-use API for running
multi-window applications.

The API is still a work in progress. The [`examples`][examples] folder contains
many examples that highlight a specific feature.

## Project Status

This project is early in development as part of [Gooey][gooey]. It is considered
alpha and unsupported at this time, and the primary focus for [@ecton][ecton] is
to use this for his own projects. Feature requests and bug fixes will be
prioritized based on @ecton's own needs.

If you would like to contribute, bug fixes are always appreciated. Before
working on a new feature, please [open an issue][issues] proposing the feature
and problem it aims to solve. Doing so will help prevent friction in merging
pull requests, as it ensures changes fit the vision the maintainers have for
Gooey.

[gooey]: https://github.com/khonsulabs/gooey
[ecton]: https://github.com/khonsulabs/ecton
[issues]: https://github.com/khonsulabs/gooey/issues

[wgpu]: https://github.com/gfx-rs/wgpu
[kludge]: https://en.wikipedia.org/wiki/Kludge
[encapsulating]: https://github.com/gfx-rs/wgpu/wiki/Encapsulating-Graphics-Work
[examples]: https://github.com/khonsulabs/kludgine/tree/main/examples
