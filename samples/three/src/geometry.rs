//! three.js's `BufferGeometry` (`src/core/BufferGeometry.js`) with
//! `BoxGeometry` and `SphereGeometry` (`src/geometries/`).
//!
//! The generators compute in `f64` in three.js's order and store each
//! attribute as `f32`, as `Float32BufferAttribute` does. `Math.sin` and
//! `Math.cos` are `jsmath`'s, rounded as Chromium rounds them.

use crate::jsmath;
use crate::math::Vector3;

/// An index buffer: `setIndex(array)` stores 32-bit indices only when one
/// of them is 65535 or more (`arrayNeedsUint32`).
#[derive(Debug, Clone, PartialEq)]
pub enum Index {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Index {
    fn from_indices(indices: &[u32]) -> Index {
        if indices.iter().any(|&i| i >= 65535) {
            Index::U32(indices.to_vec())
        } else {
            Index::U16(indices.iter().map(|&i| i as u16).collect())
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Index::U16(v) => v.len(),
            Index::U32(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The indices as bytes, in the buffer's own width.
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Index::U16(v) => v.iter().flat_map(|i| i.to_le_bytes()).collect(),
            Index::U32(v) => v.iter().flat_map(|i| i.to_le_bytes()).collect(),
        }
    }
}

/// An indexed geometry with `position`, `normal` and `uv` attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct BufferGeometry {
    pub index: Index,
    pub position: Vec<f32>,
    pub normal: Vec<f32>,
    pub uv: Vec<f32>,
}

fn to_f32(values: &[f64]) -> Vec<f32> {
    values.iter().map(|&v| v as f32).collect()
}

impl BufferGeometry {
    /// `computeBoundingSphere()`'s center: the center of the bounding box
    /// of the stored (`f32`) positions.
    pub fn bounding_sphere_center(&self) -> Vector3 {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in self.position.chunks_exact(3) {
            for axis in 0..3 {
                let v = f64::from(p[axis]);
                // `Box3.expandByPoint` uses `Vector3.min` and `max`, which
                // use `Math.min` and `Math.max`.
                min[axis] = crate::math::js_min(min[axis], v);
                max[axis] = crate::math::js_max(max[axis], v);
            }
        }
        // `Box3.getCenter`: `addVectors(min, max).multiplyScalar(0.5)`.
        Vector3::new(
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        )
    }

    /// `new BoxGeometry(width, height, depth, widthSegments,
    /// heightSegments, depthSegments)`.
    pub fn box_geometry(
        width: f64,
        height: f64,
        depth: f64,
        width_segments: u32,
        height_segments: u32,
        depth_segments: u32,
    ) -> BufferGeometry {
        let mut b = BoxBuilder::default();
        // px, nx, py, ny, pz, nz.
        b.plane(
            [2, 1, 0],
            -1.0,
            -1.0,
            [depth, height, width],
            [depth_segments, height_segments],
        );
        b.plane(
            [2, 1, 0],
            1.0,
            -1.0,
            [depth, height, -width],
            [depth_segments, height_segments],
        );
        b.plane(
            [0, 2, 1],
            1.0,
            1.0,
            [width, depth, height],
            [width_segments, depth_segments],
        );
        b.plane(
            [0, 2, 1],
            1.0,
            -1.0,
            [width, depth, -height],
            [width_segments, depth_segments],
        );
        b.plane(
            [0, 1, 2],
            1.0,
            -1.0,
            [width, height, depth],
            [width_segments, height_segments],
        );
        b.plane(
            [0, 1, 2],
            -1.0,
            -1.0,
            [width, height, -depth],
            [width_segments, height_segments],
        );
        BufferGeometry {
            index: Index::from_indices(&b.indices),
            position: to_f32(&b.vertices),
            normal: to_f32(&b.normals),
            uv: to_f32(&b.uvs),
        }
    }

