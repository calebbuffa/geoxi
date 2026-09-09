//! Unary operations, set operations, linear referencing, coverages.

use crate::internal_prelude::*;

pub fn is_empty(geometry: &Geometry) -> Result<bool> {
    geometry.is_empty()
}
pub fn is_simple(geometry: &Geometry) -> Result<bool> {
    geometry.is_simple()
}
pub fn is_ring(geometry: &Geometry) -> Result<bool> {
    geometry.is_ring()
}
pub fn is_closed(geometry: &Geometry) -> Result<bool> {
    geometry.is_closed()
}
pub fn is_valid(geometry: &Geometry) -> Result<bool> {
    geometry.is_valid()
}
pub fn is_valid_reason(geometry: &Geometry) -> Result<String> {
    geometry.is_valid_reason()
}

pub fn boundary(geometry: &Geometry) -> Result<Geometry> {
    geometry.boundary()
}
pub fn buffer(geometry: &Geometry, distance: f64, quad_segs: i32) -> Result<Geometry> {
    geometry.buffer(distance, quad_segs)
}
pub fn buffer_with_style(geometry: &Geometry, params: BufferOptions) -> Result<Geometry> {
    geometry.buffer_with_style(params)
}
pub fn offset_curve(
    geometry: &Geometry,
    distance: f64,
    quad_segs: i32,
    join_style: JoinStyle,
    mitre_limit: f64,
) -> Result<Geometry> {
    geometry.offset_curve(distance, quad_segs, join_style, mitre_limit)
}
pub fn centroid(geometry: &Geometry) -> Result<Geometry> {
    geometry.centroid()
}
pub fn clip_by_rect(
    geometry: &Geometry,
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
) -> Result<Geometry> {
    geometry.clip_by_rect(xmin, ymin, xmax, ymax)
}
pub fn concave_hull(geometry: &Geometry, ratio: f64, allow_holes: bool) -> Result<Geometry> {
    geometry.concave_hull(ratio, allow_holes)
}
pub fn convex_hull(geometry: &Geometry) -> Result<Geometry> {
    geometry.convex_hull()
}
pub fn delaunay_triangles(
    geometry: &Geometry,
    tolerance: f64,
    only_edges: bool,
) -> Result<Geometry> {
    geometry.delaunay_triangles(tolerance, only_edges)
}
pub fn constrained_delaunay_triangles(geometry: &Geometry) -> Result<Geometry> {
    geometry.constrained_delaunay_triangles()
}
pub fn envelope(geometry: &Geometry) -> Result<Geometry> {
    geometry.envelope()
}
pub fn extract_unique_points(geometry: &Geometry) -> Result<Geometry> {
    geometry.extract_unique_points()
}
pub fn build_area(geometry: &Geometry) -> Result<Geometry> {
    geometry.build_area()
}
pub fn make_valid(geometry: &Geometry) -> Result<Geometry> {
    geometry.make_valid()
}
pub fn minimum_clearance_line(geometry: &Geometry) -> Result<Geometry> {
    geometry.minimum_clearance_line()
}
pub fn normalize(geometry: &Geometry) -> Result<Geometry> {
    geometry.normalized()
}
pub fn point_on_surface(geometry: &Geometry) -> Result<Geometry> {
    geometry.point_on_surface()
}
pub fn node(geometry: &Geometry) -> Result<Geometry> {
    geometry.node()
}
pub fn remove_repeated_points(geometry: &Geometry, tolerance: f64) -> Result<Geometry> {
    geometry.remove_repeated_points(tolerance)
}
pub fn reverse(geometry: &Geometry) -> Result<Geometry> {
    geometry.reverse()
}
pub fn segmentize(geometry: &Geometry, max_segment_length: f64) -> Result<Geometry> {
    geometry.segmentize(max_segment_length)
}
pub fn simplify(geometry: &Geometry, tolerance: f64, preserve_topology: bool) -> Result<Geometry> {
    geometry.simplify(tolerance, preserve_topology)
}
pub fn snap(geometry: &Geometry, reference: &Geometry, tolerance: f64) -> Result<Geometry> {
    geometry.snap(reference, tolerance)
}
pub fn voronoi_polygons(
    geometry: &Geometry,
    envelope: Option<&Geometry>,
    tolerance: f64,
    only_edges: bool,
) -> Result<Geometry> {
    geometry.voronoi_polygons(envelope, tolerance, only_edges)
}
pub fn oriented_envelope(geometry: &Geometry) -> Result<Geometry> {
    geometry.oriented_envelope()
}
pub fn minimum_bounding_circle(geometry: &Geometry) -> Result<MinimumBoundingCircle> {
    geometry.minimum_bounding_circle()
}
pub fn minimum_width(geometry: &Geometry) -> Result<Geometry> {
    geometry.minimum_width()
}
pub fn maximum_inscribed_circle(geometry: &Geometry, tolerance: f64) -> Result<Geometry> {
    geometry.maximum_inscribed_circle(tolerance)
}
pub fn difference(a: &Geometry, b: &Geometry, grid_size: Option<f64>) -> Result<Geometry> {
    grid_size.map_or_else(|| a.difference(b), |g| a.difference_prec(b, g))
}
pub fn intersection(a: &Geometry, b: &Geometry, grid_size: Option<f64>) -> Result<Geometry> {
    grid_size.map_or_else(|| a.intersection(b), |g| a.intersection_prec(b, g))
}
pub fn symmetric_difference(
    a: &Geometry,
    b: &Geometry,
    grid_size: Option<f64>,
) -> Result<Geometry> {
    grid_size.map_or_else(
        || a.symmetric_difference(b),
        |g| a.symmetric_difference_prec(b, g),
    )
}
pub fn union(a: &Geometry, b: &Geometry, grid_size: Option<f64>) -> Result<Geometry> {
    grid_size.map_or_else(|| a.union(b), |g| a.union_prec(b, g))
}
pub fn union_all(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::GeometryCollection, geometries)?.unary_union()
}
pub fn intersection_all(mut geometries: Vec<Geometry>) -> Result<Option<Geometry>> {
    let Some(first) = geometries.pop() else {
        return Ok(None);
    };
    geometries
        .into_iter()
        .try_fold(first, |acc, geom| acc.intersection(&geom))
        .map(Some)
}
pub fn coverage_union_all(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::GeometryCollection, geometries)?.coverage_union()
}
pub fn disjoint_subset_union_all(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::GeometryCollection, geometries)?.disjoint_subset_union()
}

