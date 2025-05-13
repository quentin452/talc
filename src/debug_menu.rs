//! FPS counter for Bevy game engine

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

    use std::time::Duration;

    use crate::{chunky::{async_chunkloader::Chunks, chunk::Chunk}, render::chunk_material::RenderableChunk};

pub const FONT_SIZE: f32 = 32.;
pub const FONT_COLOR: Color = Color::WHITE;
pub const STRING_FORMAT: &str = "FPS: ";
pub const STRING_INITIAL: &str = "FPS: ...";
pub const STRING_MISSING: &str = "FPS: ???";
pub const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

/// FPS counter plugin
pub struct FpsCounterPlugin;

impl Plugin for FpsCounterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, spawn_text)
            .add_systems(Update, update)
            .init_resource::<FpsCounter>();
    }
}

#[derive(Resource)]
pub struct FpsCounter {
    pub timer: Timer,
    pub update_now: bool,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            timer: Timer::new(UPDATE_INTERVAL, TimerMode::Repeating),
            update_now: true,
        }
    }
}

impl FpsCounter {
    /// Enable FPS counter
    pub fn enable(&mut self) {
        self.timer.unpause();
        self.update_now = true;
    }

    /// Disable FPS counter
    pub fn disable(&mut self) {
        self.timer.pause();
        self.update_now = true;
    }

    /// Check if FPS counter is enabled
    pub fn is_enabled(&self) -> bool {
        !self.timer.paused()
    }
}

/// The marker on the text to be updated
#[derive(Component)]
pub struct FpsCounterText;

fn update(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    state_resources: Option<ResMut<FpsCounter>>,
    mut query: Query<Entity, With<FpsCounterText>>,
    mut writer: TextUiWriter,
    chunk_entities: Res<Chunks>,
    renderable_chunks: Query<(&Chunk, &RenderableChunk)>
) {
    let Some(mut state) = state_resources else {
        return;
    };
    if !(state.update_now || state.timer.tick(time.delta()).just_finished()) {
        return;
    }
    if state.timer.paused() {
        for entity in query.iter_mut() {
            writer.text(entity, 0).clear();
        }
    } else {
        let fps_dialog = extract_fps(&diagnostics);

        for entity in query.iter_mut() {
            if let Some((fps, frame_time)) = fps_dialog {
                *writer.text(entity, 0) = format!("{}{:.0}\n{:.1} ms\nloaded chunks: {}\nmeshed chunks: {}", STRING_FORMAT, fps, frame_time, chunk_entities.0.len(), renderable_chunks.iter().len());
            } else {
                *writer.text(entity, 0) = STRING_MISSING.to_string();
            }
        }
    }
}

fn extract_fps(diagnostics: &Res<DiagnosticsStore>) -> Option<(f64, f64)> {
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.average()) {
        if let Some(frame_time) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .and_then(|frame_time| frame_time.average()) {
            return Some((fps, frame_time));
        }
    }

    None    
}

fn spawn_text(mut commands: Commands) {
    commands
        .spawn((
            Text::new(STRING_INITIAL),
            TextFont {
                font_size: FONT_SIZE,
                ..Default::default()
            },
            TextColor(FONT_COLOR),
        ))
        .insert(FpsCounterText);
}