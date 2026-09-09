//! Native 3D algorithms that fill the gaps GEOS leaves open.
//!
//! GEOS is a planar engine: `convex_hull`, `envelope`, `oriented_envelope`, and
//! friends collapse Z. The routines here operate on true XYZ input so users get
//! the same ergonomic API for 2D and 3D work.
//!
//! Everything in this module is pure Rust and independent of GEOS, so it is
//! usable without a GEOS context and is trivially testable.

use crate::{Coord, Coord3, Error, Geometry, Result};

/// A minimal 3D vector used by the hull/normal math.
///
/// `#[repr(C)]` is guaranteed; see [`crate::Coord3`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn minus(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn plus(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Option<Self> {
        let norm = self.norm();
        if norm <= f64::MIN_POSITIVE {
            None
        } else {
            Some(self.scale(1.0 / norm))
        }
    }

    pub fn distance(self, other: Self) -> f64 {
        self.minus(other).norm()
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self.minus(other)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.plus(other)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, factor: f64) -> Self {
        self.scale(factor)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        self.scale(-1.0)
    }
}

impl From<Coord3> for Vec3 {
    fn from(value: Coord3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl From<Vec3> for Coord3 {
    fn from(value: Vec3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl From<Vec3> for Coord {
    fn from(value: Vec3) -> Self {
        Self::xyz(value.x, value.y, value.z)
    }
}

/// Axis-aligned 3D bounding box, the 3D counterpart of [`crate::Bounds`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds3 {
    pub xmin: f64,
    pub ymin: f64,
    pub zmin: f64,
    pub xmax: f64,
    pub ymax: f64,
    pub zmax: f64,
}

impl Bounds3 {
    pub fn from_points(points: &[Coord3]) -> Option<Self> {
        let first = *points.first()?;
        let mut bounds = Self {
            xmin: first.x,
            ymin: first.y,
            zmin: first.z,
            xmax: first.x,
            ymax: first.y,
            zmax: first.z,
        };
        for point in &points[1..] {
            bounds.xmin = bounds.xmin.min(point.x);
            bounds.ymin = bounds.ymin.min(point.y);
            bounds.zmin = bounds.zmin.min(point.z);
            bounds.xmax = bounds.xmax.max(point.x);
            bounds.ymax = bounds.ymax.max(point.y);
            bounds.zmax = bounds.zmax.max(point.z);
        }
        Some(bounds)
    }

    pub fn center(&self) -> Coord3 {
        Coord3::new(
            (self.xmin + self.xmax) / 2.0,
            (self.ymin + self.ymax) / 2.0,
            (self.zmin + self.zmax) / 2.0,
        )
    }

    pub fn extent(&self) -> Vec3 {
        Vec3::new(
            self.xmax - self.xmin,
            self.ymax - self.ymin,
            self.zmax - self.zmin,
        )
    }

    pub fn volume(&self) -> f64 {
        let extent = self.extent();
        extent.x.max(0.0) * extent.y.max(0.0) * extent.z.max(0.0)
    }

    pub fn contains(&self, point: Coord3) -> bool {
        point.x >= self.xmin
            && point.x <= self.xmax
            && point.y >= self.ymin
            && point.y <= self.ymax
            && point.z >= self.zmin
            && point.z <= self.zmax
    }
}

/// A bounding sphere, the 3D analogue of `minimum_bounding_circle`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub center: Coord3,
    pub radius: f64,
}

/// A triangular face of a 3D hull, indexing into the hull's vertex list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triangle {
    pub a: usize,
    pub b: usize,
    pub c: usize,
}

/// The result of [`convex_hull_3d`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConvexHull3 {
    vertices: Vec<Coord3>,
    faces: Vec<Triangle>,
    degenerate: Degeneracy,
}

/// Describes how the input degenerated, if it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Degeneracy {
    /// A genuine 3D hull with volume.
    None,
    /// All points share a plane; `faces` triangulate that planar polygon.
    Coplanar,
    /// All points lie on a line; `vertices` holds the two extremes.
    Collinear,
    /// All points are (numerically) identical.
    Point,
}

