//! `GeometryArray`, the batch/vectorised counterpart to `Geometry`.

use crate::internal_prelude::*;

#[derive(Debug, Clone, Default)]
pub struct GeometryArray {
    geometries: Vec<Geometry>,
}

impl GeometryArray {
    pub fn new(geometries: Vec<Geometry>) -> Self {
        Self { geometries }
    }

    pub fn as_slice(&self) -> &[Geometry] {
        &self.geometries
    }

    pub fn into_vec(self) -> Vec<Geometry> {
        self.geometries
    }

    pub fn len(&self) -> usize {
        self.geometries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.geometries.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Geometry> {
        self.geometries.get(index)
    }

    pub fn map<F>(&self, operation: F) -> Result<Self>
    where
        F: Fn(&Geometry) -> Result<Geometry>,
    {
        self.geometries.iter().map(operation).collect()
    }

    pub fn map_binary<F>(&self, other: &Self, operation: F) -> Result<Self>
    where
        F: Fn(&Geometry, &Geometry) -> Result<Geometry>,
    {
        if self.len() != other.len() {
            return Err(Error::InvalidInput(
                "binary array operations require equal lengths",
            ));
        }
        self.geometries
            .iter()
            .zip(&other.geometries)
            .map(|(left, right)| operation(left, right))
            .collect()
    }

    pub fn map_scalar<F>(&self, other: &Geometry, operation: F) -> Result<Self>
    where
        F: Fn(&Geometry, &Geometry) -> Result<Geometry>,
    {
        self.geometries
            .iter()
            .map(|geometry| operation(geometry, other))
            .collect()
    }

    pub fn areas(&self) -> Result<Vec<f64>> {
        self.geometries.iter().map(Geometry::area).collect()
    }

    pub fn lengths(&self) -> Result<Vec<f64>> {
        self.geometries.iter().map(Geometry::length).collect()
    }

    pub fn centroids(&self) -> Result<Self> {
        self.map(Geometry::centroid)
    }

    pub fn is_valid(&self) -> Result<Vec<bool>> {
        self.geometries.iter().map(Geometry::is_valid).collect()
    }

    pub fn buffer(&self, distance: f64, quad_segs: i32) -> Result<Self> {
        self.map(|geometry| geometry.buffer(distance, quad_segs))
    }

    pub fn intersection(&self, other: &Self) -> Result<Self> {
        self.map_binary(other, Geometry::intersection)
    }

    pub fn union(&self, other: &Self) -> Result<Self> {
        self.map_binary(other, Geometry::union)
    }

    pub fn difference(&self, other: &Self) -> Result<Self> {
        self.map_binary(other, Geometry::difference)
    }

    pub fn bounds(&self) -> Result<Vec<Bounds>> {
        self.geometries.iter().map(Geometry::bounds).collect()
    }

    pub fn total_bounds(&self) -> Result<Option<Bounds>> {
        let mut iter = self.geometries.iter();
        let Some(first) = iter.next() else {
            return Ok(None);
        };
        let mut total = first.bounds()?;
        for geometry in iter {
            let bounds = geometry.bounds()?;
            total.xmin = total.xmin.min(bounds.xmin);
            total.ymin = total.ymin.min(bounds.ymin);
            total.xmax = total.xmax.max(bounds.xmax);
            total.ymax = total.ymax.max(bounds.ymax);
        }
        Ok(Some(total))
    }

    pub fn intersects(&self, other: &Self) -> Result<Vec<bool>> {
        if self.len() != other.len() {
            return Err(Error::InvalidInput(
                "binary array operations require equal lengths",
            ));
        }
        self.geometries
            .iter()
            .zip(&other.geometries)
            .map(|(left, right)| left.intersects(right))
            .collect()
    }

    pub fn unary_union(&self) -> Result<Geometry> {
        union_all(self.geometries.clone())
    }
}

impl From<Vec<Geometry>> for GeometryArray {
    fn from(geometries: Vec<Geometry>) -> Self {
        Self::new(geometries)
    }
}

impl FromIterator<Geometry> for GeometryArray {
    fn from_iter<T: IntoIterator<Item = Geometry>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}
