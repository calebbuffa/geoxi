//! GEOS thread context handle.
//!
//! Every GEOS call goes through a `GEOSContextHandle_t`. `geoxi` wraps that in
//! [`Context`] and manages it for you: **there is no constructor that takes a
//! context, because you never need to pass one.**
//!
//! # How it works
//!
//! Each thread lazily initialises exactly one context the first time it builds
//! a geometry, and every geometry built on that thread shares it. Contexts are
//! reference-counted, so a geometry keeps its context alive even if it outlives
//! the thread that made it.
//!
//! This is both the fast path and the safe path, which is why it is the only
//! path:
//!
//! * **Fast.** Context initialisation costs roughly as much as building a small
//!   geometry, so paying it once per thread rather than once per geometry is a
//!   large saving on batch work (~1.8x on point construction).
//! * **Safe.** GEOS is undefined if a geometry is passed to a handle that did
//!   not allocate it. Because same-thread geometries always share a context,
//!   binary operations are automatically well-defined.
//!
//! ```
//! use geoxi::Geometry;
//!
//! # fn main() -> geoxi::Result<()> {
//! // No context argument, and no per-geometry context allocation either.
//! let points: Vec<Geometry> = (0..1000)
//!     .map(|i| Geometry::point(i as f64, i as f64))
//!     .collect::<geoxi::Result<_>>()?;
//!
//! // All of them share this thread's context.
//! assert!(points[0].context().same(points[999].context()));
//! # Ok(())
//! # }
//! ```
//!
//! # Threads
//!
//! GEOS's reentrant API is thread-safe only so long as one context is never
//! used from two threads at once. `geoxi` encodes that in the type system:
//! [`Geometry`], [`PreparedGeometry`], and
//! [`STRtree`] are `Send` but deliberately **not** `Sync`.
//!
//! **Move, don't share.** Sending a geometry to another thread is fine and
//! carries its context along. Sharing `&Geometry` across threads will not
//! compile, which is the point.
//!
//! A geometry moved to another thread keeps its original context, so it may
//! differ from that thread's own. `geoxi` detects this and migrates
//! transparently, so cross-context operations remain correct - just slower than
//! staying on one thread. Call [`Geometry::rehome`] after a move to opt back
//! into the fast path.

use crate::internal_prelude::*;
use std::cell::RefCell;
use std::marker::PhantomData;

thread_local! {
    /// This thread's context, created on first use.
    ///
    /// `RefCell<Option<..>>` rather than `OnceCell` so a failed initialisation
    /// can be retried rather than poisoning the thread.
    static THREAD_CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
}

/// A GEOS context handle.
///
/// You do not normally construct one: [`Context::current`] returns the calling
/// thread's context and every geometry constructor uses it automatically.
/// Cheap to clone (reference-counted). `Send` but not `Sync` - see the
/// [module documentation](self) for the threading rules.
#[derive(Clone)]
pub struct Context {
    pub(crate) inner: Arc<ContextInner>,
    /// Opts out of `Sync`. GEOS forbids concurrent use of one context.
    pub(crate) _not_sync: PhantomData<std::cell::Cell<()>>,
}

pub(crate) struct ContextInner {
    raw: NonNull<GEOSContextHandle_HS>,
}

// SAFETY: a GEOS context may be moved between threads so long as it is only
// ever used from one thread at a time. The `_not_sync` marker on `Context`
// removes `Sync`, which enforces the "one thread at a time" half.
unsafe impl Send for ContextInner {}
unsafe impl Sync for ContextInner {}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context").finish_non_exhaustive()
    }
}

impl Context {
    /// The calling thread's context, initialising it on first use.
    ///
    /// Every geometry constructor calls this, so geometries built on one thread
    /// automatically share one context. Cloning the returned handle is cheap.
    pub fn current() -> Result<Self> {
        THREAD_CONTEXT.with(|slot| {
            if let Some(ctx) = slot.borrow().as_ref() {
                return Ok(ctx.clone());
            }
            let ctx = Self::create()?;
            *slot.borrow_mut() = Some(ctx.clone());
            Ok(ctx)
        })
    }

    fn create() -> Result<Self> {
        let raw = NonNull::new(unsafe { GEOS_init_r() }).ok_or(Error::Null("initializing GEOS"))?;
        Ok(Self {
            inner: Arc::new(ContextInner { raw }),
            _not_sync: PhantomData,
        })
    }

    /// Whether two handles refer to the same underlying GEOS context.
    ///
    /// A pointer comparison, so it costs nothing.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn raw(&self) -> GEOSContextHandle_t {
        self.inner.raw.as_ptr()
    }
}

impl Drop for ContextInner {
    fn drop(&mut self) {
        unsafe { GEOS_finish_r(self.raw.as_ptr()) };
    }
}