impl ConvexHull3 {
    pub fn vertices(&self) -> &[Coord3] {
        &self.vertices
    }

    pub fn faces(&self) -> &[Triangle] {
        &self.faces
    }

    pub fn degeneracy(&self) -> Degeneracy {
        self.degenerate
    }

    pub fn is_degenerate(&self) -> bool {
        self.degenerate != Degeneracy::None
    }

    /// The triangle corner coordinates for a face.
    pub fn face_points(&self, face: Triangle) -> [Coord3; 3] {
        [
            self.vertices[face.a],
            self.vertices[face.b],
            self.vertices[face.c],
        ]
    }

    /// Outward-facing unit normal for a face, if the face is non-degenerate.
    pub fn face_normal(&self, face: Triangle) -> Option<Vec3> {
        let [a, b, c] = self.face_points(face);
        Vec3::from(b)
            .minus(Vec3::from(a))
            .cross(Vec3::from(c).minus(Vec3::from(a)))
            .normalized()
    }

    /// Total surface area of the hull.
    pub fn surface_area(&self) -> f64 {
        self.faces
            .iter()
            .map(|face| {
                let [a, b, c] = self.face_points(*face);
                Vec3::from(b)
                    .minus(Vec3::from(a))
                    .cross(Vec3::from(c).minus(Vec3::from(a)))
                    .norm()
                    / 2.0
            })
            .sum()
    }

    /// Enclosed volume, computed with the signed-tetrahedron sum. Degenerate
    /// hulls have zero volume.
    pub fn volume(&self) -> f64 {
        if self.degenerate != Degeneracy::None {
            return 0.0;
        }
        let total: f64 = self
            .faces
            .iter()
            .map(|face| {
                let [a, b, c] = self.face_points(*face);
                Vec3::from(a).dot(Vec3::from(b).cross(Vec3::from(c)))
            })
            .sum();
        (total / 6.0).abs()
    }

    /// Volumetric centroid of the hull.
    pub fn centroid(&self) -> Option<Coord3> {
        if self.vertices.is_empty() {
            return None;
        }
        if self.degenerate != Degeneracy::None {
            let count = self.vertices.len() as f64;
            let sum = self
                .vertices
                .iter()
                .fold(Vec3::new(0.0, 0.0, 0.0), |acc, v| acc.plus(Vec3::from(*v)));
            return Some(sum.scale(1.0 / count).into());
        }
        let mut weighted = Vec3::new(0.0, 0.0, 0.0);
        let mut total = 0.0;
        for face in &self.faces {
            let [a, b, c] = self.face_points(*face);
            let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
            let signed = a.dot(b.cross(c)) / 6.0;
            total += signed;
            weighted = weighted.plus(a.plus(b).plus(c).scale(signed / 4.0));
        }
        if total.abs() <= f64::EPSILON {
            return None;
        }
        Some(weighted.scale(1.0 / total).into())
    }

    /// Whether a point lies inside (or on) the hull.
    pub fn contains(&self, point: Coord3, tolerance: f64) -> bool {
        if self.degenerate != Degeneracy::None {
            return false;
        }
        let point = Vec3::from(point);
        self.faces.iter().all(|face| {
            let [a, ..] = self.face_points(*face);
            match self.face_normal(*face) {
                Some(normal) => normal.dot(point.minus(Vec3::from(a))) <= tolerance,
                None => true,
            }
        })
    }

