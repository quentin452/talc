use bevy::prelude::*;

// helper
#[derive(Copy, Clone)]
pub enum Direction {
    Left,
    Right,
    Down,
    Up,
    Back,
    Forward,
}

impl Direction {
    /// normal data is packed in the shader
    #[must_use]
    pub const fn get_normal(&self) -> i32 {
        match self {
            Self::Left => 0i32,
            Self::Right => 1i32,
            Self::Down => 2i32,
            Self::Up => 3i32,
            Self::Back => 4i32,
            Self::Forward => 5i32,
        }
    }

    #[must_use]
    pub const fn get_opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::Back => Self::Forward,
            Self::Forward => Self::Back,
        }
    }
}

/// plane data with 4 vertices
pub struct Quad {
    pub color: Color,
    pub direction: Direction,
    pub corners: [[i32; 3]; 4],
}

impl Quad {
    // the input position is assumed to be a voxel's (0,0,0) pos
    // therefore right / up / forward are offset by 1
    #[inline]
    #[must_use]
    pub const fn from_direction(direction: Direction, pos: IVec3, color: Color) -> Self {
        let corners = match direction {
            Direction::Left => [
                [pos.x, pos.y, pos.z],
                [pos.x, pos.y, pos.z + 1],
                [pos.x, pos.y + 1, pos.z + 1],
                [pos.x, pos.y + 1, pos.z],
            ],
            Direction::Right => [
                [pos.x, pos.y + 1, pos.z],
                [pos.x, pos.y + 1, pos.z + 1],
                [pos.x, pos.y, pos.z + 1],
                [pos.x, pos.y, pos.z],
            ],
            Direction::Down => [
                [pos.x, pos.y, pos.z],
                [pos.x + 1, pos.y, pos.z],
                [pos.x + 1, pos.y, pos.z + 1],
                [pos.x, pos.y, pos.z + 1],
            ],
            Direction::Up => [
                [pos.x, pos.y, pos.z + 1],
                [pos.x + 1, pos.y, pos.z + 1],
                [pos.x + 1, pos.y, pos.z],
                [pos.x, pos.y, pos.z],
            ],
            Direction::Back => [
                [pos.x, pos.y, pos.z],
                [pos.x, pos.y + 1, pos.z],
                [pos.x + 1, pos.y + 1, pos.z],
                [pos.x + 1, pos.y, pos.z],
            ],
            Direction::Forward => [
                [pos.x + 1, pos.y, pos.z],
                [pos.x + 1, pos.y + 1, pos.z],
                [pos.x, pos.y + 1, pos.z],
                [pos.x, pos.y, pos.z],
            ],
        };

        Self {
            color,
            direction,
            corners,
        }
    }
}
