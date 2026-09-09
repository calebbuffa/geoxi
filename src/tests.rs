//! Integration-style tests over the public API.

use crate::internal_prelude::*;
use crate::three_d;
use crate::three_d::{bounds_3d, convex_hull_3d, geometry_convex_hull_3d};

#[test]
fn exposes_core_geometry_operations() {
    let a = box_(0.0, 0.0, 2.0, 2.0).unwrap();
    let b = box_(1.0, 1.0, 3.0, 3.0).unwrap();

    assert!(a.intersects(&b).unwrap());
    assert_eq!(a.area().unwrap(), 4.0);
    assert_eq!(a.intersection(&b).unwrap().area().unwrap(), 1.0);
    assert!(a.convex_hull().unwrap().is_valid().unwrap());
    assert!(a.buffer(1.0, 8).unwrap().area().unwrap() > a.area().unwrap());
}

#[test]
fn splits_lines_and_polygons() {
    let line = Geometry::from_wkt("LINESTRING (0 0, 10 0)").unwrap();
    let blade = Geometry::from_wkt("POINT (5 0)").unwrap();
    let pieces = split(&line, &blade).unwrap();
    assert_eq!(pieces.num_geometries().unwrap(), 2);

    let polygon = box_(0.0, 0.0, 10.0, 10.0).unwrap();
    let cutter = Geometry::from_wkt("LINESTRING (5 -1, 5 11)").unwrap();
    let halves = split(&polygon, &cutter).unwrap();
    assert_eq!(halves.num_geometries().unwrap(), 2);
    assert!((halves.area().unwrap() - 100.0).abs() < 1e-9);
}

#[test]
fn orientation_helpers_agree_with_ring_winding() {
    let polygon = box_(0.0, 0.0, 1.0, 1.0).unwrap();
    let ccw = orient(&polygon, 1.0).unwrap();
    assert!(is_ccw(&ccw.exterior_ring().unwrap()).unwrap());

    let cw = orient(&polygon, -1.0).unwrap();
    assert!(!is_ccw(&cw.exterior_ring().unwrap()).unwrap());
}

#[test]
fn accessors_and_empty_constructors_behave() {
    let polygon =
        Geometry::from_wkt("POLYGON ((0 0, 4 0, 4 4, 0 4, 0 0), (1 1, 2 1, 2 2, 1 2, 1 1))")
            .unwrap();
    assert_eq!(get_num_interior_rings(&polygon).unwrap(), 1);
    assert_eq!(get_type_id(&polygon).unwrap(), GeometryType::Polygon);
    assert_eq!(get_dimensions(&polygon).unwrap(), 2);
    assert!(intersects_xy(&polygon, 0.5, 0.5).unwrap());
    assert!(!contains_xy(&polygon, 1.5, 1.5).unwrap());

    let stamped = set_srid(&polygon, 4326).unwrap();
    assert_eq!(get_srid(&stamped), 4326);

    assert!(empty(GeometryType::Polygon).unwrap().is_empty().unwrap());
    assert!(empty(GeometryType::MultiPoint).unwrap().is_empty().unwrap());
}

#[test]
fn equality_helpers_report_differences() {
    let a = box_(0.0, 0.0, 1.0, 1.0).unwrap();
    let b = box_(0.0, 0.0, 1.0, 1.0).unwrap();
    assert!(geometries_equal(&a, &b, 0.0).unwrap());
    assert_geometries_equal(&a, &b, 1e-9).unwrap();

    let c = box_(0.0, 0.0, 2.0, 2.0).unwrap();
    assert!(!geometries_equal(&a, &c, 1e-9).unwrap());
    assert!(assert_geometries_equal(&a, &c, 1e-9).is_err());
}

#[test]
fn symmetric_difference_all_folds_inputs() {
    let geometries = vec![
        box_(0.0, 0.0, 2.0, 2.0).unwrap(),
        box_(1.0, 0.0, 3.0, 2.0).unwrap(),
    ];
    let result = symmetric_difference_all(geometries).unwrap().unwrap();
    assert!((result.area().unwrap() - 4.0).abs() < 1e-9);
    assert!(symmetric_difference_all(Vec::new()).unwrap().is_none());
}

#[test]
fn coverage_operations_validate_and_simplify() {
    let coverage = Geometry::collection(
        CollectionType::GeometryCollection,
        vec![
            box_(0.0, 0.0, 1.0, 1.0).unwrap(),
            box_(1.0, 0.0, 2.0, 1.0).unwrap(),
        ],
    )
    .unwrap();

    let validity = coverage_is_valid(&coverage, 0.0).unwrap();
    assert!(validity.is_valid);

    let simplified = coverage_simplify(&coverage, 0.1, true).unwrap();
    assert_eq!(simplified.num_geometries().unwrap(), 2);
}