    /// Convert the hull to a GEOS `MultiPolygon` of 3D triangles.
    pub fn to_geometry(&self) -> Result<Geometry> {
        match self.degenerate {
            Degeneracy::Point => {
                let vertex = self.vertices[0];
                Geometry::point_z(vertex.x, vertex.y, vertex.z)
            }
            Degeneracy::Collinear => {
                let coords: Vec<Coord> = self.vertices.iter().map(|v| coord_of(*v)).collect();
                Geometry::line_string(&coords)
            }
            Degeneracy::None | Degeneracy::Coplanar => {
                let triangles = self
                    .faces
                    .iter()
                    .map(|face| {
                        let [a, b, c] = self.face_points(*face);
                        let ring = [coord_of(a), coord_of(b), coord_of(c), coord_of(a)];
                        Geometry::polygon(&ring, crate::no_holes())
                    })
                    .collect::<Result<Vec<_>>>()?;
                Geometry::collection(crate::CollectionType::MultiPolygon, triangles)
            }
        }
    }
}

fn coord_of(point: Coord3) -> Coord {
    Coord::xyz(point.x, point.y, point.z)
}

/// Compute the 3D convex hull of a point cloud using an incremental
/// (quickhull-style) algorithm with horizon-edge stitching.
///
/// Unlike GEOS's `convex_hull`, this preserves Z: the returned hull is a true
/// polyhedron. Degenerate inputs (coplanar, collinear, single point) are
/// reported through [`Degeneracy`] rather than failing.
pub fn convex_hull_3d(points: &[Coord3]) -> Result<ConvexHull3> {
    convex_hull_3d_with_tolerance(points, 1e-12)
}

/// [`convex_hull_3d`] with an explicit coplanarity/visibility tolerance.
pub fn convex_hull_3d_with_tolerance(points: &[Coord3], tolerance: f64) -> Result<ConvexHull3> {
    if points.is_empty() {
        return Err(Error::InvalidInput(
            "a 3D convex hull requires at least one point",
        ));
    }
    let unique = dedupe(points, tolerance);
    if unique.len() == 1 {
        return Ok(ConvexHull3 {
            vertices: unique,
            faces: Vec::new(),
            degenerate: Degeneracy::Point,
        });
    }

    let scale = scale_of(&unique);
    let epsilon = (tolerance * scale).max(f64::MIN_POSITIVE);

    let Some((i0, i1)) = extreme_pair(&unique) else {
        return Ok(ConvexHull3 {
            vertices: vec![unique[0]],
            faces: Vec::new(),
            degenerate: Degeneracy::Point,
        });
    };

    let Some(i2) = farthest_from_line(&unique, i0, i1, epsilon) else {
        return Ok(ConvexHull3 {
            vertices: vec![unique[i0], unique[i1]],
            faces: Vec::new(),
            degenerate: Degeneracy::Collinear,
        });
    };

    let Some(i3) = farthest_from_plane(&unique, i0, i1, i2, epsilon) else {
        return coplanar_hull(&unique, i0, i1, i2, epsilon);
    };

    build_hull(&unique, [i0, i1, i2, i3], epsilon)
}

/// Convenience wrapper: hull of any geometry's coordinates, preserving Z.
///
/// Coordinates without Z are treated as `z = 0.0`.
pub fn geometry_convex_hull_3d(geometry: &Geometry) -> Result<Geometry> {
    let points = geometry_points_3d(geometry)?;
    convex_hull_3d(&points)?.to_geometry()
}

/// Extract every coordinate of a geometry as XYZ, defaulting missing Z to zero.
pub fn geometry_points_3d(geometry: &Geometry) -> Result<Vec<Coord3>> {
    Ok(crate::get_coordinates(geometry)?
        .into_iter()
        .map(|coord| Coord3::new(coord.x, coord.y, coord.z.unwrap_or(0.0)))
        .collect())
}

/// 3D axis-aligned bounds of a geometry.
pub fn bounds_3d(geometry: &Geometry) -> Result<Option<Bounds3>> {
    Ok(Bounds3::from_points(&geometry_points_3d(geometry)?))
}

