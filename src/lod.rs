/// level of detail
#[derive(Copy, Clone)]
pub enum Lod {
    L16,
    L8,
    L4,
    L2,
}

impl Lod {
    /// the amount of voxels per axis
    #[must_use]
    pub const fn size(&self) -> i32 {
        match self {
            Self::L16 => 16,
            Self::L8 => 8,
            Self::L4 => 4,
            Self::L2 => 2,
        }
    }

    /// how much to multiply to reach next voxel
    /// lower lod gives higher jump
    #[must_use]
    pub const fn jump_index(&self) -> i32 {
        match self {
            Self::L16 => 1,
            Self::L8 => 2,
            Self::L4 => 4,
            Self::L2 => 8,
        }
    }
}
