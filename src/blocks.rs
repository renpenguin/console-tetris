use gemini_engine::elements::view::{utils, ColChar, Point, Vec2D, ViewElement};
mod block_data;
use block_data::BlockData;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockType {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
}

impl BlockType {
    const ALL_VARIANTS: [BlockType; 7] = [
        BlockType::I,
        BlockType::J,
        BlockType::L,
        BlockType::O,
        BlockType::S,
        BlockType::T,
        BlockType::Z,
    ];
    pub fn bag() -> [BlockType; 7] {
        let mut variants = BlockType::ALL_VARIANTS;
        variants.shuffle(&mut rand::thread_rng());
        variants
    }

    fn get_rotation_states(self) -> Vec<Vec<Vec2D>> {
        BlockData::from(self).rotation_states.clone()
    }
    fn get_colour(self) -> ColChar {
        ColChar::SOLID.with_colour(BlockData::from(self).colour)
    }
}

#[derive(Debug)]
pub struct Block {
    pub pos: Vec2D,
    pub block_shape: BlockType,
    rotation: isize,
    pub(super) is_ghost: bool,
}

impl Block {
    pub const DEFAULT: Block = Block::new(BlockType::O);

    pub const fn new(block_shape: BlockType) -> Block {
        Block {
            pos: Vec2D::new(5, 0),
            block_shape,
            rotation: 0,
            is_ghost: false,
        }
    }

    pub fn rotate(&mut self, clockwise: bool) {
        self.rotation += if clockwise { 1 } else { -1 }
    }
}

impl Clone for Block {
    fn clone(&self) -> Self {
        Self {
            pos: self.pos,
            block_shape: self.block_shape,
            rotation: self.rotation,
            is_ghost: false,
        }
    }
}

impl ViewElement for Block {
    fn active_pixels(&self) -> Vec<Point> {
        let rotation_states = self.block_shape.get_rotation_states();
        let block_colour = match self.is_ghost {
            true => ColChar::BACKGROUND, // .with_mod(Modifier::Colour(Colour::greyscale(255)))
            false => self.block_shape.get_colour(),
        };

        let block_points = rotation_states
            [self.rotation.rem_euclid(rotation_states.len() as isize) as usize]
            .iter()
            .flat_map(|p| {
                // Position block
                let mut positioned = *p + self.pos;

                // Widen block so that each pixels appears square
                positioned.x *= 2;
                vec![positioned, positioned + Vec2D::new(1, 0)]
            })
            .collect();

        utils::points_to_pixels(block_points, block_colour)
    }
}
