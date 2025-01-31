use std::time::Duration;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        system::{Commands, Query, Res},
    },
    math::Vec3,
    time::Time,
    transform::components::Transform,
};

use crate::position::FloatingPosition;

#[derive(Component)]
#[require(Transform)]
pub struct SmoothTransformTo {
    direction: Vec3,
    blocks_per_second: f32,
    end_timestamp: Duration,
}

impl SmoothTransformTo {
    #[must_use]
    pub fn new(timer: &Res<Time>, end: FloatingPosition, blocks_per_second: f32) -> Self {
        Self {
            direction: end.0.normalize(),
            blocks_per_second,
            end_timestamp: timer.elapsed()
                + Duration::from_secs_f32(end.0.distance(Vec3::ZERO) / blocks_per_second),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn smooth_transform(
    mut commands: Commands,
    mut to_move: Query<(Entity, &mut Transform, &SmoothTransformTo)>,
    timer: Res<Time>,
) {
    for (entity, mut transform, smooth_transform) in &mut to_move {
        let delta_seconds = if timer.elapsed() < smooth_transform.end_timestamp {
            timer.delta_secs()
        } else {
            commands.entity(entity).remove::<SmoothTransformTo>();
            let time_of_previous_update = timer.elapsed() - timer.delta();
            if time_of_previous_update >= smooth_transform.end_timestamp {
                return;
            }
            (smooth_transform.end_timestamp - time_of_previous_update).as_secs_f32()
        };

        transform.translation +=
            smooth_transform.direction * delta_seconds * smooth_transform.blocks_per_second;
    }
}
