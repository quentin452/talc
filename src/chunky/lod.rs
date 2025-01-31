/// level of detail
#[derive(Copy, Clone, Default)]
pub enum Lod {
    #[default]
    L32,
    L16,
    L8,
    L4,
    L2,
}

impl Lod {
    /// the amount of voxels per axis
    #[must_use]
    pub const fn size(self) -> i32 {
        match self {
            Self::L32 => 32,
            Self::L16 => 16,
            Self::L8 => 8,
            Self::L4 => 4,
            Self::L2 => 2,
        }
    }

    /// how much to multiply to reach next voxel
    /// lower lod gives higher jump
    #[must_use]
    pub const fn jump_index(self) -> i32 {
        match self {
            Self::L32 => 1,
            Self::L16 => 2,
            Self::L8 => 4,
            Self::L4 => 8,
            Self::L2 => 16,
        }
    }
}