/// Ritter's bounding sphere, refined by a second pass. This is the 3D analogue
/// of `minimum_bounding_circle` - near-minimal and O(n).
pub fn bounding_sphere(points: &[Coord3]) -> Option<Sphere> {
    let first = *points.first()?;
    if points.len() == 1 {
        return Some(Sphere {
            center: first,
            radius: 0.0,
        });
    }

    let start = Vec3::from(first);
    let far_a = points
        .iter()
        .copied()
        .max_by(|a, b| {
            start
                .distance(Vec3::from(*a))
                .total_cmp(&start.distance(Vec3::from(*b)))
        })
        .map(Vec3::from)?;
    let far_b = points
        .iter()
        .copied()
        .max_by(|a, b| {
            far_a
                .distance(Vec3::from(*a))
                .total_cmp(&far_a.distance(Vec3::from(*b)))
        })
        .map(Vec3::from)?;

    let mut center = far_a.plus(far_b).scale(0.5);
    let mut radius = far_a.distance(far_b) / 2.0;

    for point in points {
        let point = Vec3::from(*point);
        let distance = center.distance(point);
        if distance > radius {
            let new_radius = (radius + distance) / 2.0;
            let shift = (distance - new_radius) / distance;
            center = center.plus(point.minus(center).scale(shift));
            radius = new_radius;
        }
    }

    Some(Sphere {
        center: center.into(),
        radius,
    })
}

/// Newell's method: the best-fit plane normal for a 3D polygon ring.
pub fn newell_normal(ring: &[Coord3]) -> Option<Vec3> {
    if ring.len() < 3 {
        return None;
    }
    let mut normal = Vec3::new(0.0, 0.0, 0.0);
    for (current, next) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal.normalized()
}

/// True area of a planar polygon embedded in 3D. GEOS's `area` projects to XY
/// and therefore under-reports tilted surfaces; this does not.
pub fn planar_area_3d(ring: &[Coord3]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut total = Vec3::new(0.0, 0.0, 0.0);
    for (current, next) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        total = total.plus(Vec3::from(*current).cross(Vec3::from(*next)));
    }
    total.norm() / 2.0
}

/// True 3D length of a polyline. GEOS's `length` ignores Z.
pub fn length_3d(coords: &[Coord3]) -> f64 {
    coords
        .windows(2)
        .map(|pair| Vec3::from(pair[0]).distance(Vec3::from(pair[1])))
        .sum()
}

/// Whether all points lie on a common plane within `tolerance`.
pub fn is_planar(points: &[Coord3], tolerance: f64) -> bool {
    match convex_hull_3d_with_tolerance(points, tolerance) {
        Ok(hull) => matches!(
            hull.degeneracy(),
            Degeneracy::Coplanar | Degeneracy::Collinear | Degeneracy::Point
        ),
        Err(_) => true,
    }
}

/// Shortest 3D distance between two point sets' convex positions, computed as
/// the minimum pairwise vertex distance. Useful as a cheap 3D proximity check
/// alongside GEOS's planar `distance`.
pub fn min_distance_3d(left: &[Coord3], right: &[Coord3]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for a in left {
        for b in right {
            let distance = Vec3::from(*a).distance(Vec3::from(*b));
            best = Some(match best {
                Some(current) if current <= distance => current,
                _ => distance,
            });
        }
    }
    best
}

fn scale_of(points: &[Coord3]) -> f64 {
    points
        .iter()
        .map(|p| p.x.abs().max(p.y.abs()).max(p.z.abs()))
        .fold(1.0_f64, f64::max)
}

fn dedupe(points: &[Coord3], tolerance: f64) -> Vec<Coord3> {
    let scale = scale_of(points);
    let epsilon = (tolerance * scale).max(f64::MIN_POSITIVE);
    let mut unique: Vec<Coord3> = Vec::with_capacity(points.len());
    for point in points {
        if !unique
            .iter()
            .any(|kept| Vec3::from(*kept).distance(Vec3::from(*point)) <= epsilon)
        {
            unique.push(*point);
        }
    }
    unique
}