    /// `new SphereGeometry(radius, widthSegments, heightSegments)`, with
    /// the default full `phi` and `theta` ranges.
    pub fn sphere_geometry(
        radius: f64,
        width_segments: u32,
        height_segments: u32,
    ) -> BufferGeometry {
        let phi_start = 0.0;
        let phi_length = std::f64::consts::PI * 2.0;
        let theta_start = 0.0;
        let theta_length = std::f64::consts::PI;
        let width_segments = width_segments.max(3);
        let height_segments = height_segments.max(2);
        let theta_end = crate::math::js_min(theta_start + theta_length, std::f64::consts::PI);
        let mut index = 0u32;
        let mut grid: Vec<Vec<u32>> = Vec::new();
        let (mut indices, mut vertices, mut normals, mut uvs) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for iy in 0..=height_segments {
            let mut row = Vec::new();
            let v = f64::from(iy) / f64::from(height_segments);
            let theta = theta_start + v * theta_length;
            let y = radius * jsmath::cos(theta);
            let ring_radius = (radius * radius - y * y).sqrt();
            // The poles' special case.
            let mut u_offset = 0.0;
            if iy == 0 && theta_start == 0.0 {
                u_offset = 0.5 / f64::from(width_segments);
            } else if iy == height_segments && theta_end == std::f64::consts::PI {
                u_offset = -0.5 / f64::from(width_segments);
            }
            for ix in 0..=width_segments {
                let u = f64::from(ix) / f64::from(width_segments);
                let phi = phi_start + u * phi_length;
                let vertex = Vector3::new(
                    -ring_radius * jsmath::cos(phi),
                    y,
                    ring_radius * jsmath::sin(phi),
                );
                vertices.extend([vertex.x, vertex.y, vertex.z]);
                let mut normal = vertex;
                normal.normalize();
                normals.extend([normal.x, normal.y, normal.z]);
                uvs.extend([u + u_offset, 1.0 - v]);
                row.push(index);
                index += 1;
            }
            grid.push(row);
        }
        for iy in 0..height_segments as usize {
            for ix in 0..width_segments as usize {
                let a = grid[iy][ix + 1];
                let b = grid[iy][ix];
                let c = grid[iy + 1][ix];
                let d = grid[iy + 1][ix + 1];
                if iy != 0 || theta_start > 0.0 {
                    indices.extend([a, b, d]);
                }
                if iy != height_segments as usize - 1 || theta_end < std::f64::consts::PI {
                    indices.extend([b, c, d]);
                }
            }
        }
        BufferGeometry {
            index: Index::from_indices(&indices),
            position: to_f32(&vertices),
            normal: to_f32(&normals),
            uv: to_f32(&uvs),
        }
    }
}

#[derive(Default)]
struct BoxBuilder {
    indices: Vec<u32>,
    vertices: Vec<f64>,
    normals: Vec<f64>,
    uvs: Vec<f64>,
    number_of_vertices: u32,
}

impl BoxBuilder {
    /// `buildPlane(u, v, w, udir, vdir, width, height, depth, gridX,
    /// gridY)`: `axes` are the component indices of `u`, `v` and `w`.
    fn plane(
        &mut self,
        axes: [usize; 3],
        udir: f64,
        vdir: f64,
        [width, height, depth]: [f64; 3],
        [grid_x, grid_y]: [u32; 2],
    ) {
        let [u, v, w] = axes;
        let segment_width = width / f64::from(grid_x);
        let segment_height = height / f64::from(grid_y);
        let width_half = width / 2.0;
        let height_half = height / 2.0;
        let depth_half = depth / 2.0;
        let grid_x1 = grid_x + 1;
        let grid_y1 = grid_y + 1;
        let mut vertex_counter = 0;
        for iy in 0..grid_y1 {
            let y = f64::from(iy) * segment_height - height_half;
            for ix in 0..grid_x1 {
                let x = f64::from(ix) * segment_width - width_half;
                // `vector` is reused across iterations: each write sets
                // all three components.
                let mut vector = [0.0; 3];
                vector[u] = x * udir;
                vector[v] = y * vdir;
                vector[w] = depth_half;
                self.vertices.extend(vector);
                vector[u] = 0.0;
                vector[v] = 0.0;
                vector[w] = if depth > 0.0 { 1.0 } else { -1.0 };
                self.normals.extend(vector);
                self.uvs.push(f64::from(ix) / f64::from(grid_x));
                self.uvs.push(1.0 - (f64::from(iy) / f64::from(grid_y)));
                vertex_counter += 1;
            }
        }
        let n = self.number_of_vertices;
        for iy in 0..grid_y {
            for ix in 0..grid_x {
                let a = n + ix + grid_x1 * iy;
                let b = n + ix + grid_x1 * (iy + 1);
                let c = n + (ix + 1) + grid_x1 * (iy + 1);
                let d = n + (ix + 1) + grid_x1 * iy;
                self.indices.extend([a, b, d]);
                self.indices.extend([b, c, d]);
            }
        }
        self.number_of_vertices += vertex_counter;
    }
}
