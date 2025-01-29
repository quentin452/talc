use crate::lod::Lod;
use bevy::math::{IVec3, ivec3};

// helper for transforming translations based dir or "axis"
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum FaceDir {
    Up,
    Down,
    Left,
    Right,
    Forward,
    Back,
}

impl FaceDir {
    /// normal data is packed in the shader
    #[must_use]
    pub const fn normal_index(&self) -> u32 {
        match self {
            Self::Left => 0u32,
            Self::Right => 1u32,
            Self::Down => 2u32,
            Self::Up => 3u32,
            Self::Forward => 4u32,
            Self::Back => 5u32,
        }
    }

    /// direction to sample face culling
    #[must_use]
    pub const fn air_sample_dir(&self) -> IVec3 {
        match self {
            Self::Up => IVec3::Y,
            Self::Down => IVec3::NEG_Y,
            Self::Left => IVec3::NEG_X,
            Self::Right => IVec3::X,
            Self::Forward => IVec3::NEG_Z,
            Self::Back => IVec3::Z,
        }
    }

    /// offset input position with this face direction
    #[must_use]
    pub const fn world_to_sample(&self, axis: i32, x: i32, y: i32, _lod: &Lod) -> IVec3 {
        match self {
            Self::Up => ivec3(x, axis + 1, y),
            Self::Down => ivec3(x, axis, y),
            Self::Left => ivec3(axis, y, x),
            Self::Right => ivec3(axis + 1, y, x),
            Self::Forward => ivec3(x, y, axis),
            Self::Back => ivec3(x, y, axis + 1),
        }
    }

    /// returns true if vertices should be reverse.
    /// (needed because indices are always same)  
    #[must_use]
    pub const fn reverse_order(&self) -> bool {
        match self {
            Self::Up => true,      //+1
            Self::Down => false,   //-1
            Self::Left => false,   //-1
            Self::Right => true,   //+1
            Self::Forward => true, //-1
            Self::Back => false,   //+1
        }
    }

    /// get delta for traversing the previous axis pos
    #[must_use]
    pub const fn negate_axis(&self) -> i32 {
        match self {
            Self::Up => -1,     //+1
            Self::Down => 0,    //-1
            Self::Left => 0,    //-1
            Self::Right => -1,  //+1
            Self::Forward => 0, //-1
            Self::Back => 1,    //+1
        }
    }
}
