# geoxi

Ergonomic geometry API on top of GEOS.

## Linking GEOS

```toml
[dependencies]
geoxi = "0.2"
```

`geoxi` downloads, checksum-verifies, and builds GEOS itself, so `cargo build`
works on a clean machine with no preinstalled GEOS, `pkg-config`, or vcpkg.
CMake and a C++ compiler are needed at _build_ time only.

### Licensing: dynamic vs. static

GEOS is licensed under **LGPL-2.1**; `geoxi` itself is Apache-2.0. Which way GEOS
is linked decides what you owe when you distribute a binary:

| Feature             | Linking                                                        | Consequence                                                                                                                                       |
| ------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dynamic` (default) | GEOS built as a shared library (`geos_c.dll` / `libgeos_c.so`) | Ship the shared libraries next to your binary with the LGPL text and a source offer. Users can swap GEOS; your own code stays under your licence. |
| `static`            | GEOS linked into your binary                                   | The LGPL's relinking terms apply to the whole binary. Intended for open-source or internal use. A build warning is printed.                       |
| `system`            | An installed GEOS found via `GEOS_DIR`, vcpkg, or pkg-config   | Same as whatever that installation is (shared is recommended).                                                                                    |

`static` and `dynamic` are mutually exclusive:

```toml
geoxi = { version = "0.2", default-features = false, features = ["static"] }
```

See `THIRD-PARTY-NOTICES.md`. This is not legal advice.

The directory holding the shared libraries is exported to dependents' build
scripts as `DEP_GEOXI_GEOS_BINDIR` (Windows DLLs) and `DEP_GEOXI_GEOS_LIBDIR`,
so a packaging step can copy them next to the executable. `cargo run` and
`cargo test` find them automatically: the shared libraries are staged inside the
target directory, the only place Cargo adds to the loader path.

To ship an application, add `woodshed` as a build-dependency and call
`woodshed::emit_exe_rpath()` from your `build.rs` (Linux and macOS executables then
load libraries from their own directory; Windows always does), and distribute the
executable together with the library files from the two directories above.

### Shipping a static binary

(Applies to the `static` feature.) GEOS is C++, so statically linking GEOS alone does not make an executable
portable - the C++ _runtime_ is a separate dependency. On Windows an ordinary
release build still requires `msvcp140.dll` and `vcruntime140.dll` from the MSVC
redistributable on the target machine.

`geoxi` handles this for you. Its build script reads the target's CRT setting and
configures GEOS's C++ runtime to match, so the two can never disagree. If your
project opts into a static CRT - the usual way is a checked-in
`.cargo/config.toml` -

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

then `cargo build --release` produces an executable that links only OS libraries
and runs on any machine of that target.

Switching CRT modes triggers a full GEOS rebuild; the two variants are cached
separately.

Provisioning is handled by [`woodshed`](https://crates.io/crates/woodshed), which caches the built
GEOS outside `OUT_DIR` so it survives `cargo clean` and `build.rs` edits.

- Cache: `$WOODSHED_CACHE` (default `$CARGO_HOME/woodshed`).
- Offline: `WOODSHED_OFFLINE=1` forbids downloads and errors clearly if the
  archive is missing.

### Using a system GEOS

Enable the `system` feature to link an existing installation instead:

```toml
geoxi = { version = "0.2", features = ["system"] }
```

The build script looks at `GEOS_DIR`, then vcpkg, then pkg-config, and falls
back to building from source if none yields a usable GEOS. This is opt-in
because it reintroduces the deployment dependency the default avoids.

## Layout

| Module         | Contents                                                 |
| -------------- | -------------------------------------------------------- |
| `error`        | `Error` and the crate-wide `Result` alias                |
| `types`        | `Coord`, `Coord3`, `Bounds`, `GeometryType`, `CoordType` |
| `context`      | `Context`, the GEOS thread handle                        |
| `geometry`     | `Geometry`, the owned handle: lifecycle and I/O          |
| `options`      | Parameter structs such as `BufferOptions`                |
| `prepared`     | `PreparedGeometry` for repeated predicate evaluation     |
| `array`        | `GeometryArray` batch operations                         |
| `strtree`      | `STRtree` spatial index                                  |
| `constructors` | Constructors and WKT/WKB/GeoJSON I/O                     |
| `measure`      | Area, length, and distance measurements                  |
| `predicates`   | Binary spatial predicates                                |
| `affinity`     | Affine transforms and coordinate rewriting               |
| `ops`          | Unary ops, set ops, linear referencing, coverages        |
| `topology`     | Splitting, orientation, validation, equality             |
| `accessors`    | Scalar accessors                                         |
| `coords`       | Coordinate extraction and GeoArrow ragged arrays         |
| `crs`          | Reprojection against any coordinate transform engine     |
| `three_d`      | Native 3D algorithms GEOS does not provide               |
| `coord_like`   | The `CoordLike` trait: use any coordinate type anywhere  |
| `buffers`      | Interleaved `f64` buffers, the zero-copy bulk path       |
| `geo_types`    | Whole-geometry `geo-types` conversion (feature-gated)    |

Public items are re-exported at the crate root, so `use geoxi::Geometry` works
regardless of which module an item lives in. `geoxi::prelude::*` carries the
everyday subset.

## CRS interop

`geoxi` never depends on a specific PROJ binding. Instead, implement the
`CoordTransform` trait for your engine. The trait hands over whole coordinate
slices so a PROJ binding can make one `proj_trans_generic` call per batch rather
than paying per-point FFI cost.

```rust,ignore
use geoxi::crs::{CoordTransform, TransformError};
use proxi::{AngularUnits, Atomicity, CoordBatch, Direction, Transformer};

