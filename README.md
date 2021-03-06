# Kludgine

A 2d app and game engine for the [Rust](https://rust-lang.org/) language.

![Rust](https://github.com/khonsulabs/kludgine/workflows/Tests/badge.svg) [![codecov](https://codecov.io/gh/khonsulabs/kludgine/branch/main/graph/badge.svg)](https://codecov.io/gh/khonsulabs/kludgine)

Kludgine is named in a way to hopefully be ironic in nature, but it's being designed and written by a developer that is fairly new to modern graphics programming and rust. Thus, it is probably a [kludge](https://en.wikipedia.org/wiki/Kludge).

## Why use Kludgine?

Well frankly, **you probably shouldn't right now**, because it's not even considered alpha.

If you are daring enough to use it before the [v0.1](https://github.com/khonsulabs/kludgine/projects/1) release, here are some of the features you can utilize:

## About versioning

_Before v0.1, breaking changes will regularly happen._ In the process of preparing to release 0.1, a process for marking features as stable or unstable will be added. The version 0.1 milestone will be reached when a large enough core set of stable APIs has been reached to make such a milestone meaningful.

Once that process is determined, it will be applied to the versioning scheme this way:

- Major version updates: Massive fundamental architecture changes that require most users to carefully consider upgrading.
- Minor version updates: One or more stable features has breaking changes to it under expected use cases.
- Patch version updates: All other updates

As this project matures, a changelog will eventually be added.

## Fully async Component-based rendering engine

Kludgine is a higher-level rendering engine, and its UI layout system is the same system you will use to write your apps or games in it. It mirrors more closely a traditional view system, but it leverages message passing to asynchronously communicate between components. To see the current API, see the [UI example](./examples/ui.rs).

## Fast and highly portable

Kludgine is built upon [easygpu](https://github.com/khonsulabs/easygpu), which is based on [wgpu](https://lib.rs/wgpu). `wgpu` is an experimental WebGPU implementation, and it supports DirectX on Windows, Vulkan on Linux, Metal on iOS/Mac OS, and OpenGL support is coming soon to allow for WebAssembly deployment.

Apps and games written in Kludgine should have no problem running with reasonable performance on budget hardware, even without a discrete GPU.

## Multi-window support

Kludgine is one of the only engines that has multi-window support. Because a `Window` is a special type of Component, the long-term goal is to allow embedded "Windows" inside of another window, and allow them to be popped out easily on platforms that allow multiple windows. Multi-window support is available today, but [the rest is a long-term goal](https://github.com/khonsulabs/kludgine/issues/29).

## Ease of use

This is a bit nebulous but examples include:

- Ergonomical and consistent API: This is a work in progress, but the goal is a consistent design that is easy to read and write. The primary goal of v0.1 is to stabilize a reasonable API that fits this vision. Right now there is still a lot of work on the foundation, and the API is under constant revision.
- Easily include animated spritesheets from [Aseprite](https://www.aseprite.org)
- Font fallback system (with more support for Unicode-driven fallback choices [eventually coming](https://github.com/khonsulabs/kludgine/issues/28))
- Batteries-included input mapping system ([to be developed](https://github.com/khonsulabs/kludgine/issues/27)) -- Easily respond to intent-based inputs intead of hard-coding key mappings, and a flexible system for providing UI hints for the user based on those intents.

## About

This is being developed by and for [Khonsu Labs](https://khonsulabs.com/) partially as a learning experience. I hope it will be useful to others eventually.

This code is dual-licensed under the [MIT License](./LICENSE-MIT) and [Apache License 2.0](./LICENSE-APACHE). Fonts in this repository are not included by default, and are [solely llicensed under the Apache License 2.0](./fonts/README.md).
