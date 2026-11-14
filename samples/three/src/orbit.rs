//! three.js's `OrbitControls` (`examples/jsm/controls/OrbitControls.js`)
//! for a `PerspectiveCamera`: rotating, dollying and panning with the
//! mouse, the wheel and the arrow keys, with damping, auto-rotation and the
//! distance, angle and target limits.
//!
//! The controls take their input as the events three.js reads: a pointer
//! event's `button`, `clientX`/`clientY` and modifier keys, a wheel event's
//! `deltaY` and `deltaMode`, a key event's `code`. They read the element's
//! `clientWidth` and `clientHeight` from `Element`. Not ported, and why:
//!
//! - touch input (two-pointer dolly and pan): Div fires `pointer*` events
//!   only for touch, and its gesture input is W14's;
//! - `zoomToCursor`, which unprojects the pointer through the camera and
//!   intersects a plane;
//! - orthographic cameras;
//! - the `cursorStyle` and `touchAction` writes to the element's style.

use crate::camera::PerspectiveCamera;
use crate::math::{clamp, js_max, js_min, Quaternion, Spherical, Vector2, Vector3};
use std::f64::consts::PI;

const TWO_PI: f64 = 2.0 * PI;
const EPS: f64 = 0.000001;

/// `MOUSE`: what a mouse button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    Rotate,
    Dolly,
    Pan,
}

/// `_STATE`, the gesture in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    None,
    Rotate,
    Dolly,
    Pan,
}

impl State {
    /// The number three.js uses.
    pub fn code(self) -> i32 {
        match self {
            State::None => -1,
            State::Rotate => 0,
            State::Dolly => 1,
            State::Pan => 2,
        }
    }
}

/// The DOM element the controls listen on, as they read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    pub client_width: f64,
    pub client_height: f64,
}

/// The modifier keys of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

impl Modifiers {
    fn any(self) -> bool {
        self.ctrl || self.meta || self.shift
    }
}

/// A mouse `PointerEvent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    pub pointer_id: i64,
    pub button: i32,
    pub client_x: f64,
    pub client_y: f64,
    pub modifiers: Modifiers,
}

/// A `WheelEvent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelEvent {
    pub client_x: f64,
    pub client_y: f64,
    pub delta_y: f64,
    /// 0 pixels, 1 lines, 2 pages.
    pub delta_mode: u32,
    pub ctrl: bool,
}

/// The events the controls dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsEvent {
    Change,
    Start,
    End,
}

impl ControlsEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            ControlsEvent::Change => "change",
            ControlsEvent::Start => "start",
            ControlsEvent::End => "end",
        }
    }
}

pub struct OrbitControls {
    pub camera: PerspectiveCamera,
    pub element: Element,
    pub enabled: bool,
    pub state: State,
    pub target: Vector3,
    pub cursor: Vector3,
    pub min_distance: f64,
    pub max_distance: f64,
    pub min_target_radius: f64,
    pub max_target_radius: f64,
    pub min_polar_angle: f64,
    pub max_polar_angle: f64,
    pub min_azimuth_angle: f64,
    pub max_azimuth_angle: f64,
    pub enable_damping: bool,
    pub damping_factor: f64,
    pub enable_zoom: bool,
    pub zoom_speed: f64,
    pub enable_rotate: bool,
    pub rotate_speed: f64,
    pub key_rotate_speed: f64,
    pub enable_pan: bool,
    pub pan_speed: f64,
    pub screen_space_panning: bool,
    pub key_pan_speed: f64,
    pub auto_rotate: bool,
    pub auto_rotate_speed: f64,
    /// `mouseButtons`: LEFT, MIDDLE, RIGHT.
    pub mouse_buttons: [Option<MouseAction>; 3],
    pub target0: Vector3,
    pub position0: Vector3,
    pub zoom0: f64,
    /// Every event dispatched, oldest first.
    pub events: Vec<ControlsEvent>,
    control_active: bool,
    last_position: Vector3,
    last_quaternion: Quaternion,
    last_target_position: Vector3,
    quat: Quaternion,
    quat_inverse: Quaternion,
    spherical: Spherical,
    spherical_delta: Spherical,
    scale: f64,
    pan_offset: Vector3,
    rotate_start: Vector2,
    rotate_end: Vector2,
    rotate_delta: Vector2,
    pan_start: Vector2,
    pan_end: Vector2,
    pan_delta: Vector2,
    dolly_start: Vector2,
    dolly_end: Vector2,
    dolly_delta: Vector2,
    pointers: Vec<i64>,
    /// Whether the document-level `pointermove` and `pointerup` listeners
    /// are attached: between the first pointer down and the last up.
    tracking: bool,
}

