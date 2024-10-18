# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Breaking Changes

- `Window::set_inner_size` has been replaced with `Window::request_inner_size`.
  This function now matches winit's underlying `request_inner_size` behavior.
  The function returns an option containing the new size if the size was able to
  be applied before the function returns.

  This new function properly updates the `inner_size` and `outer_size` when the
  underlying window is resized immediately. Notably, this happens on Wayland but
  may happen on some other platforms as well.

  Users who are using Kludgine directly should invoke `Kludgine::resize` to
  apply the new size.
- `Window::winit` now returns an `Arc`-wrapped winit Window.
- `WindowBehavior::render` no longer returns a `bool`. Closing the window can be
  done through the `RunningWindow` parameter.

### Added

- `StrokeOptions::upx_wide` returns stroke options for a given unsigned pixel
  stroke width.
- `LazyTexture`, `CollectedTexture`, `AnyTexture`, `TextureCollection`, and
  `Texture` now implement `PartialEq`.
- `PendingApp::on_unrecoverable_error` is a new function that allows
  applications to take control when an unrecoverable error occurs. Previously,
  these errors would panic, and the provided implementation is a panic.
- `App::execute` executes a closure on the main event loop thread.
- Shape has several new functions to create textured shapes:

  - `Shape::textured_round_rect`
  - `Shape::textured_rect`
  - `Shape::textured_circle`

### Fixed

- Plotters integration now strokes paths offsetting by half of the stroke width
  to ensure proper subpixel alignment.
- Plotters integration text drawing now honors anchor positioning, rotation, and
  properly sets the line height based.

## v0.11.0 (2024-09-14)

### Breaking Changes

- `App` is a new type that replaces the previous type alias pointing to
  `appit::App`. The previously exported type wasn't able to be used beyond
  passing it as a parameter for opening additional windows. This new type
  exposes additional winit information including monitor configurations.
- `Window::ocluded` has had its spelling fixed and is now `Window::occluded`.
- `Window::position` has been renamed to `Window::outer_position`
- `Window::set_position` has been renamed to `Window::set_outer_position`.

### Fixed

- `Window` now calls winit's `pre_present_notify()` before presenting the
  surface.
- `WindowHandle`'s `Clone` implementation no longer requires its generic
  parameter to implement `Clone`.
- Temporarily worked around a Wayland-only issue where window resize events are
  not being generated from explicit window sizing requests.

### Added

- `PendingApp::on_startup` executes a callback once the event loop has begun
  executing.
- `Monitors`, `Monitor`, and `VideoMode` are new types that offer information
  about the monitor configurations available to the application. This
  information can be retrieved from an `App` or `ExecutingApp`.
- `WindowBehavior::moved` is invoked when the window is repositioned.
- `Window::outer_size` is a new function that returns the window's size
  including decorations.
- `Window::inner_position` returns the position of the top-left of the content
  area of the window.
- `App::prevent_shutdown()` returns a guard that prevents the application from
  closing automatically when the final window is closed.
- `WindowBehavior::initialized` is invoked once the window has been fully
  initialized.
- `WindowBehavior::pre_initialize` is invoked before wgpu is initialized on the
  window.

## v0.10.0 (2024-08-20)

### Breaking Changes

