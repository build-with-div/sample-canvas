//! three.js's `PerspectiveCamera` (`src/cameras/PerspectiveCamera.js`), with
//! the parts of `Object3D` and `Camera` it uses: position, rotation and
//! quaternion kept in step, `lookAt`, and the world and projection
//! matrices. The camera has no parent and no children, as an
//! `OrbitControls` camera usually has none.

use crate::math::{
    CoordinateSystem, Euler, EulerOrder, Matrix4, Quaternion, Vector3, DEG2RAD, RAD2DEG,
};

/// `PerspectiveCamera.view`: the window into a larger view that
/// `setViewOffset` sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub enabled: bool,
    pub full_width: f64,
    pub full_height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub position: Vector3,
    /// Kept in step with `quaternion`, as `Object3D` keeps them: changing
    /// one through the camera's methods updates the other.
    rotation: Euler,
    quaternion: Quaternion,
    pub scale: Vector3,
    pub up: Vector3,
    pub matrix: Matrix4,
    pub matrix_world: Matrix4,
    pub matrix_world_needs_update: bool,
    pub matrix_world_inverse: Matrix4,
    pub projection_matrix: Matrix4,
    pub projection_matrix_inverse: Matrix4,
    pub coordinate_system: CoordinateSystem,
    pub fov: f64,
    pub zoom: f64,
    pub near: f64,
    pub far: f64,
    pub focus: f64,
    pub aspect: f64,
    pub view: Option<View>,
    pub film_gauge: f64,
    pub film_offset: f64,
}

impl PerspectiveCamera {
    /// `new PerspectiveCamera(fov, aspect, near, far)`; three.js's defaults
    /// are 50, 1, 0.1 and 2000.
    pub fn new(fov: f64, aspect: f64, near: f64, far: f64) -> PerspectiveCamera {
        let mut camera = PerspectiveCamera {
            position: Vector3::default(),
            rotation: Euler::default(),
            quaternion: Quaternion::default(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            matrix: Matrix4::default(),
            matrix_world: Matrix4::default(),
            matrix_world_needs_update: false,
            matrix_world_inverse: Matrix4::default(),
            projection_matrix: Matrix4::default(),
            projection_matrix_inverse: Matrix4::default(),
            coordinate_system: CoordinateSystem::WebGl,
            fov,
            zoom: 1.0,
            near,
            far,
            focus: 10.0,
            aspect,
            view: None,
            film_gauge: 35.0,
            film_offset: 0.0,
        };
        camera.update_projection_matrix();
        camera
    }

    pub fn rotation(&self) -> Euler {
        self.rotation
    }

    pub fn quaternion(&self) -> Quaternion {
        self.quaternion
    }

    /// Sets the quaternion, and the rotation from it
    /// (`onQuaternionChange`).
    pub fn set_quaternion(&mut self, q: Quaternion) {
        self.quaternion = q;
        let order = self.rotation.order;
        self.rotation.set_from_quaternion(&self.quaternion, order);
    }

    /// Sets the rotation, and the quaternion from it (`onRotationChange`).
    pub fn set_rotation(&mut self, x: f64, y: f64, z: f64, order: EulerOrder) {
        self.rotation = Euler { x, y, z, order };
        self.quaternion.set_from_euler(&self.rotation);
    }

    /// `getFilmWidth()`.
    pub fn film_width(&self) -> f64 {
        self.film_gauge * crate::math::js_min(self.aspect, 1.0)
    }

    /// `getFilmHeight()`.
    pub fn film_height(&self) -> f64 {
        self.film_gauge / crate::math::js_max(self.aspect, 1.0)
    }

    /// `setFocalLength(focalLength)`.
    pub fn set_focal_length(&mut self, focal_length: f64) {
        let v_extent_slope = 0.5 * self.film_height() / focal_length;
        self.fov = RAD2DEG * 2.0 * libm::atan(v_extent_slope);
        self.update_projection_matrix();
    }

    /// `getEffectiveFOV()`.
    pub fn effective_fov(&self) -> f64 {
        RAD2DEG * 2.0 * libm::atan(libm::tan(DEG2RAD * 0.5 * self.fov) / self.zoom)
    }

    /// `setViewOffset(fullWidth, fullHeight, x, y, width, height)`.
    pub fn set_view_offset(
        &mut self,
        full_width: f64,
        full_height: f64,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.aspect = full_width / full_height;
        self.view = Some(View {
            enabled: true,
            full_width,
            full_height,
            offset_x: x,
            offset_y: y,
            width,
            height,
        });
        self.update_projection_matrix();
    }

    /// `clearViewOffset()`.
    pub fn clear_view_offset(&mut self) {
        if let Some(view) = &mut self.view {
            view.enabled = false;
        }
        self.update_projection_matrix();
    }

    /// `updateProjectionMatrix()`.
    pub fn update_projection_matrix(&mut self) {
        let near = self.near;
        let mut top = near * libm::tan(DEG2RAD * 0.5 * self.fov) / self.zoom;
        let mut height = 2.0 * top;
        let mut width = self.aspect * height;
        let mut left = -0.5 * width;
        if let Some(view) = self.view.filter(|v| v.enabled) {
            left += view.offset_x * width / view.full_width;
            top -= view.offset_y * height / view.full_height;
            width *= view.width / view.full_width;
            height *= view.height / view.full_height;
        }
        let skew = self.film_offset;
        if skew != 0.0 {
            left += near * skew / self.film_width();
        }
        self.projection_matrix.make_perspective(
            left,
            left + width,
            top,
            top - height,
            near,
            self.far,
            self.coordinate_system,
            false,
        );
        self.projection_matrix_inverse = self.projection_matrix;
        self.projection_matrix_inverse.invert();
    }

    /// `updateMatrix()`.
    pub fn update_matrix(&mut self) {
        self.matrix
            .compose(self.position, &self.quaternion, self.scale);
        self.matrix_world_needs_update = true;
    }

    /// `Camera.updateWorldMatrix` after `Object3D.updateWorldMatrix`, for a
    /// camera with no parent: the world matrix and its inverse.
    fn update_world(&mut self, force: bool) {
        self.update_matrix();
        if self.matrix_world_needs_update || force {
            self.matrix_world = self.matrix;
            self.matrix_world_needs_update = false;
        }
        let (position, quaternion, scale) = self.matrix_world.decompose();
        if scale.x == 1.0 && scale.y == 1.0 && scale.z == 1.0 {
            self.matrix_world_inverse = self.matrix_world;
            self.matrix_world_inverse.invert();
        } else {
            self.matrix_world_inverse
                .compose(position, &quaternion, Vector3::new(1.0, 1.0, 1.0))
                .invert();
        }
    }

    /// `updateMatrixWorld(force)`.
    pub fn update_matrix_world(&mut self, force: bool) {
        self.update_world(force);
    }

    /// `updateWorldMatrix(updateParents, updateChildren, force)`.
    pub fn update_world_matrix(&mut self, force: bool) {
        self.update_world(force);
    }

    /// `lookAt(target)`: a camera looks down its -z axis.
    pub fn look_at(&mut self, target: Vector3) {
        self.update_world_matrix(false);
        let mut position = Vector3::default();
        position.set_from_matrix_position(&self.matrix_world);
        // `_m1` is a module-level matrix in Object3D.js; `lookAt` writes
        // its rotation part and `setFromRotationMatrix` reads only that.
        let mut m1 = Matrix4::default();
        m1.look_at(position, target, self.up);
        let mut q = self.quaternion;
        q.set_from_rotation_matrix(&m1);
        self.set_quaternion(q);
    }
}
