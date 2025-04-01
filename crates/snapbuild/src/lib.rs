use std::collections::BTreeMap;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rkyv::{Archive, Deserialize, Serialize};
use s2::cellid::CellID;

// We need a wrapper for CellID that can be serialized with rkyv
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, PartialOrd, Ord))]
pub struct SerializableCellID(u64);

impl From<CellID> for SerializableCellID {
    fn from(cell: CellID) -> Self {
        SerializableCellID(cell.0)
    }
}

impl From<SerializableCellID> for CellID {
    fn from(cell: SerializableCellID) -> Self {
        CellID(cell.0)
    }
}

// To allow ordering of f64 values in BinaryHeap
#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

// Keep it simple - don't derive Archive traits for now
#[derive(Clone)]
pub struct S2PointIndex<T> 
where 
    T: Clone + Ord,
{
    points: BTreeMap<SerializableCellID, T>,
}

impl<T> S2PointIndex<T> 
where 
    T: Clone + Ord,
{
    /// Create a new empty S2PointIndex
    pub fn new() -> Self {
        Self {
            points: BTreeMap::new(),
        }
    }

    /// Add a cell with associated data to the index
    pub fn add(&mut self, cell: CellID, data: T) {
        self.points.insert(cell.into(), data);
    }

    /// Find the closest N points to the target cell
    pub fn find_closest(&self, target: CellID, n: usize) -> Vec<(CellID, &T)> {
        if self.points.is_empty() || n == 0 {
            return Vec::new();
        }

        let target_point = target.raw_point();
        let mut heap = BinaryHeap::new();

        for (serializable_cell, data) in &self.points {
            let cell: CellID = (*serializable_cell).into();
            let cell_point = cell.raw_point();
            let distance = distance_between_points(target_point, cell_point);
            heap.push(Reverse((OrderedFloat(distance), cell, data)));
        }

        let mut results = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(Reverse((_, cell, data))) = heap.pop() {
                results.push((cell, data));
            } else {
                break;
            }
        }

        results
    }

    /// Find all points within a given maximum distance (in radians)
    pub fn find_points_within_distance(&self, target: CellID, max_distance_radians: f64) -> Vec<(CellID, &T, f64)> {
        if self.points.is_empty() {
            return Vec::new();
        }

        let target_point = target.raw_point();
        let mut results = Vec::new();

        for (serializable_cell, data) in &self.points {
            let cell: CellID = (*serializable_cell).into();
            let cell_point = cell.raw_point();
            let distance = distance_between_points(target_point, cell_point);
            if distance <= max_distance_radians {
                results.push((cell, data, distance));
            }
        }

        // Sort results by distance
        results.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Get the number of points in the index
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl<T> Default for S2PointIndex<T> 
where 
    T: Clone + Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate the distance between two S2 points (in radians)
fn distance_between_points(a: s2::r3::vector::Vector, b: s2::r3::vector::Vector) -> f64 {
    // Calculate the angle between the two points
    a.angle(&b).rad()
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2::latlng::LatLng;
    
    #[test]
    fn test_add_and_find() {
        let mut index = S2PointIndex::new();
        
        // Add some points
        let cell1 = CellID::from(LatLng::from_degrees(37.7749, -122.4194)); // San Francisco
        let cell2 = CellID::from(LatLng::from_degrees(34.0522, -118.2437)); // Los Angeles
        let cell3 = CellID::from(LatLng::from_degrees(40.7128, -74.0060));  // New York
        
        index.add(cell1, 1u32);
        index.add(cell2, 2u32);
        index.add(cell3, 3u32);
        
        // Find closest 2 to San Francisco
        let target = CellID::from(LatLng::from_degrees(37.7749, -122.4194));
        let closest = index.find_closest(target, 2);
        
        assert_eq!(closest.len(), 2);
        assert_eq!(closest[0].0, cell1); // San Francisco should be first
    }
    
    // We'll implement proper serialization in a future PR
    // #[test]
    // fn test_serialization() {
    //     // ...
    // }
}
