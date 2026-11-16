//! A `Scene` of `Mesh`es (`src/scenes/Scene.js`, `src/objects/Mesh.js`,
//! `src/core/Object3D.js`) with `MeshBasicMaterial`
//! (`src/materials/MeshBasicMaterial.js`), as far as the renderer in
//! `three-render` draws them: meshes directly under the scene, each with a
//! color and an opacity.

use std::rc::Rc;

use crate::color::Color;
use crate::geometry::BufferGeometry;
use crate::math::{Euler, EulerOrder, Matrix4, Quaternion, Vector3};

/// `MeshBasicMaterial` with a color and no map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBasicMaterial {
    pub color: Color,
    pub opacity: f64,
}

impl MeshBasicMaterial {
    /// `new MeshBasicMaterial({ color: hex })`.
    pub fn new(hex: u32) -> MeshBasicMaterial {
        MeshBasicMaterial {
            color: Color::from_hex(hex),
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    /// `Object3D.id`, which counts objects in creation order.
    pub id: u32,
    pub geometry: Rc<BufferGeometry>,
    pub material: MeshBasicMaterial,
    pub position: Vector3,
    pub rotation: Euler,
    pub scale: Vector3,
    pub matrix: Matrix4,
    pub matrix_world: Matrix4,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub id: u32,
    /// `scene.background` as a color.
    pub background: Option<Color>,
    pub children: Vec<Mesh>,
    next_id: u32,
}

impl Scene {
    /// `new Scene()`: `Object3D.id` starts at 0 for the first object.
    pub fn new() -> Scene {
        Scene {
            id: 0,
            background: None,
            children: Vec::new(),
            next_id: 1,
        }
    }

    /// `scene.add(new Mesh(geometry, material))`, with the mesh's
    /// `position`, `rotation` (XYZ order) and unit scale.
    pub fn add_mesh(
        &mut self,
        geometry: Rc<BufferGeometry>,
        material: MeshBasicMaterial,
        position: Vector3,
        rotation: [f64; 3],
    ) -> &mut Mesh {
        let id = self.next_id;
        self.next_id += 1;
        self.children.push(Mesh {
            id,
            geometry,
            material,
            position,
            rotation: Euler {
                x: rotation[0],
                y: rotation[1],
                z: rotation[2],
                order: EulerOrder::Xyz,
            },
            scale: Vector3::new(1.0, 1.0, 1.0),
            matrix: Matrix4::default(),
            matrix_world: Matrix4::default(),
        });
        self.children.last_mut().unwrap()
    }

    /// `scene.updateMatrixWorld()`: the scene's own matrix composes to the
    /// identity, and each child's world matrix is the scene's times its
    /// own.
    pub fn update_matrix_world(&mut self) {
        let mut scene_world = Matrix4::default();
        let mut q = Quaternion::default();
        q.identity();
        scene_world.compose(Vector3::default(), &q, Vector3::new(1.0, 1.0, 1.0));
        for mesh in &mut self.children {
            // `rotation`'s onChange keeps `quaternion` in step.
            let mut q = Quaternion::default();
            q.set_from_euler(&mesh.rotation);
            mesh.matrix.compose(mesh.position, &q, mesh.scale);
            mesh.matrix_world
                .multiply_matrices(&scene_world, &mesh.matrix);
        }
    }
}
