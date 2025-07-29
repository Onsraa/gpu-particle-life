use bevy::input::ButtonInput;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::math::{EulerRot, Quat, Vec2, Vec3};
use bevy::prelude::{Camera, MouseButton, Query, Res, Transform, With};
use crate::resources::world::camera::CameraSettings;

pub fn orbit(
    mut camera: Query<&mut Transform, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
) {
    let delta = mouse_motion.delta;

    if mouse_buttons.pressed(MouseButton::Left) && delta != Vec2::ZERO {
        for mut transform in camera.iter_mut() {
            let delta_pitch = delta.y * camera_settings.pitch_speed;
            let delta_yaw = delta.x * camera_settings.yaw_speed;

            let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

            let pitch = (pitch + delta_pitch).clamp(
                camera_settings.pitch_range.start,
                camera_settings.pitch_range.end,
            );
            let yaw = yaw + delta_yaw;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

            let target = Vec3::ZERO;

            let orbit_distance = camera_settings.orbit_distance;

            transform.translation = target - transform.forward() * orbit_distance;
        }
    }
}