fn extreme_pair(points: &[Coord3]) -> Option<(usize, usize)> {
    let mut best = (0usize, 0usize);
    let mut best_distance = 0.0;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = Vec3::from(points[i]).distance(Vec3::from(points[j]));
            if distance > best_distance {
                best_distance = distance;
                best = (i, j);
            }
        }
    }
    (best_distance > 0.0).then_some(best)
}

fn farthest_from_line(points: &[Coord3], i0: usize, i1: usize, epsilon: f64) -> Option<usize> {
    let a = Vec3::from(points[i0]);
    let direction = Vec3::from(points[i1]).minus(a).normalized()?;
    let mut best = None;
    let mut best_distance = epsilon;
    for (index, point) in points.iter().enumerate() {
        if index == i0 || index == i1 {
            continue;
        }
        let offset = Vec3::from(*point).minus(a);
        let distance = offset.minus(direction.scale(offset.dot(direction))).norm();
        if distance > best_distance {
            best_distance = distance;
            best = Some(index);
        }
    }
    best
}

fn farthest_from_plane(
    points: &[Coord3],
    i0: usize,
    i1: usize,
    i2: usize,
    epsilon: f64,
) -> Option<usize> {
    let normal = plane_normal(points, i0, i1, i2)?;
    let origin = Vec3::from(points[i0]);
    let mut best = None;
    let mut best_distance = epsilon;
    for (index, point) in points.iter().enumerate() {
        if index == i0 || index == i1 || index == i2 {
            continue;
        }
        let distance = normal.dot(Vec3::from(*point).minus(origin)).abs();
        if distance > best_distance {
            best_distance = distance;
            best = Some(index);
        }
    }
    best
}

fn plane_normal(points: &[Coord3], i0: usize, i1: usize, i2: usize) -> Option<Vec3> {
    let a = Vec3::from(points[i0]);
    Vec3::from(points[i1])
        .minus(a)
        .cross(Vec3::from(points[i2]).minus(a))
        .normalized()
}

fn coplanar_hull(
    points: &[Coord3],
    i0: usize,
    i1: usize,
    i2: usize,
    epsilon: f64,
) -> Result<ConvexHull3> {
    let normal = plane_normal(points, i0, i1, i2)
        .ok_or(Error::InvalidInput("degenerate plane for coplanar hull"))?;
    let origin = Vec3::from(points[i0]);
    let u = Vec3::from(points[i1])
        .minus(origin)
        .normalized()
        .ok_or(Error::InvalidInput("degenerate axis for coplanar hull"))?;
    let v = normal.cross(u);

    let projected: Vec<(f64, f64)> = points
        .iter()
        .map(|point| {
            let offset = Vec3::from(*point).minus(origin);
            (offset.dot(u), offset.dot(v))
        })
        .collect();

    let hull_indices = convex_hull_2d(&projected, epsilon);
    let vertices: Vec<Coord3> = hull_indices.iter().map(|index| points[*index]).collect();
    let faces = (1..vertices.len().saturating_sub(1))
        .map(|index| Triangle {
            a: 0,
            b: index,
            c: index + 1,
        })
        .collect();

    Ok(ConvexHull3 {
        vertices,
        faces,
        degenerate: Degeneracy::Coplanar,
    })
}

