# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `WindowCreator` and `WindowBuilder` now support initial_position.

### Breaking Changes

- `WindowCreator` and `WindowBuilder` now use `Points` as the unit for
  `initial_size`. When creating the window, we now request the logical size
  rather than pixels, correspondingly.

### Fixes

- Redrawing while resizing is done with more expediency. Previously, we were
  waiting for the OS to ask for us to redraw after resizing, rather than forcing
  a resize.

## v0.1.0-dev.1

### Breaking Changes

- Added `WindowHandle`, which allows interacting with the window after it has
  been built. This parameter is passed into nearly all `Window` trait functions
  now.
- `WindowCreator` now takes `&self` parameter for all methods. There was no
  reason for these methods to be static, and it prevented a window from being
  able to control how it was built based on its initial configuration.

### Fixes

- Rendering a SpriteSource using a Point without specifying a Size now renders
  it at `Scaled` resolution. This restores the behavior before the parameters
  were switched to `Displayable`.

## v0.1.0-dev.0

### Breaking Changes

- Removed all user interface code, and spun off a new user interface project,
  [Gooey](https://github.com/khonsulabs/gooey).
- Split Kludgine into three crates:
  - `kludgine-core`: The rendering aspects of Kludgine. Can now be used for headless rendering as well.
  - `kludgine-app`: The windowing/event handling layer of Kludgine.
  - `kludgine`: An omnibus crate that marries the two with one crate include.
- Now uses `figures` for its math types. If you're using functionality that was
  in `euclid` but is no longer available in `figures`, please submit [an
  issue](https://github.com/khonsulabs/figures/issues). We may not add all
  requested functionality, but as long as it extends one of the types `figures`
  already has, it likely will be added upon request.
- Introduced `unstable-apis` feature flag. The plan for this flag is to offer a
  way to provide APIs that are still under heavy development to be used without
  forcing semver updates when the APIs change. After 1.0, breaking changes to
  `unstable-apis` will be one of the factors that causes minor version
  increments.
