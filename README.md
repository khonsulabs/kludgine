# Kludgine

A 2d rendering engine for the [Rust](https://rust-lang.org/) language.

![Kludgine is considered experimental](https://img.shields.io/badge/status-experimental-blueviolet)
[![crate version](https://img.shields.io/crates/v/kludgine.svg)](https://crates.io/crates/kludgine)
[![Live Build Status](https://img.shields.io/github/workflow/status/khonsulabs/kludgine/Tests/main)](https://github.com/khonsulabs/kludgine/actions?query=workflow:Tests)
[![HTML Coverage Report for `main` branch](https://khonsulabs.github.io/kludgine/coverage/badge.svg)](https://khonsulabs.github.io/kludgine/coverage/)
[![Documentation for `main` branch](https://img.shields.io/badge/docs-main-informational)](https://khonsulabs.github.io/kludgine/main/kludgine/)

Kludgine is named in a way to hopefully be ironic in nature, but it's being designed and written by a developer that is fairly new to modern graphics programming and rust. Thus, it is probably a [kludge](https://en.wikipedia.org/wiki/Kludge).

## Why use Kludgine?

Well frankly, **you probably shouldn't right now**, because it's not even considered alpha.

If you are daring enough to use it before the [v0.1](https://github.com/khonsulabs/kludgine/projects/1) release, here are some of the features you can utilize:

## Fast and highly portable

Kludgine is built upon [easygpu](https://github.com/khonsulabs/easygpu), which is based on [wgpu](https://lib.rs/wgpu). `wgpu` is an experimental WebGPU implementation, and it supports DirectX on Windows, Vulkan on Linux, Metal on iOS/Mac OS, and OpenGL support is coming soon to allow for WebAssembly deployment.

Apps and games written in Kludgine should have no problem running with reasonable performance on budget hardware, even without a discrete GPU.

## Multi-window support

Kludgine is one of the only engines that has multi-window support. In general, most games don't need multiple windows, but general purpose applications do.

## Ease of use

This is a bit nebulous but examples include:

- Ergonomical and consistent API: This is a work in progress, but the goal is a
  consistent design that is easy to read and write.
- Easily include animated spritesheets from [Aseprite](https://www.aseprite.org).
- Modular design (work in progress): Kludgine started becoming a large engine, but in the process of creating [Gooey](https://github.com/khonsulabs/gooey), this crate became a much leaner rendering and windowing crate. It will continue evolving to allow it to be used standalone, within other wgpu applications, or as a frontend for Gooey.

## About

This is being developed by and for [Khonsu Labs](https://khonsulabs.com/) for [Cosmic Verge](https://github.com/khonsulabs/cosmicverge). I hope it will be useful to others as well.

This code is dual-licensed under the [MIT License](./LICENSE-MIT) and [Apache License 2.0](./LICENSE-APACHE). Fonts in this repository are not included by default, and are [solely licensed under the Apache License 2.0](./fonts/README.md).
