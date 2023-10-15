use std::io::stdout;

use console_input::keypress::{exit_raw_mode, Input};
use crossterm::{
    cursor::MoveTo,
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};
use gemini_engine::{
    elements::{
        containers::CollisionContainer,
        view::{ColChar, Modifier, Wrapping},
        PixelContainer, Sprite, Text, Vec2D, View,
    },
    gameloop::MainLoopRoot,
};
mod alerts;
mod blocks;
mod borders;
mod pause;
use alerts::AlertDisplay;
use blocks::{block_manipulation as tetris_core, Block, BlockType};
use pause::pause;
use rand::Rng;

use self::alerts::generate_alert_for_filled_lines;

pub struct Game {
    view: View,
    alert_display: AlertDisplay,
    active_block: Option<Block>,
    ghost_block: Block,
    held_piece: Option<BlockType>,
    has_held: bool,
    game_boundaries: PixelContainer,
    stationary_blocks: PixelContainer,
    bag: Vec<BlockType>,
    placing_cooldown: u32,
    score: isize,
    t: usize,
    // Constants
    block_place_cooldown: u32,
    piece_preview_count: usize,
    controls_help_text: String,
}

impl Game {
    pub fn new(
        block_place_cooldown: u32,
        piece_preview_count: usize,
        controls_help_text: &str,
    ) -> Game {
        Game {
            view: View::new(50, 21, ColChar::EMPTY),
            alert_display: AlertDisplay::new(Vec2D::new(12, 7)),
            active_block: None,
            ghost_block: Block::DEFAULT,
            held_piece: None,
            has_held: false,
            game_boundaries: borders::generate_borders(),
            stationary_blocks: PixelContainer::new(),
            bag: BlockType::bag()[0..rand::thread_rng().gen_range(1..8)].to_vec(),
            placing_cooldown: block_place_cooldown,
            score: 0,
            t: 0,
            // Constants
            block_place_cooldown,
            piece_preview_count,
            controls_help_text: controls_help_text.to_string(),
        }
    }
}

impl MainLoopRoot for Game {
    type InputDataType = Event;

