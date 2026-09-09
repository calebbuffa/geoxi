//! `geoxi` build script - self-provisioning, fully self-contained GEOS.
//!
//! # Why this exists
//!
//! GEOS is a **C++** library. Linking it statically is not by itself enough to
//! produce a portable binary: if the C++ standard library and runtime are
//! linked dynamically, the resulting executable still requires the MSVC
//! redistributable (`msvcp140.dll`, `vcruntime140.dll`) on Windows or a
//! matching `libstdc++.so.6` elsewhere. Shipping such a binary to a clean
//! machine fails at process start - exactly the situation this crate must
//! never put its users in.
//!
//! `geoxi` therefore builds GEOS from pinned, checksum-verified source with a
//! C++ runtime that matches the Rust target, and links the C++ standard
//! library statically wherever that is possible. Building with
//! `-C target-feature=+crt-static` yields an executable that depends only on
//! OS-provided libraries and can be copied to any machine of the same target.
//!
//! The provisioning itself - download, checksum, extract, cache, build - is
//! [`woodshed`]'s job. What stays here is only what is specific to GEOS: which
//! prefixes are usable, which CMake options to set, and how to link the result.
//!
//! # Decision tree
//!
//! 1. If `DOCS_RS` is set: emit nothing (rustdoc only needs an rlib).
//! 2. If the `system` feature is enabled: link an installed GEOS found via
//!    `GEOS_DIR`, vcpkg, or pkg-config. Opt-in, and *not* self-contained - the
//!    resulting binary depends on that installation.
//! 3. Otherwise (the default): build vendored GEOS from source and link it
//!    statically.
//!
//! The from-source path is the default precisely because it is the only one
//! that works on a machine with no GEOS installed.

use std::env;
use std::path::{Path, PathBuf};
use woodshed::{BuildKey, Cache, CmakeBuild, Manifest, probe};

/// Files that must exist in an install prefix for a build to count as
/// finished. A prefix missing any of them is the debris of an interrupted
/// build, and is rebuilt rather than linked against.
const INSTALLED: [&str; 2] = ["include/geos_c.h", "lib"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=native/versions.toml");

    // docs.rs builds in a network-isolated sandbox and only needs the crate to
    // type-check as an rlib, so skip provisioning rather than failing on the
    // first download attempt.
    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    // Cargo exposes features to build scripts only as CARGO_FEATURE_* env
    // vars; `cfg!(feature = ...)` would silently always be false here.
    if env::var_os("CARGO_FEATURE_SYSTEM").is_some() && try_system_geos() {
        return;
    }

    build_vendored_geos();
}

/// Try each system-discovery strategy in turn. Each degrades to "not found"
/// rather than failing hard, so an unusable install falls through to the
/// vendored build instead of breaking it.
fn try_system_geos() -> bool {
    let env_dir = probe::EnvDirProbe {
        env_var: "GEOS_DIR",
        validate: validate_installation,
        link: link_from_prefix,
    };
    let vcpkg = probe::VcpkgProbe::new("geos");
    let pkg_config = probe::PkgConfigProbe::new("geos");
    let probes: [&dyn probe::Probe; 3] = [&env_dir, &vcpkg, &pkg_config];

    if probe::first_match(probes) {
        woodshed::link_cpp_runtime();
        return true;
    }

    eprintln!("GEOXI: no usable system GEOS found; building from vendored source");
    false
}

/// A prefix is usable only if it has the C API header and a linkable library.
fn validate_installation(prefix: &Path) -> Result<(), String> {
    let has_header = ["include/geos_c.h", "include/geos/geos_c.h"]
        .iter()
        .any(|header| prefix.join(header).exists());
    if !has_header {
        return Err(format!("{} lacks include/geos_c.h", prefix.display()));
    }

    for dir in ["lib", "lib64", "bin"] {
        let dir = prefix.join(dir);
        if !dir.is_dir() {
            continue;
        }
        for candidate in [
            "geos_c.lib",
            "libgeos_c.a",
            "libgeos_c.so",
            "libgeos_c.dylib",
            "geos_c.dll",
        ] {
            if dir.join(candidate).exists() {
                return Ok(());
            }
        }
    }
    Err(format!("{} lacks a GEOS C library", prefix.display()))
}