pub fn line_interpolate_point(
    line: &Geometry,
    distance: f64,
    normalized: bool,
) -> Result<Geometry> {
    line.line_interpolate_point(distance, normalized)
}
pub fn line_locate_point(line: &Geometry, point: &Geometry, normalized: bool) -> Result<f64> {
    line.line_locate_point(point, normalized)
}
pub fn line_merge(line: &Geometry, directed: bool) -> Result<Geometry> {
    line.line_merge(directed)
}
pub fn shared_paths(a: &Geometry, b: &Geometry) -> Result<Geometry> {
    a.shared_paths(b)
}
pub fn shortest_line(a: &Geometry, b: &Geometry) -> Result<Geometry> {
    a.shortest_line(b)
}
pub fn nearest_points(a: &Geometry, b: &Geometry) -> Result<[Coord; 2]> {
    a.nearest_points(b)
}
pub fn substring(line: &Geometry, start_fraction: f64, end_fraction: f64) -> Result<Geometry> {
    line.line_substring(start_fraction, end_fraction)
}

pub fn polygonize(geometries: &[Geometry]) -> Result<Geometry> {
    let ctx = geometries
        .first()
        .map_or_else(Context::current, |g| Ok(g.ctx.clone()))?;
    let raw: Vec<_> = geometries
        .iter()
        .map(|g| g.ptr.as_ptr().cast_const())
        .collect();
    let ptr = unsafe { GEOSPolygonize_r(ctx.raw(), raw.as_ptr(), raw.len() as c_uint) };
    unsafe { Geometry::from_raw(ctx, ptr) }.map_err(|_| Error::Null("polygonizing geometries"))
}

pub fn polygonize_valid(geometries: &[Geometry]) -> Result<Geometry> {
    let ctx = geometries
        .first()
        .map_or_else(Context::current, |g| Ok(g.ctx.clone()))?;
    let raw: Vec<_> = geometries
        .iter()
        .map(|g| g.ptr.as_ptr().cast_const())
        .collect();
    let ptr = unsafe { GEOSPolygonize_valid_r(ctx.raw(), raw.as_ptr(), raw.len() as c_uint) };
    unsafe { Geometry::from_raw(ctx, ptr) }
        .map_err(|_| Error::Null("valid polygonizing geometries"))
}

pub fn polygonizer_cut_edges(geometries: &[Geometry]) -> Result<Geometry> {
    let ctx = geometries
        .first()
        .map_or_else(Context::current, |g| Ok(g.ctx.clone()))?;
    let raw: Vec<_> = geometries
        .iter()
        .map(|g| g.ptr.as_ptr().cast_const())
        .collect();
    let ptr =
        unsafe { GEOSPolygonizer_getCutEdges_r(ctx.raw(), raw.as_ptr(), raw.len() as c_uint) };
    unsafe { Geometry::from_raw(ctx, ptr) }
        .map_err(|_| Error::Null("getting polygonizer cut edges"))
}

#[derive(Debug)]
pub struct PolygonizeFull {
    pub polygons: Geometry,
    pub cut_edges: Geometry,
    pub dangles: Geometry,
    pub invalid_rings: Geometry,
}

