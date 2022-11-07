# Kludgine

A 2d rendering engine for the [Rust](https://rust-lang.org/) language.

![Kludgine is considered experimental](https://img.shields.io/badge/status-experimental-blueviolet)
[![crate version](https://img.shields.io/crates/v/kludgine.svg)](https://crates.io/crates/kludgine)
[![Live Build Status](https://img.shields.io/github/workflow/status/khonsulabs/kludgine/Tests/main)](https://github.com/khonsulabs/kludgine/actions?query=workflow:Tests)
[![HTML Coverage Report for `main` branch](https://khonsulabs.github.io/kludgine/coverage/badge.svg)](https://khonsulabs.github.io/kludgine/coverage/)
[![Documentation for `main` branch](https://img.shields.io/badge/docs-main-informational)](https://khonsulabs.github.io/kludgine/main/kludgine/)

Kludgine is named in a way to hopefully be ironic in nature, but it's being
designed and written by a developer that is fairly new to modern graphics
programming and rust. Thus, it is probably a
[kludge](https://en.wikipedia.org/wiki/Kludge).

## Why use Kludgine?

Kludgine is early in development and is subject to breaking API changes. That
being said, the API has started to stabilize, and these are the primary benefits
of Kludgine:

### Fast and highly portable

Kludgine is built upon [easygpu](https://github.com/khonsulabs/easygpu), which
is based on [wgpu](https://lib.rs/wgpu). `wgpu` is an experimental WebGPU
implementation, and it supports DirectX on Windows, Vulkan on Linux, Metal on
iOS/Mac OS, and is close to working within the web browser. Kludgine does not
yet currently work inside of the web browser, but we eventually will support it.

Apps and games written in Kludgine should have no problem running with
reasonable performance on budget hardware, even without a discrete GPU.

### Intelligent Redrawing

While most games want a steady framerate, many developers may want to allow
their programs to not redraw constantly. Kludgine makes managing this easy with
the
[`RedrawStatus`](https://khonsulabs.github.io/kludgine/main/kludgine/app/struct.RedrawStatus.html)
type, allowing easy scheduling of future frame redraws or immediately requesting
a redraw.

If you do want a consistent redraw, implement
[`Window::target_fps`](https://khonsulabs.github.io/kludgine/main/kludgine/app/trait.Window.html#method.target_fps).

### Multi-window support

Kludgine has multi-window support. In general, most games don't need multiple
windows, but general purpose applications do.

### Ease of use

This is a bit nebulous but examples include:

- Ergonomic and consistent API: This is a work in progress, but the goal is a
  consistent design that is easy to read and write.
- Modular design: Kludgine started becoming a large engine, but in the process
  of creating [Gooey](https://github.com/khonsulabs/gooey), this crate became a
  much leaner rendering and windowing crate. It will continue evolving to allow
  it to be used standalone, within other wgpu applications, or as a frontend for
  Gooey.

```rust
let texture = Texture::load("kludgine/examples/assets/k.png")?;
let sprite = SpriteSource::entire_texture(texture);
sprite.render_at(
    scene,
    Rect::from(scene.size()).center(),
    SpriteRotation::around_center(self.rotation_angle),
);
```

To see examples on how to use various features of Kludgine, see the
[kludgine/examples][examples] folder in the repository. If you are having
issues, make sure you're looking at examples for the correct version of
Kludgine. If you're using a released version, switch to viewing the repository
at that version's tag.

## Getting Started

To use Kludgine, add it to your Cargo.toml:

```sh
cargo add kludgine
```

## About

This is being developed by and for [Khonsu Labs](https://khonsulabs.com/) for
[Cosmic Verge](https://github.com/khonsulabs/cosmicverge). I hope it will be
useful to others as well.

This code is dual-licensed under the [MIT License](./LICENSE-MIT) and [Apache
License 2.0](./LICENSE-APACHE). Fonts in this repository are not included by
default, and are [solely licensed under the Apache License
2.0](./fonts/README.md).

[examples]: https://github.com/khonsulabs/kludgine/tree/main/kludgine/examples
