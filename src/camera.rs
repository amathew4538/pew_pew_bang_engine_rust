//! # Camera
//!
//! Virtual camera for orthographic projection in the window.
//! Also has camera controlling!

use std::{f32::consts::FRAC_PI_2, time::Duration};

use glam::{Mat4, Vec3, camera::rh::{proj, view}};
use winit::{dpi::PhysicalPosition, event::{ElementState, MouseScrollDelta}, keyboard::KeyCode};

/// # Old Camera
///
/// Virtual camera in 3D space
// pub struct Camera {
//     eye: Vec3,
//     target: Vec3,
//     up: Vec3,
//     aspect: f32,
//     fovy: f32,
//     znear: f32,
//     zfar: f32,
// }

// impl Camera {
//     pub fn new(
//         eye: Vec3,
//         target: Vec3,
//         up: Vec3,
//         aspect: f32,
//         fovy: f32,
//         znear: f32,
//         zfar: f32,
//     ) -> Self {
//         Self { eye, target, up, aspect, fovy, znear, zfar }
//     }

//     pub fn build_view_projection_matrix(&self) -> Mat4 {
//         // Moves the world to be at the position and rotation of the camera
//         let view: Mat4 = view::look_at_mat4(self.eye, self.target, self.up);
//         // Warps the scene to give the effect of depth
//         let proj: Mat4 = proj::directx::perspective(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);

//         proj * view
//     }
// }

/// # Camera Uniform
///
/// Stores view data
// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    // We can't use glam with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).to_cols_array_2d();
    }
}

/// # Old Camera Controller
///
/// "Move" camera around and stuff
// pub struct CameraController {
//     speed: f32,
//     is_forward_pressed: bool,
//     is_backward_pressed: bool,
//     is_left_pressed: bool,
//     is_right_pressed: bool,
// }

// impl CameraController {
//     pub fn new(speed: f32) -> Self {
//         Self {
//             speed,
//             is_forward_pressed: false,
//             is_backward_pressed: false,
//             is_left_pressed: false,
//             is_right_pressed: false,
//         }
//     }

//     pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
//         match code {
//             KeyCode::KeyW | KeyCode::ArrowUp => {
//                 self.is_forward_pressed = is_pressed;
//                 true
//             }
//             KeyCode::KeyA | KeyCode::ArrowLeft => {
//                 self.is_left_pressed = is_pressed;
//                 true
//             }
//             KeyCode::KeyS | KeyCode::ArrowDown => {
//                 self.is_backward_pressed = is_pressed;
//                 true
//             }
//             KeyCode::KeyD | KeyCode::ArrowRight => {
//                 self.is_right_pressed = is_pressed;
//                 true
//             }
//             _ => false,
//         }
//     }

//     pub fn update_camera(&self, camera: &mut Camera) {
//         let forward: Vec3 = camera.target - camera.eye;
//         let forward_norm: Vec3 = forward.normalize();
//         let forward_mag: f32 = forward.length();

//         // Prevents glitching when the camera gets too close to the
//         // center of the scene.
//         if self.is_forward_pressed && forward_mag > self.speed {
//             camera.eye += forward_norm * self.speed;
//         }
//         if self.is_backward_pressed {
//             camera.eye -= forward_norm * self.speed;
//         }

//         let right: Vec3 = forward_norm.cross(camera.up);

//         // Redo radius calc in case the forward/backward is pressed.
//         let forward: Vec3 = camera.target - camera.eye;
//         let forward_mag: f32 = forward.length();

//         if self.is_right_pressed {
//             // Rescale the distance between the target and the eye so
//             // that it doesn't change. The eye, therefore, still
//             // lies on the circle made by the target and eye.
//             camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
//         }
//         if self.is_left_pressed {
//             camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
//         }
//     }
// }

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new<
        V: Into<Vec3>,
        Y: Into<f32>,
        P: Into<f32>,
    >(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        view::look_to_mat4(
            self.position,
            Vec3::new(
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw
            ).normalize(),
            Vec3::Y,
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<f32>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        proj::directx::perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool{
        let amount: f32 = if state == ElementState::Pressed { 1.0 } else { 0.0 };

        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition {
                y: scroll,
                ..
            }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt: f32 = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward: Vec3 = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right: Vec3 = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward: Vec3 = Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        camera.yaw += self.rotate_horizontal * self.sensitivity * dt;
        camera.pitch += -self.rotate_vertical * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non-cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -FRAC_PI_2 {
            camera.pitch = -FRAC_PI_2;
        } else if camera.pitch > FRAC_PI_2 {
            camera.pitch = FRAC_PI_2;
        }
    }
}