pub fn polygonize_full(input: &Geometry) -> Result<PolygonizeFull> {
    let mut cut_edges = ptr::null_mut();
    let mut dangles = ptr::null_mut();
    let mut invalid_rings = ptr::null_mut();
    let polygons = unsafe {
        GEOSPolygonize_full_r(
            input.ctx.raw(),
            input.ptr.as_ptr(),
            &mut cut_edges,
            &mut dangles,
            &mut invalid_rings,
        )
    };
    Ok(PolygonizeFull {
        polygons: unsafe { Geometry::from_raw(input.ctx.clone(), polygons) }
            .map_err(|_| Error::Null("polygonize_full polygons"))?,
        cut_edges: unsafe { Geometry::from_raw(input.ctx.clone(), cut_edges) }
            .map_err(|_| Error::Null("polygonize_full cut edges"))?,
        dangles: unsafe { Geometry::from_raw(input.ctx.clone(), dangles) }
            .map_err(|_| Error::Null("polygonize_full dangles"))?,
        invalid_rings: unsafe { Geometry::from_raw(input.ctx.clone(), invalid_rings) }
            .map_err(|_| Error::Null("polygonize_full invalid rings"))?,
    })
}

/// Coverage validity result: whether the coverage is valid, plus the edges that
/// make it invalid.
#[derive(Debug)]
pub struct CoverageValidity {
    pub is_valid: bool,
    pub invalid_edges: Geometry,
}

/// Check whether a polygonal coverage is valid, returning the offending edges.
pub fn coverage_is_valid(coverage: &Geometry, gap_width: f64) -> Result<CoverageValidity> {
    let mut edges = ptr::null_mut();
    let code = unsafe {
        GEOSCoverageIsValid_r(
            coverage.ctx.raw(),
            coverage.ptr.as_ptr(),
            gap_width,
            &mut edges,
        )
    };
    if code == 2 {
        return Err(Error::Failure("validating coverage"));
    }
    Ok(CoverageValidity {
        is_valid: code == 1,
        invalid_edges: unsafe { Geometry::from_raw(coverage.ctx.clone(), edges) }
            .map_err(|_| Error::Null("reading coverage invalid edges"))?,
    })
}

/// Just the invalid edges of a coverage.
pub fn coverage_invalid_edges(coverage: &Geometry, gap_width: f64) -> Result<Geometry> {
    Ok(coverage_is_valid(coverage, gap_width)?.invalid_edges)
}

/// Simplify a coverage with Visvalingam-Whyatt while keeping shared edges
/// topologically consistent.
pub fn coverage_simplify(
    coverage: &Geometry,
    tolerance: f64,
    preserve_boundary: bool,
) -> Result<Geometry> {
    unary_geom(
        coverage,
        |ctx, g| unsafe { GEOSCoverageSimplifyVW_r(ctx, g, tolerance, preserve_boundary as c_int) },
        "simplifying coverage",
    )
}

/// Options for [`coverage_clean`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CoverageCleanOptions {
    pub snapping_distance: Option<f64>,
    pub gap_maximum_width: Option<f64>,
    pub overlap_merge_strategy: Option<OverlapMergeStrategy>,
}

/// How overlapping coverage areas are assigned during cleaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapMergeStrategy {
    LongestBorder = 0,
    MaxArea = 1,
    MinArea = 2,
    MinIndex = 3,
}

/// Repair gaps and overlaps in a polygonal coverage.
pub fn coverage_clean(coverage: &Geometry, options: CoverageCleanOptions) -> Result<Geometry> {
    let ctx = coverage.ctx.raw();
    let params = NonNull::new(unsafe { GEOSCoverageCleanParams_create_r(ctx) })
        .ok_or(Error::Null("creating coverage clean parameters"))?;

    let configure = || -> Result<()> {
        if let Some(distance) = options.snapping_distance {
            check_code(
                unsafe {
                    GEOSCoverageCleanParams_setSnappingDistance_r(ctx, params.as_ptr(), distance)
                },
                "setting coverage snapping distance",
            )?;
        }
        if let Some(width) = options.gap_maximum_width {
            check_code(
                unsafe {
                    GEOSCoverageCleanParams_setGapMaximumWidth_r(ctx, params.as_ptr(), width)
                },
                "setting coverage gap maximum width",
            )?;
        }
        if let Some(strategy) = options.overlap_merge_strategy {
            check_code(
                unsafe {
                    GEOSCoverageCleanParams_setOverlapMergeStrategy_r(
                        ctx,
                        params.as_ptr(),
                        strategy as c_int,
                    )
                },
                "setting coverage overlap merge strategy",
            )?;
        }
        Ok(())
    };

    let result = configure().and_then(|()| {
        let ptr = unsafe {
            GEOSCoverageCleanWithParams_r(ctx, coverage.ptr.as_ptr(), params.as_ptr().cast_const())
        };
        unsafe { Geometry::from_raw(coverage.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("cleaning coverage"))
    });
    unsafe { GEOSCoverageCleanParams_destroy_r(ctx, params.as_ptr()) };
    result
}