impl OrbitControls {
    /// `new OrbitControls(camera, element)`.
    pub fn new(camera: PerspectiveCamera, element: Element) -> OrbitControls {
        let mut quat = Quaternion::default();
        quat.set_from_unit_vectors(camera.up, Vector3::new(0.0, 1.0, 0.0));
        let mut quat_inverse = quat;
        quat_inverse.invert();
        let position0 = camera.position;
        let zoom0 = camera.zoom;
        let mut controls = OrbitControls {
            camera,
            element,
            enabled: true,
            state: State::None,
            target: Vector3::default(),
            cursor: Vector3::default(),
            min_distance: 0.0,
            max_distance: f64::INFINITY,
            min_target_radius: 0.0,
            max_target_radius: f64::INFINITY,
            min_polar_angle: 0.0,
            max_polar_angle: PI,
            min_azimuth_angle: f64::NEG_INFINITY,
            max_azimuth_angle: f64::INFINITY,
            enable_damping: false,
            damping_factor: 0.05,
            enable_zoom: true,
            zoom_speed: 1.0,
            enable_rotate: true,
            rotate_speed: 1.0,
            key_rotate_speed: 1.0,
            enable_pan: true,
            pan_speed: 1.0,
            screen_space_panning: true,
            key_pan_speed: 7.0,
            auto_rotate: false,
            auto_rotate_speed: 2.0,
            mouse_buttons: [
                Some(MouseAction::Rotate),
                Some(MouseAction::Dolly),
                Some(MouseAction::Pan),
            ],
            target0: Vector3::default(),
            position0,
            zoom0,
            events: Vec::new(),
            control_active: false,
            last_position: Vector3::default(),
            last_quaternion: Quaternion::default(),
            last_target_position: Vector3::default(),
            quat,
            quat_inverse,
            spherical: Spherical::default(),
            spherical_delta: Spherical::default(),
            scale: 1.0,
            pan_offset: Vector3::default(),
            rotate_start: Vector2::default(),
            rotate_end: Vector2::default(),
            rotate_delta: Vector2::default(),
            pan_start: Vector2::default(),
            pan_end: Vector2::default(),
            pan_delta: Vector2::default(),
            dolly_start: Vector2::default(),
            dolly_end: Vector2::default(),
            dolly_delta: Vector2::default(),
            pointers: Vec::new(),
            tracking: false,
        };
        controls.update(None);
        // The constructor's update dispatches 'change' before anyone can
        // listen.
        controls.events.clear();
        controls
    }

    fn dispatch(&mut self, event: ControlsEvent) {
        self.events.push(event);
    }

    /// `getPolarAngle()`.
    pub fn polar_angle(&self) -> f64 {
        self.spherical.phi
    }

    /// `getAzimuthalAngle()`.
    pub fn azimuthal_angle(&self) -> f64 {
        self.spherical.theta
    }

    /// `getDistance()`.
    pub fn distance(&self) -> f64 {
        self.camera.position.distance_to(self.target)
    }

    /// `saveState()`.
    pub fn save_state(&mut self) {
        self.target0 = self.target;
        self.position0 = self.camera.position;
        self.zoom0 = self.camera.zoom;
    }

    /// `reset()`.
    pub fn reset(&mut self) {
        self.target = self.target0;
        self.camera.position = self.position0;
        self.camera.zoom = self.zoom0;
        self.camera.update_projection_matrix();
        self.dispatch(ControlsEvent::Change);
        self.update(None);
        self.state = State::None;
    }

    /// `pan(deltaX, deltaY)`.
    pub fn pan(&mut self, delta_x: f64, delta_y: f64) {
        self.pan_by(delta_x, delta_y);
        self.update(None);
    }

    /// `dollyIn(dollyScale)`.
    pub fn dolly_in(&mut self, dolly_scale: f64) {
        self.scale *= dolly_scale;
        self.update(None);
    }

