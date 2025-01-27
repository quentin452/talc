#[repr(u16)]
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
        !self.is_transparent()
    }
    #[must_use] pub const fn is_transparent(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Air => true,
            _ => false,
        }
    }
}
