use std::ops::{Add, Div, Mul, Sub};

use bevy::math::{IVec3, Vec3};

use crate::chunk::CHUNK_SIZE_I32;

/// A grid aligned position in the world using absolute coordinates.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct Position(pub IVec3);

/// A grid aligned position in the world using relative coordinates.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct RelativePosition(pub IVec3);

/// A floating point position in the world.
#[derive(Debug, Clone, Copy)]
pub struct FloatingPosition(pub Vec3);

/// Represents the location of a chunk.
/// The x, y, z components are scaled down by a factor of `chunk::CHUNK_SIZE`
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct ChunkPosition(pub IVec3);

impl Position {
    #[must_use]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3 { x, y, z })
    }

    #[rustfmt::skip] #[inline] #[must_use]    pub const fn x(&self) -> i32 { self.0.x }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn y(&self) -> i32 { self.0.y }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn z(&self) -> i32 { self.0.z }
}

impl RelativePosition {
    #[must_use]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3 { x, y, z })
    }

    #[rustfmt::skip] #[inline] #[must_use]    pub const fn x(&self) -> i32 { self.0.x }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn y(&self) -> i32 { self.0.y }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn z(&self) -> i32 { self.0.z }
}

impl FloatingPosition {
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3 { x, y, z })
    }

    #[rustfmt::skip] #[inline] #[must_use]    pub const fn x(&self) -> f32 { self.0.x }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn y(&self) -> f32 { self.0.y }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn z(&self) -> f32 { self.0.z }
}

impl ChunkPosition {
    #[must_use]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3 { x, y, z })
    }

    #[rustfmt::skip] #[inline] #[must_use]    pub const fn x(&self) -> i32 { self.0.x }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn y(&self) -> i32 { self.0.y }
    #[rustfmt::skip] #[inline] #[must_use]    pub const fn z(&self) -> i32 { self.0.z }
}

impl From<Position> for ChunkPosition {
    fn from(position: Position) -> Self {
        Self(IVec3 {
            x: position.0.x / CHUNK_SIZE_I32,
            y: position.0.y / CHUNK_SIZE_I32,
            z: position.0.z / CHUNK_SIZE_I32,
        })
    }
}

impl From<ChunkPosition> for Position {
    fn from(chunk_position: ChunkPosition) -> Self {
        Self(IVec3 {
            x: chunk_position.0.x * CHUNK_SIZE_I32,
            y: chunk_position.0.y * CHUNK_SIZE_I32,
            z: chunk_position.0.z * CHUNK_SIZE_I32,
        })
    }
}

impl From<FloatingPosition> for Position {
    fn from(position: FloatingPosition) -> Self {
        Self(IVec3 {
            x: position.0.x.floor() as i32,
            y: position.0.y.floor() as i32,
            z: position.0.z.floor() as i32,
        })
    }
}

impl From<Position> for FloatingPosition {
    fn from(position: Position) -> Self {
        Self(Vec3 {
            x: position.0.x as f32,
            y: position.0.y as f32,
            z: position.0.z as f32,
        })
    }
}

impl From<ChunkPosition> for FloatingPosition {
    fn from(chunk_position: ChunkPosition) -> Self {
        Position::from(chunk_position).into()
    }
}

impl From<FloatingPosition> for ChunkPosition {
    fn from(position: FloatingPosition) -> Self {
        Position::from(position).into()
    }
}

macro_rules! impl_arithmetic_ops {
    ($type:ty) => {
        impl Add for $type {
            type Output = $type;

            fn add(self, other: Self) -> Self::Output {
                Self(self.0 + other.0)
            }
        }

        impl Sub for $type {
            type Output = $type;

            fn sub(self, other: Self) -> Self::Output {
                Self(self.0 - other.0)
            }
        }

        impl Mul for $type {
            type Output = $type;

            fn mul(self, other: Self) -> Self::Output {
                Self(self.0 * other.0)
            }
        }

        impl Div for $type {
            type Output = $type;

            fn div(self, other: Self) -> Self::Output {
                Self(self.0 / other.0)
            }
        }
    };
}

impl_arithmetic_ops!(Position);
impl_arithmetic_ops!(ChunkPosition);
impl_arithmetic_ops!(RelativePosition);
impl_arithmetic_ops!(FloatingPosition);
