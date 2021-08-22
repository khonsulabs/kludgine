# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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

