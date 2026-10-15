# Third-party notices

`geoxi` is licensed under Apache-2.0 (see `LICENSE`). It builds and links the
following native software. This file is informational, not legal advice.

## GEOS - LGPL-2.1

- Project: https://libgeos.org
- Version: see `native/versions.toml` (source archive and SHA-256 are pinned)
- Licence: GNU Lesser General Public License v2.1,
  https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html

GEOS source is unmodified and is downloaded from the URL in
`native/versions.toml`. If you redistribute a binary that contains GEOS you must:

1. Include the LGPL-2.1 text and GEOS's copyright notices.
2. Offer the corresponding GEOS source (the pinned URL above satisfies this for
   the unmodified version).
3. With the default `dynamic` feature: ship GEOS as the separate shared
   libraries (`geos_c`, `geos`) so users can replace them.
4. With the `static` feature: also allow users to relink your application against
   a modified GEOS (for example by providing object files or your source). Most
   closed-source products should use `dynamic` instead.

## Other notices

- GEOS bundles `json.hpp` (MIT) and `ryu` (Apache-2.0 / Boost-1.0).
- Any application that also links PROJ (MIT) should reproduce its notice.
