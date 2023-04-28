# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Breaking Changes

- `set_always_on_top`/`with_always_on_top`/`always_top` have been replaced with
  `set_window_level`/`with_window_level`/`window_level` respectively. This
  change was due to upgrading to the latest `winit`.

### Changes

- Updated `easygpu` to v0.5.0.
  - `wgpu` has been updated to v0.16.0
- `winit` has been updated to v0.28.3
- `palette` has been updated to v0.7.1

### Fixes

- Returning a scale from `Window::additional_scale` now works.
- `CloseResponse` is now exported.

### Added

- `Scene::set_additional_scale` has been added to set the scaling factor between
  Points and Scaled. This allows application-level scaling in addition to the
  DPI scaling Kludgine already does.

## v0.4.0

### Changes

- Updated `easygpu` to 0.4.0:
  - `wgpu` has been updated to 0.15.0.

## v0.3.1

### Changes

- Updated `rusttype` to 0.9.3:
  - `ttf-parser` has been updated to 0.15.2.
  - Versions of rusttype are now pinned to prevent transient dependency upgrades
    breaking compilation.

## v0.3.0

### Changes

- Updated `easygpu` to 0.3.0:
  - `wgpu` has been updated to 0.14.0.
  - `winit` has been updated to 0.27.4.

## v0.2.0

### Changes

- Updated `easygpu` to 0.2.0:
  - Updated `wgpu` to 0.13.1
  - Updated `lyon_tessellation` to 1.0.1

## v0.1.0

### Changes

- Switching off of pre-release version numbering. They just add more pain than
  they're worth.
- Updated dependencies to `wgpu` 0.12

### Fixes

- Fixed incompatibility with image crate update.
- Changed dependency versions to be less lenient.

## v0.1.0-dev.6

### Fixes

- Fixed issue where render_one_frame would freeze in headless environments (#53).

## v0.1.0-dev.5

### Changed

- Updated dependencies for compatability with wgpu 0.11.1.
- Implemented Sprite alpha rendering. The APIs already existed, but the alpha value was being ignored.

## v0.1.0-dev.4

### Added

- `Sprite::current_frame` immutably retrieves the current frame. This
  is equivalent to calling `Sprite::get_frame(None)` but can be used
  in non-mutable settings.

## v0.1.0-dev.3

### Changes

- Updated `easygpu` and `easygpu-lyon`, which moves Kludgine onto `wgpu` 0.11.

## v0.1.0-dev.2

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