- Added Zoom setting to [`Kludgine`], allowing a second scaling factor to be
  applied to all scaled operations. This change has affected these APIs:

  - [`Kludgine::resize()`]: Now takes an additional parameter `zoom`.
  - [`Kludgine::scale()`]: Now returns an effective scale combining zoom and DPI
        scaling.
  - [`Kludgine::dpi_scale()`]: A new function returning the currently set DPI
        scale.
  - [`Kludgine::zoom()`]: A new function returning the current zoom value.
  - [`Kludgine::set_zoom()`]: A new function setting just the zoom value.
  - [`Kludgine::set_dpi_scale()`]: A new function setting just the DPI scale.
  - [`Graphics::set_zoom()]: A new function setting the zoom level for a
        graphics context.

### Added

- `CornerRadii` now implements `figures::Round`.

## v0.9.0 (2024-07-22)

### Breaking Changes

- `wgpu` has been updated to `22.0.0`.
- `cosmic-text` has been updated to `0.12.0`.

### Added

- `WindowBehavior::memory_hints` is a new trait function that controls the
  memory hints `wgpu` is initialized with. The provided implementation returns
  `wgpu::MemoryHints::default()`.

## v0.8.0 (2024-05-12)

### Breaking Changes

- `Frame::render_into` no longer takes a `Graphics` parameter, but instead
  accepts the `wgpu::Queue` and `wgpu::Device` parameters directly. Using
  `Graphics` causes lifetime issues in some rendering workflows.
- The `render` module has been renamed to `drawing` to match the type it
  contains. The old name was a remnant from when `Drawing` used to be named
  `Rendering`, which was incredibly confusing with `Renderer` types around as
  well.
- This crate now supports `wgpu` 0.20.0.
- This crate now supports `cosmic-text` 0.11.2.
- This crate now supports `image` 0.25.1.
- These `WindowBehavior` functions have had a `&Self::Context` parameter added
  to them, ensuring each function where Kludgine is requesting information from
  the implementor either receives an `&self` or an `&Self::Context`:

  - `WindowBehavior::power_preference()`
  - `WindowBehavior::limits()`
  - `WindowBehavior::multisample_count()`
- `SpriteSheet::new()` now takes an additional parameter: `gutter_size`. Passing
  `Size::ZERO` will cause the returned sprite sheet to be the same as before
  this change.

  This new parameter allows using sprite sheets that have been exported with
  consistent spacing between each sprite.
- These APIs now require exclusive references to the application:
  - `WindowBehavior::open`
  - `WindowBehavior::open_with`
- These events have been renamed to match `winit`'s nomenclature:
  - `WindowBehavior::touchpad_magnify` -> `WindowBehavior::pinch_gesture`
  - `WindowBehavior::smart_magnify` -> `WindowBehavior::double_tap_gesture`

### Changed

- All `&Appplication` bounds now are `?Sized`, enabling `&dyn Application`
  parameters.
- `Color` now exposes its inner `u32` as public.

### Added

- `WindowBeahvior::pan_gesture` is a new event provided by `winit`.
- `Kludgine::id()` returns the instance's unique id.
- `Kludgine::REQUIRED_FEATURES` specifies the `wgpu::Features`` that Kludgine uses.
- `Kludgine::adjust_limits()` adjusts `wgpu::Limits` to ensure Kludgine will
  function.
- `Texture::multisampled` allows creating a `Texture` that can be used as a
  multisample render attachment.
- `Texture::copy[_rect]_to_buffer` are convenience helpers for copying image
  data to a `wgpu::Buffer`.
- `Texture::wgpu()` returns a handle to the underlying `wgpu::Texture`.
- `Texture::view()` returns a `wgpu::TextureView` for the entire texture.
- A new feature `plotters` enables integration with the excellent
  [plotters][plotters] crate. `Renderer::as_plot_area()` is a new function that
  returns a `plotters::DrawingArea`.
- `Kludgine::rebuild_font_system()` is a new function that recreates the
  `cosmic_text::FontSystem`, which has the net effect of clearing font-database
  related caches.
- `WindowBehavior::present_mode()` allows a window to pick a different
  presentation mode. The default implementation returns
  `wgpu::PresentMode::AutoVsync`.

[plotters]: https://github.com/plotters-rs/plotters

### Fixed

- `Drawing::render()` no longer makes any assumptions about the current clipping
  rectangle when drawing is started.
- Color correction for Srgb is now being done more accurately using the
  `palette` crate. This affects colors being applied to textures as tints and
  shape drawing, but the Srgb handling of textures themselves remain handled
  purely by wgpu.
- Drawing text with an empty first line no longer panics.

## v0.7.0

### Breaking Changes

- `UnwindSafe` has been removed from the bounds of `WindowBehavior::Context`,
  and various types may or may no longer implmement `UnwindSafe`. The underlying
  requirement for this has been removed from `appit`.
- `Texture::lazy_from_data` and `Texture::lazy_from_image` have been refactored
  into constructors of a new type:
  `LazyTexture::from_data`/`LazyTexture::from_image`.
- `include_texture!` now returns a `LazyTexture` instead of a `Texture`.
- `SharedTexture::region()` has been removed. `TextureRegion::new()` is the
  replacement API that allows creating a region for any `ShareableTexture`.
- `Sprite::load_aseprite_json` now accepts `impl Into<ShareableTexture>` instead
  of `&SharedTexture`. In theory, no code will break from this change due to
  trait implementations.
- `SpriteSheet::texture` is now a `ShareableTexture`.

## Added

- `app::PendingApp` is a type that allows opening one or more windows before
  running an application.
- `app::App` is a handle to a running application.
- `app::Window::app()` returns a handle to the application of the window.
- `WindowBehavior::open[_with]()` are new functions that allow opening a window
  into a reference of an `App` or `PendingApp`.
- `CanRenderTo::can_render_to()` is a new trait that checks if a resource can be
  rendered in a given `Kludgine` instance.

  This is implemented by all types in Kludgine that utilize textures.
- `LazyTexture` is a new texture type that supports being shared across
  different windows/wgpu rendering contexts by loading its data on-demand.
  `LazyTexture::upgrade()` loads a `SharedTexture` that is compatible with the
  given graphics context.
- `ShareableTexture` is a texture type that can resolve to a `SharedTexture`.
  Currently this is either a `SharedTexture` or a `LazyTexture`.
- `RunningWindow::close` is allows closing a window.

## Fixed

- Internally, text drawing now uses weak references for the glyph handles to
  prevent `wgpu` resources from being freed if a `MeasuredText` was being held.
- Each window now has its own `wgpu::Instance` instead of sharing a single
  instance between windows.
- Each `Window` frame now waits until it's fully rendered before yielding back
  to the windqow loop.
- When a `Window` has a 0 dimension, it will no longer try to redraw. This
  presented as a panic when a window was minimized.
- Subpixel alignment of text rendering is no longer accounted for twice. Additionally,
  `TextOrigin::Center` rounds the offset calculated to the nearest whole pixel.
- A workaround has been added that caused clipping on the OpenGL backend to only
  use the final scissor rect.

## v0.6.1 (2023-12-19)

### Fixed

- A panic "Unsupported uniform datatype! 0x1405" has been resolved that occurred
  on some devices where push constants were being emulated and only signed
  integers were unspported.
- [#66][66]: A rounding error has been fixed when calculating the text width
      would cause the line width to be rounded down in some cases.
- If `wgpu` reports a `SurfaceError::Lost`, the `create_surface()` call is now
  correctly made on the main thread rather than the window thread. Thanks to
  [@Plecra][plecra] for reviewing the unsafe code and noticing this issue. This
  review also led to further reductions in the amount of unsafe code and
  improved the safety comments.

[66]: https://github.com/khonsulabs/kludgine/pull/66
[plecra]: https://github.com/Plecra

## v0.6.0 (2023-12-18)

This version is a complete rewrite. While some code was copied across, this
library now directly depends upon `wgpu` instead of using `easygpu`, and it has
an API inspired by `wgpu`'s [Encapsulating Graphics Work][encapsulating]
article.

[encapsulating]: https://github.com/gfx-rs/wgpu/wiki/Encapsulating-Graphics-Work

## v0.5.0 (2023-04-28)

### Breaking Changes

- `set_always_on_top`/`with_always_on_top`/`always_top` have been replaced with
  `set_window_level`/`with_window_level`/`window_level` respectively. This
  change was due to upgrading to the latest `winit`.
- The MSRV has been updated to 1.64.0 due to nested dependency requirements.
- These feature flags have been renamed:
  - `serialization` has become `serde`
  - `tokio-rt` has become `tokio`
  - `smol-rt` has become `smol`

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

## v0.4.0 (2023-01-27)

### Changes

- Updated `easygpu` to 0.4.0:
  - `wgpu` has been updated to 0.15.0.

## v0.3.1 (2022-11-06)

### Changes

- Updated `rusttype` to 0.9.3:
  - `ttf-parser` has been updated to 0.15.2.
  - Versions of rusttype are now pinned to prevent transient dependency upgrades
    breaking compilation.

## v0.3.0 (2022-11-06)

### Changes

- Updated `easygpu` to 0.3.0:
  - `wgpu` has been updated to 0.14.0.
  - `winit` has been updated to 0.27.4.

## v0.2.0 (2022-07-31)

### Changes

- Updated `easygpu` to 0.2.0:
  - Updated `wgpu` to 0.13.1
  - Updated `lyon_tessellation` to 1.0.1

## v0.1.0 (2022-02-02)

### Changes

- Switching off of pre-release version numbering. They just add more pain than
  they're worth.
- Updated dependencies to `wgpu` 0.12

### Fixes

- Fixed incompatibility with image crate update.
- Changed dependency versions to be less lenient.

## v0.1.0-dev.6 (2021-12-07)

### Fixes

- Fixed issue where render_one_frame would freeze in headless environments (#53).

## v0.1.0-dev.5 (2021-12-06)

### Changed

- Updated dependencies for compatability with wgpu 0.11.1.
- Implemented Sprite alpha rendering. The APIs already existed, but the alpha value was being ignored.

## v0.1.0-dev.4 (2021-10-31)

### Added

- `Sprite::current_frame` immutably retrieves the current frame. This
  is equivalent to calling `Sprite::get_frame(None)` but can be used
  in non-mutable settings.

## v0.1.0-dev.3 (2021-10-11)

### Changes

- Updated `easygpu` and `easygpu-lyon`, which moves Kludgine onto `wgpu` 0.11.

## v0.1.0-dev.2 (2021-10-11)

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

## v0.1.0-dev.1 (2021-09-01)

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

## v0.1.0-dev.0 (2021-08-22)

### Breaking Changes

- Removed all user interface code, and spun off a new user interface project,
  [Cushy](https://github.com/khonsulabs/cushy).
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
