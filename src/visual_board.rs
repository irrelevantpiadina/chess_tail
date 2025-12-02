// most if not all magic numbers here are just simple offsets

use std::{collections::HashSet, fs, time};

use libchess::{
    color as chess_color, moves,
    piece::{self, bb},
    pos,
};

use macroquad::prelude::*;

use crate::app;

pub struct VisualPiece {
    pub current_square: pos::Square,
    pub current_pos: Vec2,
    pub desired_pos: Vec2,
    pub texture_idx: usize,
    pub movement_duration: time::Duration,
}
pub struct VisualBoard {
    // pub outline_thickness: f32,
    pub light_square: Color,
    pub dark_square: Color,
    pub is_flipped: bool,
    pub square_size: f32,
    pub pos: Vec2,
    pub eval_bar_width: f32,
    pub mouse_input_sqs: app::MouseInputSquares,
    pub draw_ply: usize,
    pub selected_piece: piece::Piece,
    pub pieces: Vec<VisualPiece>,
    pub selected_piece_theme: usize,
    pub selected_board_theme: usize,
    pub selected_squares: HashSet<(pos::File, pos::Rank)>,
    pub arrows: HashSet<(pos::File, pos::Rank, pos::File, pos::Rank)>,
    board_textures: Vec<Texture2D>,
    piece_textures: Vec<[Texture2D; 12]>,
}

impl VisualBoard {
    pub fn new(light_square: Color, dark_square: Color, flipped: bool) -> Self {
        VisualBoard {
            // outline_thickness: 15.0,
            light_square,
            dark_square,
            is_flipped: flipped,
            piece_textures: Vec::new(),
            board_textures: Vec::new(),
            square_size: 0.0,
            pos: vec2(0.0, 0.0),
            eval_bar_width: 0.0,
            mouse_input_sqs: app::MouseInputSquares {
                keys_down: HashSet::new(),
                up_left: None,
                down_left: None,
                down_right: None,
            },
            draw_ply: 0,
            selected_piece: piece::NONE,
            pieces: Vec::new(),
            selected_squares: HashSet::new(),
            arrows: HashSet::new(),
            selected_piece_theme: 0,
            selected_board_theme: 0,
        }
    }

    pub fn board_width(&self) -> f32 {
        self.square_size * 8.0
    }
}

impl VisualBoard {
    pub async fn load_piece_assets(&mut self, paths: &Vec<String>) {
        for path in paths {
            self.piece_textures.push([
                load_texture(format!("assets/pieces/{path}/wp.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/wn.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/wb.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/wr.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/wq.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/wk.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/bp.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/bn.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/bb.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/br.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/bq.png").as_str())
                    .await
                    .unwrap(),
                load_texture(format!("assets/pieces/{path}/bk.png").as_str())
                    .await
                    .unwrap(),
            ]);
        }
    }

    pub async fn load_board_assets(&mut self) {
        for board in fs::read_dir("assets/boards/")
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<String>>()
        {
            self.board_textures.push(
                load_texture(format!("assets/boards/{board}").as_str())
                    .await
                    .unwrap(),
            );
        }
    }

    pub fn sync_pieces(&mut self, pos: &pos::Position) {
        let pos = pos.from_ply(self.draw_ply);

        self.pieces.clear();

        for sq in 0..64 {
            if pos.piece_on(sq) == piece::NONE {
                continue;
            }

            let vec2 = {
                let (f, r) = pos::make_tuple(sq);
                let vec2 = vec2(
                    (f - 7 * self.is_flipped as isize).abs() as f32 * self.square_size,
                    (r - 7 * !self.is_flipped as isize).abs() as f32 * self.square_size,
                );

                self.pos + vec2
            };

            self.pieces.push(VisualPiece {
                current_square: sq,
                current_pos: vec2,
                desired_pos: vec2,
                texture_idx: bb::p_to_idx(pos.piece_on(sq)),
                movement_duration: time::Duration::ZERO,
            });
        }
    }

    pub fn make_move(&mut self, mov: moves::Move, side_to_move: chess_color::Color) {
        match mov.type_of() {
            moves::MoveType::Capture(_) | moves::MoveType::PromoCapture(_, _) => {
                let idx = self
                    .pieces
                    .iter()
                    .position(|p| p.current_square == mov.to_sq());

                if let Some(idx) = idx {
                    self.pieces.remove(idx);
                }
            }
            moves::MoveType::KingSideCastle => {
                for piece in &mut self.pieces {
                    if piece.current_square == pos::WK_ROOK_SQ && side_to_move == chess_color::BLACK
                        || piece.current_square == pos::BK_ROOK_SQ
                            && side_to_move == chess_color::WHITE
                    {
                        piece.current_square -= 2;
                    }
                }
            }

            moves::MoveType::QueenSideCastle => {
                for piece in &mut self.pieces {
                    if piece.current_square == pos::WQ_ROOK_SQ && side_to_move == chess_color::BLACK
                        || piece.current_square == pos::BQ_ROOK_SQ
                            && side_to_move == chess_color::WHITE
                    {
                        piece.current_square += 3;
                    }
                }
            }
            _ => {}
        }
        for piece in &mut self.pieces {
            // piece.movement_duration = time::Duration::ZERO;
            if piece.current_square == mov.from_sq() {
                piece.current_square = mov.to_sq();
            }
        }
    }