#[test]
fn three_d_hull_bridges_into_geometry() {
    let cloud =
        Geometry::from_wkt("MULTIPOINT Z (0 0 0, 1 0 0, 1 1 0, 0 1 0, 0 0 1, 1 0 1, 1 1 1, 0 1 1)")
            .unwrap();

    let planar = cloud.convex_hull().unwrap();
    assert!(!planar.has_z().unwrap() || planar.area().unwrap() == 1.0);

    let solid = geometry_convex_hull_3d(&cloud).unwrap();
    assert!(solid.has_z().unwrap());
    assert_eq!(solid.geometry_type().unwrap(), GeometryType::MultiPolygon);

    let hull = convex_hull_3d(&three_d::geometry_points_3d(&cloud).unwrap()).unwrap();
    assert!((hull.volume() - 1.0).abs() < 1e-9);

    let bounds = bounds_3d(&cloud).unwrap().unwrap();
    assert_eq!(bounds.zmax, 1.0);
}

#[test]
fn strtree_returns_candidate_indices() {
    let geoms = vec![
        point(0.0, 0.0).unwrap(),
        point(5.0, 5.0).unwrap(),
        point(1.0, 1.0).unwrap(),
    ];
    let tree = STRtree::new(geoms, 10).unwrap();
    let query = box_(-0.5, -0.5, 1.5, 1.5).unwrap();
    let mut hits = tree
        .query_predicate(&query, BinaryPredicate::Intersects)
        .unwrap();
    hits.sort_unstable();
    assert_eq!(hits, vec![0, 2]);
}

#[test]
fn geometry_array_supports_batch_measurements_and_parts() {
    let array = GeometryArray::from(vec![
        box_(0.0, 0.0, 2.0, 2.0).unwrap(),
        box_(0.0, 0.0, 3.0, 1.0).unwrap(),
    ]);
    assert_eq!(array.areas().unwrap(), vec![4.0, 3.0]);
    assert_eq!(array.total_bounds().unwrap().unwrap().xmax, 3.0);

    let polygon = box_(0.0, 0.0, 2.0, 2.0).unwrap();
    assert_eq!(polygon.rings().unwrap().len(), 1);
    assert_eq!(get_parts(&polygon).unwrap().len(), 1);
}

#[test]
fn geometries_on_one_thread_share_a_context() {
    let a = Geometry::point(0.0, 0.0).unwrap();
    let b = Geometry::rectangle(-1.0, -1.0, 1.0, 1.0).unwrap();

    assert!(a.context().same(b.context()));
    assert!(b.contains(&a).unwrap());
}

#[test]
fn distinct_threads_get_distinct_contexts() {
    let here = Geometry::point(0.0, 0.0).unwrap();
    let there = std::thread::spawn(|| Geometry::point(1.0, 1.0).unwrap())
        .join()
        .unwrap();

    assert!(!here.context().same(there.context()));

    // A geometry built on another thread still operates correctly here: binary
    // ops migrate the foreign operand rather than mixing GEOS handles.
    assert_eq!(here.distance(&there).unwrap(), 2f64.sqrt());
    assert!(!here.intersects(&there).unwrap());
}

#[test]
fn rehome_moves_a_foreign_geometry_onto_this_thread() {
    let foreign = std::thread::spawn(|| {
        let mut g = Geometry::from_wkt("POINT Z (1 2 3)").unwrap();
        g.set_srid(4326);
        g
    })
    .join()
    .unwrap();

    let local = Geometry::point(0.0, 0.0).unwrap();
    assert!(!foreign.context().same(local.context()));

    let rehomed = foreign.rehome().unwrap();

    assert!(rehomed.context().same(local.context()));
    assert_eq!(rehomed.srid(), 4326);
    assert!(rehomed.has_z().unwrap());
    assert!(foreign.equals(&rehomed).unwrap());
}

#[test]
fn geometry_outliving_its_creating_thread_drops_cleanly() {
    let orphan =
        std::thread::spawn(|| Geometry::from_wkt("POLYGON ((0 0, 1 0, 1 1, 0 0))").unwrap())
            .join()
            .unwrap();
    assert_eq!(orphan.area().unwrap(), 0.5);
    drop(orphan);
}

#[test]
fn collections_absorb_members_from_foreign_contexts() {
    let members = vec![
        Geometry::point(0.0, 0.0).unwrap(),
        Geometry::point(1.0, 1.0).unwrap(),
    ];
    let multi = Geometry::collection(CollectionType::MultiPoint, members).unwrap();
    assert_eq!(multi.num_geometries().unwrap(), 2);
}

#[test]
fn strtree_and_prepared_accept_foreign_geometries() {
    let tree = STRtree::new(
        vec![
            box_(0.0, 0.0, 1.0, 1.0).unwrap(),
            box_(5.0, 5.0, 6.0, 6.0).unwrap(),
        ],
        10,
    )
    .unwrap();
    let probe = box_(0.5, 0.5, 0.6, 0.6).unwrap();
    assert_eq!(tree.query(&probe).unwrap(), vec![0]);

    let prepared = box_(0.0, 0.0, 1.0, 1.0).unwrap().prepared().unwrap();
    assert!(prepared.contains(&probe).unwrap());
}

#[test]
fn coord3_is_layout_compatible_with_a_plain_f64_triple() {
    assert_eq!(size_of::<Coord3>(), size_of::<[f64; 3]>());
    assert_eq!(align_of::<Coord3>(), align_of::<[f64; 3]>());
    assert_eq!(size_of::<three_d::Vec3>(), size_of::<[f64; 3]>());
}