fn convex_hull_2d(points: &[(f64, f64)], epsilon: f64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|a, b| {
        points[*a]
            .0
            .total_cmp(&points[*b].0)
            .then(points[*a].1.total_cmp(&points[*b].1))
    });

    let cross = |o: usize, a: usize, b: usize| {
        (points[a].0 - points[o].0) * (points[b].1 - points[o].1)
            - (points[a].1 - points[o].1) * (points[b].0 - points[o].0)
    };

    let mut lower: Vec<usize> = Vec::new();
    for &index in &order {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], index) <= epsilon
        {
            lower.pop();
        }
        lower.push(index);
    }

    let mut upper: Vec<usize> = Vec::new();
    for &index in order.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], index) <= epsilon
        {
            upper.pop();
        }
        upper.push(index);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn build_hull(points: &[Coord3], seed: [usize; 4], epsilon: f64) -> Result<ConvexHull3> {
    let [i0, i1, i2, i3] = seed;
    let centroid = Vec3::from(points[i0])
        .plus(Vec3::from(points[i1]))
        .plus(Vec3::from(points[i2]))
        .plus(Vec3::from(points[i3]))
        .scale(0.25);

    let mut faces: Vec<Triangle> = vec![
        Triangle {
            a: i0,
            b: i1,
            c: i2,
        },
        Triangle {
            a: i0,
            b: i1,
            c: i3,
        },
        Triangle {
            a: i0,
            b: i2,
            c: i3,
        },
        Triangle {
            a: i1,
            b: i2,
            c: i3,
        },
    ];
    for face in &mut faces {
        orient_outward(points, face, centroid);
    }

    for (index, point) in points.iter().enumerate() {
        if seed.contains(&index) {
            continue;
        }
        let point = Vec3::from(*point);
        let visible: Vec<usize> = faces
            .iter()
            .enumerate()
            .filter(|(_, face)| is_visible(points, **face, point, epsilon))
            .map(|(position, _)| position)
            .collect();
        if visible.is_empty() {
            continue;
        }

        let horizon = horizon_edges(&faces, &visible);
        for position in visible.iter().rev() {
            faces.swap_remove(*position);
        }
        for (a, b) in horizon {
            let mut face = Triangle { a, b, c: index };
            orient_outward(points, &mut face, centroid);
            faces.push(face);
        }
    }

    let mut used: Vec<usize> = faces
        .iter()
        .flat_map(|face| [face.a, face.b, face.c])
        .collect();
    used.sort_unstable();
    used.dedup();

    let vertices: Vec<Coord3> = used.iter().map(|index| points[*index]).collect();
    let remap = |index: usize| used.binary_search(&index).expect("hull vertex is indexed");
    let faces = faces
        .into_iter()
        .map(|face| Triangle {
            a: remap(face.a),
            b: remap(face.b),
            c: remap(face.c),
        })
        .collect();

    Ok(ConvexHull3 {
        vertices,
        faces,
        degenerate: Degeneracy::None,
    })
}

fn face_plane(points: &[Coord3], face: Triangle) -> Option<(Vec3, Vec3)> {
    let a = Vec3::from(points[face.a]);
    let normal = Vec3::from(points[face.b])
        .minus(a)
        .cross(Vec3::from(points[face.c]).minus(a))
        .normalized()?;
    Some((a, normal))
}

fn orient_outward(points: &[Coord3], face: &mut Triangle, interior: Vec3) {
    if let Some((origin, normal)) = face_plane(points, *face)
        && normal.dot(interior.minus(origin)) > 0.0
    {
        std::mem::swap(&mut face.b, &mut face.c);
    }
}

fn is_visible(points: &[Coord3], face: Triangle, point: Vec3, epsilon: f64) -> bool {
    match face_plane(points, face) {
        Some((origin, normal)) => normal.dot(point.minus(origin)) > epsilon,
        None => false,
    }
}

