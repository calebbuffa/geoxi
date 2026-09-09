//! STRtree spatial index.

use crate::internal_prelude::*;

pub struct STRtree {
    pub(crate) ctx: Context,
    pub(crate) ptr: NonNull<GEOSSTRtree>,
    pub(crate) geometries: Vec<Geometry>,
}

// SAFETY: see the note on `Geometry`. Movable across threads, never shared.
// GEOS additionally mutates an STRtree on first query, so shared access would
// be unsound even setting the context rule aside.
unsafe impl Send for STRtree {}

impl STRtree {
    /// Build an index inside an existing context.
    ///
    /// Geometries belonging to another context are migrated into `ctx`, since
    /// GEOS cannot index a geometry it did not allocate.
    pub fn new<I>(geometries: I, node_capacity: usize) -> Result<Self>
    where
        I: IntoIterator<Item = Geometry>,
    {
        let ctx = Context::current()?;
        let tree = NonNull::new(unsafe { GEOSSTRtree_create_r(ctx.raw(), node_capacity.max(2)) })
            .ok_or(Error::Null("creating STRtree"))?;
        let geometries: Vec<_> = geometries
            .into_iter()
            .map(|g| g.to_context(&ctx))
            .collect::<Result<_>>()?;
        for (index, geom) in geometries.iter().enumerate() {
            unsafe {
                GEOSSTRtree_insert_r(
                    ctx.raw(),
                    tree.as_ptr(),
                    geom.ptr.as_ptr(),
                    index as *mut c_void,
                );
            }
        }
        check_code(
            unsafe { GEOSSTRtree_build_r(ctx.raw(), tree.as_ptr()) },
            "building STRtree",
        )?;
        Ok(Self {
            ctx,
            ptr: tree,
            geometries,
        })
    }

    pub fn len(&self) -> usize {
        self.geometries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.geometries.is_empty()
    }

    pub fn geometries(&self) -> &[Geometry] {
        &self.geometries
    }

    /// Query the index for candidates whose envelope intersects `geometry`.
    ///
    /// Returns indices into [`STRtree::geometries`].
    pub fn query(&self, geometry: &Geometry) -> Result<Vec<usize>> {
        extern "C" fn collect(item: *mut c_void, userdata: *mut c_void) {
            let out = unsafe { &mut *(userdata as *mut Vec<usize>) };
            out.push(item as usize);
        }

        let geometry = crate::ffi::aligned_to(&self.ctx, geometry)?;
        let mut out = Vec::new();
        unsafe {
            GEOSSTRtree_query_r(
                self.ctx.raw(),
                self.ptr.as_ptr(),
                geometry.ptr.as_ptr(),
                Some(collect),
                (&mut out as *mut Vec<usize>).cast(),
            );
        }
        Ok(out)
    }

    pub fn query_predicate(
        &self,
        geometry: &Geometry,
        predicate: BinaryPredicate,
    ) -> Result<Vec<usize>> {
        let candidates = self.query(geometry)?;
        candidates
            .into_iter()
            .filter_map(
                |idx| match predicate.test(geometry, &self.geometries[idx]) {
                    Ok(true) => Some(Ok(idx)),
                    Ok(false) => None,
                    Err(err) => Some(Err(err)),
                },
            )
            .collect()
    }
}

impl Drop for STRtree {
    fn drop(&mut self) {
        unsafe { GEOSSTRtree_destroy_r(self.ctx.raw(), self.ptr.as_ptr()) };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryPredicate {
    Intersects,
    Within,
    Contains,
    Overlaps,
    Crosses,
    Touches,
    Covers,
    CoveredBy,
    ContainsProperly,
    DWithin(f64),
}

impl BinaryPredicate {
    fn test(self, left: &Geometry, right: &Geometry) -> Result<bool> {
        match self {
            Self::Intersects => left.intersects(right),
            Self::Within => left.within(right),
            Self::Contains => left.contains(right),
            Self::Overlaps => left.overlaps(right),
            Self::Crosses => left.crosses(right),
            Self::Touches => left.touches(right),
            Self::Covers => left.covers(right),
            Self::CoveredBy => left.covered_by(right),
            Self::ContainsProperly => left.prepared()?.contains_properly(right),
            Self::DWithin(distance) => left.dwithin(right, distance),
        }
    }
}
