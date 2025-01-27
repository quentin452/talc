#[repr(u32)]
#[derive(Eq, PartialEq, Default, Copy, Clone, Debug)]
pub enum BlockType {
    #[default]
    Air,
    Grass,
    Dirt,
}

pub const MESHABLE_BLOCK_TYPES: &[BlockType] = &[BlockType::Grass, BlockType::Dirt];

impl BlockType {
    #[must_use] pub const fn is_solid(&self) -> bool {
        match self {
            Self::Air => false,
            Self::Grass => true,
            Self::Dirt => true,
        }
    }
    #[must_use] pub const fn is_air(&self) -> bool {
        !self.is_solid()
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct BlockData {
    pub block_type: BlockType,
}