    fn highlight(&self, file: isize, rank: isize, color: Color) {
        draw_rectangle(
            self.pos.x + (file - self.is_flipped as isize * 7).abs() as f32 * self.square_size,
            self.pos.y + (rank - !self.is_flipped as isize * 7).abs() as f32 * self.square_size,
            self.square_size,
            self.square_size,
            color,
        );
    }

    fn highlight_move_squares(&self, mov: Option<moves::Move>) {
        if let Some(mov) = mov {
            let ((ff, fr), (tf, tr)) =
                (pos::make_tuple(mov.from_sq()), pos::make_tuple(mov.to_sq()));
            self.highlight(ff, fr, YELLOW.with_alpha(0.4));
            self.highlight(tf, tr, YELLOW.with_alpha(0.4));
        }
    }

    fn highlight_legal_moves(&self, legal_moves: Vec<moves::Move>) {
        if self.selected_piece != piece::NONE {
            let f_sq = self.mouse_input_sqs.down_left.unwrap();
            let squares = legal_moves
                .iter()
                .filter(|m| m.from_sq() == f_sq)
                .map(|m| m.to_sq())
                .collect::<Vec<pos::Square>>();

            for square in squares {
                let (tf, tr) = pos::make_tuple(square);
                self.highlight(tf, tr, RED.with_alpha(0.4));
            }
        }
    }

    fn highlight_selected_squares(&self) {
        for sq in &self.selected_squares {
            self.highlight(sq.0, sq.1, PURPLE.with_alpha(0.6));
        }
    }

    fn draw_arrows(&self) {
        for arrow in &self.arrows {
            let arrow_x1 = self.pos.x
                + (arrow.0 - self.is_flipped as isize * 7).abs() as f32 * self.square_size
                + self.square_size / 2.0;
            let arrow_y1 = self.pos.y
                + (arrow.1 - !self.is_flipped as isize * 7).abs() as f32 * self.square_size
                + self.square_size / 2.0;
            let arrow_x2 = self.pos.x
                + (arrow.2 - self.is_flipped as isize * 7).abs() as f32 * self.square_size
                + self.square_size / 2.0;
            let arrow_y2 = self.pos.y
                + (arrow.3 - !self.is_flipped as isize * 7).abs() as f32 * self.square_size
                + self.square_size / 2.0;

            let thickness = self.square_size / 3.0;

            draw_line(
                arrow_x1,
                arrow_y1,
                arrow_x2,
                arrow_y2,
                thickness,
                ORANGE.with_alpha(0.7),
            );

            let angle = {
                let m = (arrow_y2 - arrow_y1) / (arrow_x2 - arrow_x1);
                m.atan()
            };

            let point1 = {
                let x1 = arrow_x2;
                let y1 = arrow_y2 + thickness;

                let x2 = (x1 - arrow_x2) * angle.cos() - (y1 - arrow_y2) * angle.sin();
                let y2 = (x1 - arrow_x2) * angle.sin() + (y1 - arrow_y2) * angle.cos();

                vec2(x2 + arrow_x2, y2 + arrow_y2)
            };
            let point2 = {
                let x1 = arrow_x2;
                let y1 = arrow_y2 - thickness;

                let x2 = (x1 - arrow_x2) * angle.cos() - (y1 - arrow_y2) * angle.sin();
                let y2 = (x1 - arrow_x2) * angle.sin() + (y1 - arrow_y2) * angle.cos();

                vec2(x2 + arrow_x2, y2 + arrow_y2)
            };
            let point3 = {
                let x1 = arrow_x2
                    + if arrow_x2 < arrow_x1 {
                        -thickness
                    } else {
                        thickness
                    };
                let y1 = arrow_y2;

                let x2 = (x1 - arrow_x2) * angle.cos() - (y1 - arrow_y2) * angle.sin();
                let y2 = (x1 - arrow_x2) * angle.sin() + (y1 - arrow_y2) * angle.cos();

                vec2(x2 + arrow_x2, y2 + arrow_y2)
            };

            draw_triangle(point1, point2, point3, ORANGE.with_alpha(0.7));
        }
    }