    /// `dollyOut(dollyScale)`.
    pub fn dolly_out(&mut self, dolly_scale: f64) {
        self.scale /= dolly_scale;
        self.update(None);
    }

    /// `rotateLeft(angle)`.
    pub fn rotate_left(&mut self, angle: f64) {
        self.spherical_delta.theta -= angle;
        self.update(None);
    }

    /// `rotateUp(angle)`.
    pub fn rotate_up(&mut self, angle: f64) {
        self.spherical_delta.phi -= angle;
        self.update(None);
    }

    /// `update(deltaTime)`. Returns whether the camera or target moved, in
    /// which case a `change` event was dispatched.
    pub fn update(&mut self, delta_time: Option<f64>) -> bool {
        let mut v = self.camera.position;
        v.sub(self.target);
        v.apply_quaternion(&self.quat);
        self.spherical.set_from_vector3(v);
        if self.auto_rotate && self.state == State::None {
            let angle = self.auto_rotation_angle(delta_time);
            self.spherical_delta.theta -= angle;
        }
        if self.enable_damping {
            self.spherical.theta += self.spherical_delta.theta * self.damping_factor;
            self.spherical.phi += self.spherical_delta.phi * self.damping_factor;
        } else {
            self.spherical.theta += self.spherical_delta.theta;
            self.spherical.phi += self.spherical_delta.phi;
        }
        let mut min = self.min_azimuth_angle;
        let mut max = self.max_azimuth_angle;
        if min.is_finite() && max.is_finite() {
            if min < -PI {
                min += TWO_PI;
            } else if min > PI {
                min -= TWO_PI;
            }
            if max < -PI {
                max += TWO_PI;
            } else if max > PI {
                max -= TWO_PI;
            }
            if min <= max {
                self.spherical.theta = js_max(min, js_min(max, self.spherical.theta));
            } else {
                self.spherical.theta = if self.spherical.theta > (min + max) / 2.0 {
                    js_max(min, self.spherical.theta)
                } else {
                    js_min(max, self.spherical.theta)
                };
            }
        }
        self.spherical.phi = js_max(
            self.min_polar_angle,
            js_min(self.max_polar_angle, self.spherical.phi),
        );
        self.spherical.make_safe();
        if self.enable_damping {
            self.target
                .add_scaled_vector(self.pan_offset, self.damping_factor);
        } else {
            self.target.add(self.pan_offset);
        }
        self.target.sub(self.cursor);
        self.target
            .clamp_length(self.min_target_radius, self.max_target_radius);
        self.target.add(self.cursor);
        let prev_radius = self.spherical.radius;
        self.spherical.radius = self.clamp_distance(self.spherical.radius * self.scale);
        let zoom_changed = prev_radius != self.spherical.radius;
        let mut v = Vector3::default();
        v.set_from_spherical(&self.spherical);
        v.apply_quaternion(&self.quat_inverse);
        let mut position = self.target;
        position.add(v);
        self.camera.position = position;
        self.camera.look_at(self.target);
        if self.enable_damping {
            self.spherical_delta.theta *= 1.0 - self.damping_factor;
            self.spherical_delta.phi *= 1.0 - self.damping_factor;
            self.pan_offset.multiply_scalar(1.0 - self.damping_factor);
        } else {
            self.spherical_delta.set(0.0, 0.0, 0.0);
            self.pan_offset.set(0.0, 0.0, 0.0);
        }
        self.scale = 1.0;
        let quaternion = self.camera.quaternion();
        if zoom_changed
            || self.last_position.distance_to_squared(self.camera.position) > EPS
            || 8.0 * (1.0 - self.last_quaternion.dot(&quaternion)) > EPS
            || self.last_target_position.distance_to_squared(self.target) > EPS
        {
            self.dispatch(ControlsEvent::Change);
            self.last_position = self.camera.position;
            self.last_quaternion = quaternion;
            self.last_target_position = self.target;
            return true;
        }
        false
    }

    fn auto_rotation_angle(&self, delta_time: Option<f64>) -> f64 {
        match delta_time {
            Some(dt) => (TWO_PI / 60.0 * self.auto_rotate_speed) * dt,
            None => TWO_PI / 60.0 / 60.0 * self.auto_rotate_speed,
        }
    }