struct Proj(Transformer);

impl CoordTransform for Proj {
    fn preserves_z(&self) -> bool {
        true
    }

    fn transform_batch(
        &mut self,
        x: &mut [f64],
        y: &mut [f64],
        z: Option<&mut [f64]>,
    ) -> Result<(), TransformError> {
        let batch = CoordBatch::new(x, y)
            .and_then(|b| match z {
                Some(z) => b.with_z(z),
                None => Ok(b),
            })
            .map_err(|e| TransformError::new(e.to_string()))?;

        self.0
            .transform_batch(batch, Direction::Forward, AngularUnits::Auto, Atomicity::All)
            .map(|_| ())
            .map_err(|e| TransformError::new(e.to_string()))
    }
}

let projected = geoxi::crs::reproject(&geometry, &mut Proj(transform))?;
```

`reproject_all` batches a whole collection, and `reproject_bounds` densifies
edges before transforming because projected edges are curved.

## Contexts, threading, and performance

Every GEOS call goes through a `GEOSContextHandle_t`. `geoxi` wraps that in
`Context` - but you never construct one, pass one, or name one in a signature.
Each thread lazily creates its context on first use, and every geometry built on
that thread shares it automatically.

```bash
100000 points, private context each: 215.6ms
100000 points, thread-shared context: 121.8ms
speedup: 1.77x
```

The thread-local gets you the fast number unconditionally. There is no slow path
to accidentally take, and no second way to spell the same operation.

Sharing also makes binary operations well-defined for free: GEOS is undefined if
a geometry is passed to a handle that did not allocate it, and same-thread
geometries are always on the same handle.

### Threading

GEOS forbids sharing a context between threads. `geoxi` encodes this in the type
system: `Geometry`, `PreparedGeometry`, and `STRtree` are `Send` but deliberately
**not** `Sync`. `&Geometry` across threads will not compile, which is the point.

**Move, don't share.** A geometry moved to another thread keeps its original
context, so operations mixing it with local geometries transparently migrate it
via a WKB round-trip - safe, but a per-operation cost. Call `Geometry::rehome()`
once to rebuild it in the current thread's context and get back on the fast path:

```rust
use geoxi::Geometry;

let foreign = std::thread::spawn(|| Geometry::point(1.0, 1.0).unwrap())
    .join()
    .unwrap();
let local = foreign.rehome()?;  // now shares this thread's context
# Ok::<(), geoxi::Error>(())
```

## Features

- `v3_14_0` (default) - the GEOS entry points `geoxi` binds. `geoxi` provisions
  and builds GEOS 3.14.1 itself; this feature must match that version, and the
  build script asserts it does.
- `system` - link an already-installed GEOS instead of building one, located
  via `GEOS_DIR`, vcpkg, or pkg-config. Opt-in; see "Linking GEOS" above.
- `glam`, `nalgebra`, `geo-types` - coordinate-type interop, described below.

### Bring your own coordinate type

Every constructor is generic over `CoordLike`, and every coordinate reader is
too. Tuples, arrays, `Coord`, `Coord3`, and `Vec3` implement it out of the box;
these features add more:

| Feature     | Types                                                                           |
| ----------- | ------------------------------------------------------------------------------- |
| `glam`      | `DVec2`, `DVec3`                                                                |
| `nalgebra`  | `Point2`/`Point3`/`Vector2`/`Vector3<f64>`                                      |
| `geo-types` | `geo_types::Coord`, `geo_types::Point`, plus whole-geometry `from_geo`/`to_geo` |

```rust
use geoxi::Geometry;

let line = Geometry::line_string(&[(0.0, 0.0), (3.0, 4.0)])?;
let line_3d = Geometry::line_string(&[[0.0, 0.0, 0.0], [3.0, 4.0, 5.0]])?;
let back: Vec<(f64, f64)> = line.coords()?;
# Ok::<(), geoxi::Error>(())
```

Implement `CoordLike` for your own struct and it works everywhere too - see
`cargo run --example byoc`.

This is **zero-allocation, not zero-copy**: the trait reads `x()`/`y()`/`z()`
component-wise and rebuilds via `from_xyzm`, with every accessor `#[inline]`. No
conversion function is ever called and no intermediate `Vec` is built. There is
nothing to borrow in the other direction - a GEOS coordinate sequence is an
opaque C++ object behind a pointer.

`geo-types` whole-geometry conversion is a genuine structural walk in both
directions. Convert at your program boundary, not in a loop. `geo-types` is also
strictly 2D, so Z is dropped.