/// Emit link directives for a validated installation prefix.
fn link_from_prefix(prefix: &Path) {
    for dir in ["lib", "lib64", "bin"] {
        let dir = prefix.join(dir);
        if dir.is_dir() {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    }
    println!("cargo:rustc-link-lib=geos_c");
    println!("cargo:includedir={}/include", prefix.display());
}

/// Download, verify, extract, configure, build, and link GEOS from source.
fn build_vendored_geos() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let manifest = Manifest::load_default(&manifest_dir)
        .unwrap_or_else(|error| fail(&format!("cannot load native/versions.toml: {error}")));
    let dep = manifest
        .get("geos")
        .unwrap_or_else(|error| fail(&error.to_string()));

    assert_bindings_match(&dep.version);

    let cache = Cache::discover().unwrap_or_else(|error| fail(&error.to_string()));
    let source = cache
        .source(dep)
        .unwrap_or_else(|error| fail(&format!("cannot provision GEOS source: {error}")));

    // The CRT mode is part of the build's identity: a static-CRT and a
    // dynamic-CRT GEOS are not interchangeable and must never share a prefix.
    let key = BuildKey::new("geos")
        .dep(dep)
        .mix(woodshed::crt_static().to_string());
    let dirs = cache.build_dir(&key);

    if dirs.is_complete(INSTALLED) {
        eprintln!(
            "GEOXI: reusing cached GEOS build at {}",
            dirs.prefix.display()
        );
    } else {
        CmakeBuild::new(&source, &dirs)
            // Static libraries only: nothing to ship alongside the executable.
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("BUILD_TESTING", "OFF")
            .define("BUILD_BENCHMARKS", "OFF")
            .define("BUILD_DOCUMENTATION", "OFF")
            .define("BUILD_GEOSOP", "OFF")
            .define("CMAKE_INSTALL_LIBDIR", "lib")
            .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
            // GEOS predates CMake 4's minimum-version floor.
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .compiler_launcher_define("CMAKE_C_COMPILER_LAUNCHER")
            .install()
            .unwrap_or_else(|error| {
                fail(&format!(
                    "{error}\n\
                     geoxi builds GEOS from source so the resulting binary is fully \
                     self-contained; a working C++ compiler and CMake are required at \
                     build time (not at run time). Install CMake, or enable the \
                     `system` feature to link an already-installed GEOS instead."
                ))
            });
    }

    emit_link_directives(&dirs.prefix);
    woodshed::link_cpp_runtime();
}

/// Fail early if the `geos-sys` binding version does not match the GEOS being
/// compiled.
///
/// A mismatch is not caught by the compiler: `geos-sys` in `dox` mode declares
/// every function regardless, so the build succeeds and only fails much later
/// at link time with a wall of unresolved `GEOS*` symbols. Checking here turns
/// that into an immediate, explanatory error.
fn assert_bindings_match(pinned: &str) {
    let mut parts = pinned.split('.');
    let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
        return;
    };
    if env::var_os(format!("CARGO_FEATURE_V{major}_{minor}_0")).is_none() {
        fail(&format!(
            "native/versions.toml pins GEOS {pinned}, but the matching \
             `geos-sys` binding feature (v{major}_{minor}_0) is not enabled.\n\
             The declared bindings and the compiled library must agree or the \
             build fails at link time with unresolved GEOS symbols.\n\
             Fix the `geos-sys` features in Cargo.toml to match the pinned version."
        ));
    }
}

/// Emit link directives for the built prefix.
///
/// Order matters for static linking: `geos_c` depends on `geos`, so it is
/// listed first.
fn emit_link_directives(prefix: &Path) {
    for dir in ["lib", "lib64"] {
        let dir = prefix.join(dir);
        if dir.is_dir() {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    }
    println!("cargo:rustc-link-lib=static=geos_c");
    println!("cargo:rustc-link-lib=static=geos");
    println!("cargo:includedir={}/include", prefix.display());
}

fn fail(message: &str) -> ! {
    panic!("GEOXI: {message}");
}