    fn zoom_scale(&self, delta: f64) -> f64 {
        let normalized_delta = (delta * 0.01).abs();
        crate::jsmath::pow(0.95, self.zoom_speed * normalized_delta)
    }

    fn pan_left(&mut self, distance: f64) {
        let mut v = Vector3::default();
        v.set_from_matrix_column(&self.camera.matrix, 0);
        v.multiply_scalar(-distance);
        self.pan_offset.add(v);
    }

    fn pan_up(&mut self, distance: f64) {
        let mut v = Vector3::default();
        if self.screen_space_panning {
            v.set_from_matrix_column(&self.camera.matrix, 1);
        } else {
            v.set_from_matrix_column(&self.camera.matrix, 0);
            let column = v;
            v.cross_vectors(self.camera.up, column);
        }
        v.multiply_scalar(distance);
        self.pan_offset.add(v);
    }

    /// `_pan(deltaX, deltaY)`, for a perspective camera.
    fn pan_by(&mut self, delta_x: f64, delta_y: f64) {
        let mut v = self.camera.position;
        v.sub(self.target);
        let mut target_distance = v.length();
        target_distance *= libm::tan((self.camera.fov / 2.0) * PI / 180.0);
        let height = self.element.client_height;
        self.pan_left(2.0 * delta_x * target_distance / height);
        self.pan_up(2.0 * delta_y * target_distance / height);
    }

    fn clamp_distance(&self, dist: f64) -> f64 {
        js_max(self.min_distance, js_min(self.max_distance, dist))
    }

    fn mouse_move_rotate(&mut self, x: f64, y: f64) {
        self.rotate_end.set(x, y);
        self.rotate_delta
            .sub_vectors(self.rotate_end, self.rotate_start);
        self.rotate_delta.multiply_scalar(self.rotate_speed);
        let height = self.element.client_height;
        self.spherical_delta.theta -= TWO_PI * self.rotate_delta.x / height;
        self.spherical_delta.phi -= TWO_PI * self.rotate_delta.y / height;
        self.rotate_start = self.rotate_end;
        self.update(None);
    }

    fn mouse_move_dolly(&mut self, x: f64, y: f64) {
        self.dolly_end.set(x, y);
        self.dolly_delta
            .sub_vectors(self.dolly_end, self.dolly_start);
        if self.dolly_delta.y > 0.0 {
            self.scale /= self.zoom_scale(self.dolly_delta.y);
        } else if self.dolly_delta.y < 0.0 {
            self.scale *= self.zoom_scale(self.dolly_delta.y);
        }
        self.dolly_start = self.dolly_end;
        self.update(None);
    }

    fn mouse_move_pan(&mut self, x: f64, y: f64) {
        self.pan_end.set(x, y);
        self.pan_delta.sub_vectors(self.pan_end, self.pan_start);
        self.pan_delta.multiply_scalar(self.pan_speed);
        self.pan_by(self.pan_delta.x, self.pan_delta.y);
        self.pan_start = self.pan_end;
        self.update(None);
    }

    /// The element's `pointerdown` listener, for a mouse.
    pub fn pointer_down(&mut self, event: &PointerEvent) {
        if !self.enabled {
            return;
        }
        if self.pointers.is_empty() {
            // setPointerCapture, and the document's pointermove and
            // pointerup listeners.
            self.tracking = true;
        }
        if self.pointers.contains(&event.pointer_id) {
            return;
        }
        self.pointers.push(event.pointer_id);
        self.mouse_down(event);
    }

    fn mouse_down(&mut self, event: &PointerEvent) {
        let action = match event.button {
            0 => self.mouse_buttons[0],
            1 => self.mouse_buttons[1],
            2 => self.mouse_buttons[2],
            _ => None,
        };
        let (x, y) = (event.client_x, event.client_y);
        match action {
            Some(MouseAction::Dolly) => {
                if !self.enable_zoom {
                    return;
                }
                self.dolly_start.set(x, y);
                self.state = State::Dolly;
            }
            Some(MouseAction::Rotate) => {
                if event.modifiers.any() {
                    if !self.enable_pan {
                        return;
                    }
                    self.pan_start.set(x, y);
                    self.state = State::Pan;
                } else {
                    if !self.enable_rotate {
                        return;
                    }
                    self.rotate_start.set(x, y);
                    self.state = State::Rotate;
                }
            }
            Some(MouseAction::Pan) => {
                if event.modifiers.any() {
                    if !self.enable_rotate {
                        return;
                    }
                    self.rotate_start.set(x, y);
                    self.state = State::Rotate;
                } else {
                    if !self.enable_pan {
                        return;
                    }
                    self.pan_start.set(x, y);
                    self.state = State::Pan;
                }
            }
            None => self.state = State::None,
        }
        if self.state != State::None {
            self.dispatch(ControlsEvent::Start);
        }
    }