    fn frame(&mut self, input_data: Option<Self::InputDataType>) {
        let mut block_speed = 12;

        let collision = CollisionContainer::from(vec![
            &self.game_boundaries as _,
            &self.stationary_blocks as _,
        ]);

        let mut block = match self.active_block {
            Some(ref block) => block.clone(),
            None => {
                let next_piece = self.bag.pop().unwrap();
                if self.bag.len() <= self.piece_preview_count {
                    let mut new_bag = BlockType::bag().to_vec();
                    new_bag.extend(&self.bag);
                    self.bag.clear();
                    self.bag.extend(new_bag);
                }

                Block::new(next_piece)
            }
        };

        // Handle user input
        if let Some(Event::Key(key_event)) = input_data {
            match key_event {
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.view.clear();
                    self.view.display_render().unwrap();
                    pause();
                }

                KeyEvent {
                    code: KeyCode::Left, // Shift left
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    if tetris_core::try_move_block(&collision, &mut block, Vec2D::new(-1, 0)) {
                        self.placing_cooldown = self.block_place_cooldown;
                    }
                }

                KeyEvent {
                    code: KeyCode::Right, // Shift right
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    if tetris_core::try_move_block(&collision, &mut block, Vec2D::new(1, 0)) {
                        self.placing_cooldown = self.block_place_cooldown;
                    }
                }

                KeyEvent {
                    code: KeyCode::Char('z'), // Rotate Anti-clockwise
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    if tetris_core::try_rotate_block(&collision, &mut block, false) {
                        self.placing_cooldown = self.block_place_cooldown;
                    }
                }

                KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('x'), // Rotate Clockwise
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    if tetris_core::try_rotate_block(&collision, &mut block, true) {
                        self.placing_cooldown = self.block_place_cooldown;
                    }
                }

                KeyEvent {
                    code: KeyCode::Down, // Soft Drop
                    kind: KeyEventKind::Press,
                    ..
                } => block_speed = 2,

                KeyEvent {
                    code: KeyCode::Char(' '), // Hard drop
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.ghost_block = tetris_core::generate_ghost_block(&collision, &block);
                    self.score += self.ghost_block.pos.y - block.pos.y;
                    block = self.ghost_block.clone();
                    self.t = block_speed - 1;
                    self.placing_cooldown = 1;
                }

                KeyEvent {
                    code: KeyCode::Char('c'), // Hold
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    if !self.has_held {
                        let current_held_piece = self.held_piece;
                        self.held_piece = Some(block.block_shape);
                        match current_held_piece {
                            Some(piece) => block = Block::new(piece),
                            None => {
                                self.active_block = None;
                                return;
                            }
                        }
                        self.has_held = true;
                    }
                }

                _ => (),
            }
        }

        self.ghost_block = tetris_core::generate_ghost_block(&collision, &block);

        let is_above_block = collision.will_overlap_element(&block, Vec2D::new(0, 1));

        self.t += 1;
        self.active_block = if self.t % block_speed == 0 || is_above_block {
            if tetris_core::try_move_block(&collision, &mut block, Vec2D::new(0, 1)) {
                if block_speed == 2 {
                    self.score += 1;
                }
                Some(block)
            } else {
                self.placing_cooldown -= 1;
                if self.placing_cooldown == 0 {
                    // Placing a block
                    let pre_clear_blocks = self.stationary_blocks.clone();
                    self.placing_cooldown = self.block_place_cooldown;
                    self.has_held = false;
                    self.stationary_blocks.blit(&block);
                    if block.pos.y < 1 {
                        println!("Game over!\r");
                        exit_raw_mode()
                    }
                    let cleared_lines =
                        tetris_core::clear_filled_lines(&mut self.stationary_blocks);
                    let mut alert = generate_alert_for_filled_lines(cleared_lines);
                    if let Some(t_spin_alert) = tetris_core::handle_t_spin(
                        &CollisionContainer::from(vec![&pre_clear_blocks as _]),
                        &block,
                        cleared_lines,
                    ) {
                        alert = Some(t_spin_alert)
                    }

                    self.alert_display.handle_with_score(&mut self.score, alert);
                    None
                } else {
                    Some(block)
                }
            }
        } else {
            Some(block)
        };
    }

    fn render_frame(&mut self) {
        self.view.clear();
        self.view
            .blit_double_width(&self.game_boundaries, Wrapping::Panic);
        self.view
            .blit_double_width(&self.stationary_blocks, Wrapping::Ignore);
        self.view
            .blit_double_width(&self.ghost_block, Wrapping::Ignore);
        if let Some(ref block) = self.active_block {
            self.view.blit_double_width(block, Wrapping::Ignore);
        }

        // Next piece display
        self.view.blit(
            &Text::new(Vec2D::new(29, 9), "Next:", Modifier::None),
            Wrapping::Panic,
        );

        for i in 0..self.piece_preview_count {
            let mut next_block_display = Block::new(self.bag[self.bag.len() - i - 1]);
            next_block_display.pos = Vec2D::new(15, 12 + i as isize * 3);
            self.view
                .blit_double_width(&next_block_display, Wrapping::Ignore);
        }

        // Held piece display
        if let Some(piece) = self.held_piece {
            self.view.blit(
                &Text::new(Vec2D::new(29, 1), "Hold", Modifier::None),
                Wrapping::Panic,
            );
            let mut held_block_display = Block::new(piece);
            held_block_display.pos = Vec2D::new(15, 4);
            self.view
                .blit_double_width(&held_block_display, Wrapping::Panic);
        } else {
            self.view.blit(
                &Sprite::new(Vec2D::new(26, 0), &self.controls_help_text, Modifier::None),
                Wrapping::Panic,
            );
        }

        // Score display
        self.view.blit(
            &Text::new(
                Vec2D::new(26, 7),
                &format!("Score: {}", self.score),
                Modifier::None,
            ),
            Wrapping::Panic,
        );

        // Alerts display
        self.view.blit(&self.alert_display, Wrapping::Ignore);
        self.alert_display.frame();

        execute!(stdout(), MoveTo(0, 0)).unwrap();
        execute!(stdout(), Clear(ClearType::FromCursorDown)).unwrap();
        self.view.display_render().unwrap();
    }

    fn sleep_and_get_input_data(
        &self,
        fps: f32,
        elapsed: std::time::Duration,
    ) -> (bool, Option<Self::InputDataType>) {
        Input::sleep_fps_and_get_input(fps, elapsed)
            .exit_on_kb_interrupt()
            .as_tuple()
    }
}