fn horizon_edges(faces: &[Triangle], visible: &[usize]) -> Vec<(usize, usize)> {
    let mut counts: Vec<((usize, usize), usize)> = Vec::new();
    for position in visible {
        let face = faces[*position];
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            let key = if a < b { (a, b) } else { (b, a) };
            match counts.iter_mut().find(|(edge, _)| *edge == key) {
                Some((_, count)) => *count += 1,
                None => counts.push((key, 1)),
            }
        }
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(edge, _)| edge)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Vec<Coord3> {
        vec![
            Coord3::new(0.0, 0.0, 0.0),
            Coord3::new(1.0, 0.0, 0.0),
            Coord3::new(1.0, 1.0, 0.0),
            Coord3::new(0.0, 1.0, 0.0),
            Coord3::new(0.0, 0.0, 1.0),
            Coord3::new(1.0, 0.0, 1.0),
            Coord3::new(1.0, 1.0, 1.0),
            Coord3::new(0.0, 1.0, 1.0),
        ]
    }

    #[test]
    fn hull_of_a_cube_has_unit_volume_and_area() {
        let hull = convex_hull_3d(&cube()).unwrap();
        assert_eq!(hull.degeneracy(), Degeneracy::None);
        assert_eq!(hull.vertices().len(), 8);
        assert!((hull.volume() - 1.0).abs() < 1e-9);
        assert!((hull.surface_area() - 6.0).abs() < 1e-9);
        let centroid = hull.centroid().unwrap();
        assert!((centroid.x - 0.5).abs() < 1e-9);
        assert!((centroid.z - 0.5).abs() < 1e-9);
    }

    #[test]
    fn interior_points_do_not_change_the_hull() {
        let mut points = cube();
        points.push(Coord3::new(0.5, 0.5, 0.5));
        points.push(Coord3::new(0.25, 0.75, 0.5));
        let hull = convex_hull_3d(&points).unwrap();
        assert_eq!(hull.vertices().len(), 8);
        assert!((hull.volume() - 1.0).abs() < 1e-9);
        assert!(hull.contains(Coord3::new(0.5, 0.5, 0.5), 1e-9));
        assert!(!hull.contains(Coord3::new(2.0, 0.5, 0.5), 1e-9));
    }

    #[test]
    fn coplanar_input_is_reported_and_triangulated() {
        let points = vec![
            Coord3::new(0.0, 0.0, 5.0),
            Coord3::new(2.0, 0.0, 5.0),
            Coord3::new(2.0, 2.0, 5.0),
            Coord3::new(0.0, 2.0, 5.0),
            Coord3::new(1.0, 1.0, 5.0),
        ];
        let hull = convex_hull_3d(&points).unwrap();
        assert_eq!(hull.degeneracy(), Degeneracy::Coplanar);
        assert_eq!(hull.vertices().len(), 4);
        assert_eq!(hull.volume(), 0.0);
        assert!((hull.surface_area() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn collinear_and_single_point_inputs_degrade_gracefully() {
        let collinear = vec![
            Coord3::new(0.0, 0.0, 0.0),
            Coord3::new(1.0, 1.0, 1.0),
            Coord3::new(2.0, 2.0, 2.0),
        ];
        assert_eq!(
            convex_hull_3d(&collinear).unwrap().degeneracy(),
            Degeneracy::Collinear
        );

        let single = vec![Coord3::new(3.0, 3.0, 3.0), Coord3::new(3.0, 3.0, 3.0)];
        assert_eq!(
            convex_hull_3d(&single).unwrap().degeneracy(),
            Degeneracy::Point
        );
    }

    #[test]
    fn tilted_surface_area_and_length_respect_z() {
        let ring = vec![
            Coord3::new(0.0, 0.0, 0.0),
            Coord3::new(1.0, 0.0, 1.0),
            Coord3::new(1.0, 1.0, 1.0),
            Coord3::new(0.0, 1.0, 0.0),
        ];
        assert!((planar_area_3d(&ring) - 2.0_f64.sqrt()).abs() < 1e-9);
        assert!((length_3d(&ring) - (2.0_f64.sqrt() + 1.0 + 2.0_f64.sqrt())).abs() < 1e-9);
        let normal = newell_normal(&ring).unwrap();
        assert!((normal.norm() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn bounding_sphere_and_bounds_cover_all_points() {
        let points = cube();
        let bounds = Bounds3::from_points(&points).unwrap();
        assert_eq!(bounds.volume(), 1.0);
        assert!(bounds.contains(Coord3::new(0.5, 0.5, 0.5)));

        let sphere = bounding_sphere(&points).unwrap();
        for point in &points {
            let distance = Vec3::from(sphere.center).distance(Vec3::from(*point));
            assert!(distance <= sphere.radius + 1e-9);
        }
    }
}