    /// The document's `pointermove` listener, attached while a pointer is
    /// down.
    pub fn pointer_move(&mut self, event: &PointerEvent) {
        if !self.tracking || !self.enabled {
            return;
        }
        let (x, y) = (event.client_x, event.client_y);
        match self.state {
            State::Rotate => {
                if self.enable_rotate {
                    self.mouse_move_rotate(x, y);
                }
            }
            State::Dolly => {
                if self.enable_zoom {
                    self.mouse_move_dolly(x, y);
                }
            }
            State::Pan => {
                if self.enable_pan {
                    self.mouse_move_pan(x, y);
                }
            }
            State::None => {}
        }
    }

    /// The document's `pointerup` listener (and the element's
    /// `pointercancel`).
    pub fn pointer_up(&mut self, event: &PointerEvent) {
        if !self.tracking {
            return;
        }
        self.pointers.retain(|&id| id != event.pointer_id);
        if self.pointers.is_empty() {
            self.tracking = false;
            self.dispatch(ControlsEvent::End);
            self.state = State::None;
        }
    }

    /// The element's `wheel` listener.
    pub fn wheel(&mut self, event: &WheelEvent) {
        if !self.enabled || !self.enable_zoom || self.state != State::None {
            return;
        }
        self.dispatch(ControlsEvent::Start);
        let mut delta_y = event.delta_y;
        match event.delta_mode {
            1 => delta_y *= 16.0,
            2 => delta_y *= 100.0,
            _ => {}
        }
        if event.ctrl && !self.control_active {
            delta_y *= 10.0;
        }
        if delta_y < 0.0 {
            self.scale *= self.zoom_scale(delta_y);
        } else if delta_y > 0.0 {
            self.scale /= self.zoom_scale(delta_y);
        }
        self.update(None);
        self.dispatch(ControlsEvent::End);
    }

    /// The document's capturing `keydown` listener, which notes the Control
    /// key, and `keyup`.
    pub fn control_key(&mut self, down: bool) {
        self.control_active = down;
    }

    /// The `keydown` listener `listenToKeyEvents` attaches. `code` is the
    /// event's `code`.
    pub fn key_down(&mut self, code: &str, modifiers: Modifiers) {
        if !self.enabled {
            return;
        }
        let height = self.element.client_height;
        let step = TWO_PI * self.key_rotate_speed / height;
        let rotate = modifiers.any();
        match code {
            "ArrowUp" => {
                if rotate {
                    if self.enable_rotate {
                        self.spherical_delta.phi -= step;
                    }
                } else if self.enable_pan {
                    self.pan_by(0.0, self.key_pan_speed);
                }
            }
            "ArrowDown" => {
                if rotate {
                    if self.enable_rotate {
                        self.spherical_delta.phi -= -step;
                    }
                } else if self.enable_pan {
                    self.pan_by(0.0, -self.key_pan_speed);
                }
            }
            "ArrowLeft" => {
                if rotate {
                    if self.enable_rotate {
                        self.spherical_delta.theta -= step;
                    }
                } else if self.enable_pan {
                    self.pan_by(self.key_pan_speed, 0.0);
                }
            }
            "ArrowRight" => {
                if rotate {
                    if self.enable_rotate {
                        self.spherical_delta.theta -= -step;
                    }
                } else if self.enable_pan {
                    self.pan_by(-self.key_pan_speed, 0.0);
                }
            }
            _ => return,
        }
        self.update(None);
    }

    /// Clamps as `MathUtils.clamp`; exposed for the tests.
    pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
        clamp(value, min, max)
    }
}
