# Kludgine

A 2d app and game engine for the [Rust](https://rust-lang.org/) language.

![Rust](https://github.com/khonsulabs/kludgine/workflows/Tests/badge.svg) [![codecov](https://codecov.io/gh/khonsulabs/kludgine/branch/main/graph/badge.svg)](https://codecov.io/gh/khonsulabs/kludgine)

Kludgine is named in a way to hopefully be ironic in nature, but it's being designed and written by a developer that is fairly new to modern graphics programming and rust. Thus, it is probably a [kludge](https://en.wikipedia.org/wiki/Kludge).

# Why use Kludgine?

Well frankly, **you probably shouldn't right now**, because it's not even considered alpha.

If you are daring enough to use it before the [v0.1](https://github.com/khonsulabs/kludgine/projects/1) release, here are some of the features you can utilize:

## Fully async Component-based rendering engine

Kludgine is a higher-level rendering engine, and its UI layout system is the same system you will use to write your apps or games in it. It mirrors more closely a traditional view system, but it leverages message passing to asynchronously communicate between components. To see the current API, see the [UI example](./examples/ui.rs).

## Fast and highly portable

Kludgine is built upon [rgx](https://lib.rs/rgx), which is based on [wgpu](https://lib.rs/wgpu). `wgpu` is an experimental WebGPU implementation, and it supports DirectX on Windows, Vulkan on Linux, Metal on iOS/Mac OS, and OpenGL support is coming soon to allow for WebAssembly deployment.

Apps and games written in Kludgine should have no problem running with reasonable performance on budget hardware, even without a discrete GPU.

## Multi-window support

Kludgine is one of the only engines that has multi-window support. Because a `Window` is a special type of Component, the long-term goal is to allow embedded "Windows" inside of another window, and allow them to be popped out easily on platforms that allow multiple windows. Multi-window support is available today, but the rest is a long-term goal.

## Ease of use

This is a bit nebulous but examples include:

- Ergonomical and consistent API: This is a work in progress, but the goal is a consistent design that is easy to read and write. The primary goal of v0.1 is to stabilize a reasonable API that fits this vision. Right now there is still a lot of work on the foundation, and the API is under constant revision.
- Easily include animated spritesheets from [Aseprite](https://www.aseprite.org)
- Font fallback system (with more support for Unicode-driven fallback choices eventually coming)
- Batteries-included input mapping system (to be developed) -- Easily respond to intent-based inputs intead of hard-coding key mappings, and a flexible system for providing UI hints for the user based on those intents.

# About

This is being developed by and for [Khonsu Labs](https://khonsulabs.com/) partially as a learning experience. I hope it will be useful to others eventually.

This code is [licensed](./README.md) under the [MIT](https://opensource.org/licenses/MIT) license. This code will eventually be dual-licensed under the Apache License v2.0. If this is an issue for you please contact [me](https://github.com/ecton), and I will prioritize it.
