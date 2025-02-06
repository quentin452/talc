use crate::{bevy::prelude::*, position::FloatingPosition};

use super::{camera::Camera, debug_camera::FlyCam, render_distance::Scanner};

pub fn new(commands: &mut Commands) {
    commands.spawn((
        Scanner::new(12),
        Transform::from_xyz(0.0, 200.0, 0.5),
        FlyCam,
        Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: FloatingPosition::new(0.0, 1.0, 2.0),
            // have it look at the origin
            orientation: Vec3::new(0.0, 0.0, -1.0), // Facing -Z direction
            // which way is "up"
            up: Vec3::Y,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    ));
}