    fn draw_board_squares(&mut self, xy: Vec2, square_size: f32) {
        let params = DrawTextureParams {
            dest_size: Some(vec2(self.square_size * 8.0, self.square_size * 8.0)),
            source: None,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            pivot: None,
        };

        draw_texture_ex(
            &self.board_textures[self.selected_board_theme],
            xy.x,
            xy.y,
            WHITE,
            params,
        );

        if (self.square_size, self.pos) != (square_size, xy) {
            for piece in &mut self.pieces {
                let new_pos = {
                    let (f, r) = pos::make_tuple(piece.current_square);
                    let vec2 = vec2(f as f32 * square_size, (r - 7).abs() as f32 * square_size);
                    xy + vec2
                };

                piece.current_pos = new_pos;
                piece.desired_pos = new_pos;
            }
        }

        self.square_size = square_size;
        self.pos = xy;
    }

    fn draw_square_coords(&self) {
        let font_size = 20.0;

        for f in 0..8isize {
            draw_text(
                ((f as u8 + b'a') as char).to_string().as_str(),
                self.pos.x
                    + (f - self.is_flipped as isize * 7).abs() as f32 * self.square_size
                    + 5.0,
                self.pos.y + 8.0 * self.square_size,
                font_size,
                if f % 2 == 0 {
                    if !self.is_flipped {
                        self.dark_square
                    } else {
                        self.light_square
                    }
                } else {
                    if self.is_flipped {
                        self.dark_square
                    } else {
                        self.light_square
                    }
                },
            );
        }

        for r in 0..8isize {
            draw_text(
                (r - 8).abs().to_string().as_str(),
                self.pos.x,
                self.pos.y
                    + (r - self.is_flipped as isize * 7).abs() as f32 * self.square_size
                    + 15.0,
                font_size,
                if r % 2 == 0 {
                    if self.is_flipped {
                        self.dark_square
                    } else {
                        self.light_square
                    }
                } else {
                    if !self.is_flipped {
                        self.dark_square
                    } else {
                        self.light_square
                    }
                },
            );
        }
    }

    fn draw_pieces(&mut self, pos: &pos::Position) {
        let params = DrawTextureParams {
            dest_size: Some(vec2(self.square_size, self.square_size)),
            source: None,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            pivot: None,
        };

        let mut sync = false;

        for piece in &mut self.pieces {
            piece.desired_pos = {
                let (f, r) = pos::make_tuple(piece.current_square);
                let vec2 = vec2(
                    (f - 7 * self.is_flipped as isize).abs() as f32 * self.square_size,
                    (r - 7 * !self.is_flipped as isize).abs() as f32 * self.square_size,
                );
                self.pos + vec2
            };

            if piece.current_pos != piece.desired_pos && !is_mouse_button_down(MouseButton::Left) {
                let t = piece.movement_duration.as_secs_f32() / 0.5;
                piece.current_pos += (piece.desired_pos - piece.current_pos) * vec2(t, t);

                if (piece.movement_duration.as_secs_f32() - 0.5).abs() < 0.01 {
                    piece.movement_duration = time::Duration::ZERO;
                    piece.current_pos = piece.desired_pos;
                } else {
                    piece.movement_duration += time::Duration::from_secs_f32(get_frame_time());
                }

                if piece.current_pos == piece.desired_pos {
                    sync = true;
                }
            }

            piece.current_pos = if self
                .mouse_input_sqs
                .down_left
                .is_some_and(|sq| sq == piece.current_square)
                && is_mouse_button_down(MouseButton::Left)
            {
                vec2(
                    mouse_position().0 - self.square_size / 2.0 + mouse_delta_position().x * 10.0,
                    mouse_position().1 - self.square_size / 2.0 + mouse_delta_position().y * 10.0,
                )
            } else {
                piece.current_pos
            };

            draw_texture_ex(
                &self.piece_textures[self.selected_piece_theme][piece.texture_idx],
                piece.current_pos.x,
                piece.current_pos.y,
                WHITE,
                params.clone(),
            );
        }

        if sync {
            self.sync_pieces(pos);
        }
    }

    pub fn draw_board(
        &mut self,
        position: Option<&mut pos::Position>,
        lc_data: &app::LibchessData,
    ) -> time::Duration {
        let draw_time = time::Instant::now();

        // self.draw_board_outline(LIGHTGRAY);

        self.draw_board_squares(
            vec2(
                self.eval_bar_width + 10.0,
                screen_height() / 2.0 - self.board_width() / 2.0,
            ),
            if screen_height() < screen_width() {
                screen_height()
            } else {
                screen_width()
            } / 8.5,
        );

        self.draw_square_coords();

        if let Some(position) = position {
            if position.moves_opt().len() > 0 {
                self.highlight_move_squares(position.moves_opt()[self.draw_ply])
            }

            self.highlight_selected_squares();
            self.highlight_legal_moves(moves::gen_legal(position, &lc_data.masks, &lc_data.zb));

            self.draw_pieces(position);
        } else {
            self.draw_ply = 0;
            self.draw_pieces(&pos::Position::from_fen(pos::START_FEN, &lc_data.zb));
        }

        self.draw_arrows();

        draw_time.elapsed()
    }
}
