//! Use your own coordinate type directly with geoxi.
//!
//! Implement [`geoxi::CoordLike`] once and your type becomes a first-class
//! argument to every geometry constructor. No conversion function is ever
//! called, and no intermediate `Vec` is allocated.
//!
//! Run with: `cargo run --example bring_your_own_coords`

use geoxi::{CoordLike, Geometry, no_holes};

/// A survey station: a named 3D fix with an instrument height.
#[derive(Debug, Clone, Copy)]
struct Station {
    easting: f64,
    northing: f64,
    elevation: f64,
}

impl CoordLike for Station {
    #[inline]
    fn x(&self) -> f64 {
        self.easting
    }

    #[inline]
    fn y(&self) -> f64 {
        self.northing
    }

    #[inline]
    fn z(&self) -> Option<f64> {
        Some(self.elevation)
    }

    #[inline]
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
        Station {
            easting: x,
            northing: y,
            elevation: z.unwrap_or(0.0),
        }
    }
}

fn main() -> geoxi::Result<()> {
    let traverse = [
        Station {
            easting: 0.0,
            northing: 0.0,
            elevation: 100.0,
        },
        Station {
            easting: 30.0,
            northing: 0.0,
            elevation: 112.0,
        },
        Station {
            easting: 30.0,
            northing: 40.0,
            elevation: 108.0,
        },
        Station {
            easting: 0.0,
            northing: 40.0,
            elevation: 96.0,
        },
        Station {
            easting: 0.0,
            northing: 0.0,
            elevation: 100.0,
        },
    ];

    // Your type goes straight in.
    let boundary = Geometry::polygon(&traverse, no_holes())?;
    println!("planimetric area: {:.1} m^2", boundary.area()?);

    // Native 3D algorithms read the same geometry back out as Coord3.
    let ring: Vec<geoxi::Coord3> = boundary.exterior_ring()?.coords()?;
    println!("true 3D area:     {:.1} m^2", geoxi::planar_area_3d(&ring));
    println!("2D length: {:.2} m", boundary.exterior_ring()?.length()?);
    println!("3D length: {:.2} m", geoxi::length_3d(&ring));

    // ...and comes straight back out as your own type, too.
    let recovered: Vec<Station> = boundary.exterior_ring()?.coords()?;
    println!("first station back: {:?}", recovered[0]);

    // The same API accepts plain tuples and arrays, so quick literals stay easy.
    let quick = Geometry::line_string(&[(0.0, 0.0), (1.0, 1.0)])?;
    println!("tuple line length: {:.4}", quick.length()?);

    let with_z = Geometry::line_string(&[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]])?;
    println!("array line is 3D: {}", with_z.has_z()?);

    Ok(())
}
