//! Constructive operations: buffers, hulls, simplification, overlays.

use crate::internal_prelude::*;

impl Geometry {
    pub fn buffer(&self, distance: f64, quad_segs: i32) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSBuffer_r(ctx, g, distance, quad_segs) },
            "buffer",
        )
    }

    pub fn buffer_with_style(&self, params: BufferOptions) -> Result<Self> {
        let raw_params = NonNull::new(unsafe { GEOSBufferParams_create_r(self.ctx.raw()) })
            .ok_or(Error::Null("creating buffer parameters"))?;
        check_code(
            unsafe {
                GEOSBufferParams_setQuadrantSegments_r(
                    self.ctx.raw(),
                    raw_params.as_ptr(),
                    params.quad_segs,
                )
            },
            "setting buffer quadrant segments",
        )?;
        check_code(
            unsafe {
                GEOSBufferParams_setEndCapStyle_r(
                    self.ctx.raw(),
                    raw_params.as_ptr(),
                    params.cap_style as c_int,
                )
            },
            "setting buffer cap style",
        )?;
        check_code(
            unsafe {
                GEOSBufferParams_setJoinStyle_r(
                    self.ctx.raw(),
                    raw_params.as_ptr(),
                    params.join_style as c_int,
                )
            },
            "setting buffer join style",
        )?;
        check_code(
            unsafe {
                GEOSBufferParams_setMitreLimit_r(
                    self.ctx.raw(),
                    raw_params.as_ptr(),
                    params.mitre_limit,
                )
            },
            "setting buffer mitre limit",
        )?;
        check_code(
            unsafe {
                GEOSBufferParams_setSingleSided_r(
                    self.ctx.raw(),
                    raw_params.as_ptr(),
                    params.single_sided as c_int,
                )
            },
            "setting single-sided buffer",
        )?;
        let ptr = unsafe {
            GEOSBufferWithParams_r(
                self.ctx.raw(),
                self.ptr.as_ptr(),
                raw_params.as_ptr(),
                params.distance,
            )
        };
        unsafe { GEOSBufferParams_destroy_r(self.ctx.raw(), raw_params.as_ptr()) };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("buffering geometry"))
    }

    pub fn offset_curve(
        &self,
        distance: f64,
        quad_segs: i32,
        join_style: JoinStyle,
        mitre_limit: f64,
    ) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe {
                GEOSOffsetCurve_r(
                    ctx,
                    g,
                    distance,
                    quad_segs,
                    join_style as c_int,
                    mitre_limit,
                )
            },
            "offset curve",
        )
    }

    pub fn boundary(&self) -> Result<Self> {
        unary_ptr(self, GEOSBoundary_r, "boundary")
    }
    pub fn envelope(&self) -> Result<Self> {
        unary_ptr(self, GEOSEnvelope_r, "envelope")
    }
    pub fn convex_hull(&self) -> Result<Self> {
        unary_ptr(self, GEOSConvexHull_r, "convex hull")
    }
    pub fn centroid(&self) -> Result<Self> {
        unary_ptr(self, GEOSGetCentroid_r, "centroid")
    }
    pub fn point_on_surface(&self) -> Result<Self> {
        unary_ptr(self, GEOSPointOnSurface_r, "point on surface")
    }
    pub fn minimum_rotated_rectangle(&self) -> Result<Self> {
        unary_ptr(
            self,
            GEOSMinimumRotatedRectangle_r,
            "minimum rotated rectangle",
        )
    }
    pub fn oriented_envelope(&self) -> Result<Self> {
        self.minimum_rotated_rectangle()
    }
    pub fn minimum_width(&self) -> Result<Self> {
        unary_ptr(self, GEOSMinimumWidth_r, "minimum width")
    }
    pub fn minimum_clearance_line(&self) -> Result<Self> {
        unary_ptr(self, GEOSMinimumClearanceLine_r, "minimum clearance line")
    }
    pub fn extract_unique_points(&self) -> Result<Self> {
        unary_ptr(
            self,
            GEOSGeom_extractUniquePoints_r,
            "extract unique points",
        )
    }
    pub fn build_area(&self) -> Result<Self> {
        unary_ptr(self, GEOSBuildArea_r, "build area")
    }
    pub fn node(&self) -> Result<Self> {
        unary_ptr(self, GEOSNode_r, "node")
    }
    pub fn make_valid(&self) -> Result<Self> {
        unary_ptr(self, GEOSMakeValid_r, "make valid")
    }
    pub fn remove_repeated_points(&self, tolerance: f64) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSRemoveRepeatedPoints_r(ctx, g, tolerance) },
            "remove repeated points",
        )
    }
    pub fn reverse(&self) -> Result<Self> {
        unary_ptr(self, GEOSReverse_r, "reverse")
    }
    pub fn segmentize(&self, max_segment_length: f64) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSDensify_r(ctx, g, max_segment_length) },
            "segmentize",
        )
    }
    pub fn simplify(&self, tolerance: f64, preserve_topology: bool) -> Result<Self> {
        if preserve_topology {
            unary_geom(
                self,
                |ctx, g| unsafe { GEOSTopologyPreserveSimplify_r(ctx, g, tolerance) },
                "topology-preserving simplify",
            )
        } else {
            unary_geom(
                self,
                |ctx, g| unsafe { GEOSSimplify_r(ctx, g, tolerance) },
                "simplify",
            )
        }
    }
    pub fn clip_by_rect(&self, xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSClipByRect_r(ctx, g, xmin, ymin, xmax, ymax) },
            "clip by rect",
        )
    }
    pub fn concave_hull(&self, ratio: f64, allow_holes: bool) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSConcaveHull_r(ctx, g, ratio, allow_holes as c_uint) },
            "concave hull",
        )
    }
    pub fn delaunay_triangles(&self, tolerance: f64, only_edges: bool) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSDelaunayTriangulation_r(ctx, g, tolerance, only_edges as c_int) },
            "Delaunay triangulation",
        )
    }
    pub fn constrained_delaunay_triangles(&self) -> Result<Self> {
        unary_ptr(
            self,
            GEOSConstrainedDelaunayTriangulation_r,
            "constrained Delaunay triangulation",
        )
    }
    pub fn voronoi_polygons(
        &self,
        envelope: Option<&Geometry>,
        tolerance: f64,
        only_edges: bool,
    ) -> Result<Self> {
        let env = envelope.map_or(ptr::null(), |g| g.ptr.as_ptr());
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSVoronoiDiagram_r(ctx, g, env, tolerance, only_edges as c_int) },
            "Voronoi polygons",
        )
    }
    pub fn maximum_inscribed_circle(&self, tolerance: f64) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSMaximumInscribedCircle_r(ctx, g, tolerance) },
            "maximum inscribed circle",
        )
    }
    pub fn minimum_bounding_circle(&self) -> Result<MinimumBoundingCircle> {
        let mut radius = 0.0;
        let mut center = ptr::null_mut();
        let circle = unsafe {
            GEOSMinimumBoundingCircle_r(self.ctx.raw(), self.ptr.as_ptr(), &mut radius, &mut center)
        };
        Ok(MinimumBoundingCircle {
            circle: unsafe { Self::from_raw(self.ctx.clone(), circle) }
                .map_err(|_| Error::Null("minimum bounding circle"))?,
            center: unsafe { Self::from_raw(self.ctx.clone(), center) }
                .map_err(|_| Error::Null("minimum bounding circle center"))?,
            radius,
        })
    }

    pub fn intersection(&self, other: &Geometry) -> Result<Self> {
        binary_ptr(self, other, GEOSIntersection_r, "intersection")
    }
    pub fn union(&self, other: &Geometry) -> Result<Self> {
        binary_ptr(self, other, GEOSUnion_r, "union")
    }
    pub fn difference(&self, other: &Geometry) -> Result<Self> {
        binary_ptr(self, other, GEOSDifference_r, "difference")
    }
    pub fn symmetric_difference(&self, other: &Geometry) -> Result<Self> {
        binary_ptr(self, other, GEOSSymDifference_r, "symmetric difference")
    }
    pub fn intersection_prec(&self, other: &Geometry, grid_size: f64) -> Result<Self> {
        binary_geom(
            self,
            other,
            |ctx, a, b| unsafe { GEOSIntersectionPrec_r(ctx, a, b, grid_size) },
            "precision intersection",
        )
    }
    pub fn union_prec(&self, other: &Geometry, grid_size: f64) -> Result<Self> {
        binary_geom(
            self,
            other,
            |ctx, a, b| unsafe { GEOSUnionPrec_r(ctx, a, b, grid_size) },
            "precision union",
        )
    }
    pub fn difference_prec(&self, other: &Geometry, grid_size: f64) -> Result<Self> {
        binary_geom(
            self,
            other,
            |ctx, a, b| unsafe { GEOSDifferencePrec_r(ctx, a, b, grid_size) },
            "precision difference",
        )
    }
    pub fn symmetric_difference_prec(&self, other: &Geometry, grid_size: f64) -> Result<Self> {
        binary_geom(
            self,
            other,
            |ctx, a, b| unsafe { GEOSSymDifferencePrec_r(ctx, a, b, grid_size) },
            "precision symmetric difference",
        )
    }
    pub fn snap(&self, reference: &Geometry, tolerance: f64) -> Result<Self> {
        binary_geom(
            self,
            reference,
            |ctx, a, b| unsafe { GEOSSnap_r(ctx, a, b, tolerance) },
            "snap",
        )
    }
    pub fn shared_paths(&self, other: &Geometry) -> Result<Self> {
        binary_ptr(self, other, GEOSSharedPaths_r, "shared paths")
    }

    pub fn shortest_line(&self, other: &Geometry) -> Result<Self> {
        let points = self.nearest_points(other)?;
        Self::line_string(&points)
    }

    pub fn nearest_points(&self, other: &Geometry) -> Result<[Coord; 2]> {
        let seq = NonNull::new(unsafe {
            GEOSNearestPoints_r(self.ctx.raw(), self.ptr.as_ptr(), other.ptr.as_ptr())
        })
        .ok_or(Error::Null("finding nearest points"))?;
        let mut points = [Coord::xy(0.0, 0.0), Coord::xy(0.0, 0.0)];
        for (i, point) in points.iter_mut().enumerate() {
            let mut x = 0.0;
            let mut y = 0.0;
            check_code(
                unsafe {
                    GEOSCoordSeq_getXY_r(self.ctx.raw(), seq.as_ptr(), i as c_uint, &mut x, &mut y)
                },
                "reading nearest point coordinate",
            )?;
            *point = Coord::xy(x, y);
        }
        unsafe { GEOSCoordSeq_destroy_r(self.ctx.raw(), seq.as_ptr()) };
        Ok(points)
    }

    pub fn unary_union(&self) -> Result<Self> {
        unary_ptr(self, GEOSUnaryUnion_r, "unary union")
    }
    pub fn unary_union_prec(&self, grid_size: f64) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSUnaryUnionPrec_r(ctx, g, grid_size) },
            "precision unary union",
        )
    }
    pub fn coverage_union(&self) -> Result<Self> {
        unary_ptr(self, GEOSCoverageUnion_r, "coverage union")
    }
    pub fn disjoint_subset_union(&self) -> Result<Self> {
        unary_ptr(self, GEOSDisjointSubsetUnion_r, "disjoint subset union")
    }
}
