# Kludgine

Kludgine is named in a way to hopefully be ironic in nature, but it's being designed and written by a developer that is fairly new to modern 3d graphics and rust. Thus, it is probably a [kludge](https://en.wikipedia.org/wiki/Kludge).

Kludgine is a Rust library built on [glium](https://lib.rs/glium) and [futures](https://lib.rs/futures). The goal is to expose a high-level, fully async/await API for rendering graphics using OpenGL.

For the forseeable future, this engine will be focused on graphics. If there's a good reason to bring anything else directly in, I will try to keep it modular or opt-in.

This is being developed by and for [Khonsu Labs](https://khonsulabs.com/) partially as a learning experience. I hope it will be useful to others eventually.

This code is [licensed](./README.md) under the [MIT](https://opensource.org/licenses/MIT) license.
