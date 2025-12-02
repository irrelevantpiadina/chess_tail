use libchess::{
    color as chess_color, moves,
    piece::{self, bb},
    pos,
};

use crate::{app, visual_board::VisualBoard};

use macroquad::prelude::*;

pub fn do_board_mouse_events(
    vb: &mut VisualBoard,
    position: &pos::Position,
    game_input_sqs: &mut app::MouseInputSquares,
    engines: [&Option<app::EnginePath>; 2],
) {
    if position.ply() > vb.draw_ply {
        return; // user is looking at previous positions, so don't do anything
    }

    fn find_sq_and<F: FnMut(isize, isize)>(
        board_xy: Vec2,
        is_flipped: bool,
        sq_size: f32,
        mut c: F,
    ) {
        for f in 0..8 {
            for r in 0..8 {
                let x = board_xy.x + (f - is_flipped as isize * 7).abs() as f32 * sq_size;
                let y = board_xy.y + (r - !is_flipped as isize * 7).abs() as f32 * sq_size;
                let m = mouse_position();

                if m.0 > x && m.1 > y && m.0 < x + sq_size && m.1 < y + sq_size {
                    c(f, r);
                }
            }
        }
    }

    game_input_sqs.keys_down = get_keys_down();

    if is_mouse_button_pressed(MouseButton::Left) {
        find_sq_and(vb.pos, vb.is_flipped, vb.square_size, |f, r| {
            let c = chess_color::of(position.piece_on(pos::make_sq(f, r)));
            let e = &engines[bb::c_to_idx(c)];
            if e.is_none() {
                if game_input_sqs.down_left.is_some() {
                    vb.mouse_input_sqs.down_left = None;
                    vb.selected_piece = piece::NONE;

                    game_input_sqs.up_left = Some(pos::make_sq(f, r));
                } else {
                    vb.mouse_input_sqs.down_left = Some(pos::make_sq(f, r));
                    vb.selected_piece = position.piece_on(vb.mouse_input_sqs.down_left.unwrap());

                    game_input_sqs.down_left = Some(pos::make_sq(f, r));
                    game_input_sqs.up_left = None;
                }
            }

            vb.selected_squares.clear();
            vb.arrows.clear();
        });
    }

    if is_mouse_button_released(MouseButton::Left) {
        find_sq_and(vb.pos, vb.is_flipped, vb.square_size, |f, r| {
            if !game_input_sqs
                .down_left
                .is_some_and(|sq| sq == pos::make_sq(f, r))
            {
                vb.mouse_input_sqs.down_left = None;
                vb.selected_piece = piece::NONE;

                game_input_sqs.up_left = Some(pos::make_sq(f, r));
            }
        });
    }

    if is_mouse_button_pressed(MouseButton::Right) {
        find_sq_and(vb.pos, vb.is_flipped, vb.square_size, |f, r| {
            vb.mouse_input_sqs.down_right = Some(pos::make_sq(f, r));
        });
    }

    if is_mouse_button_released(MouseButton::Right) {
        find_sq_and(vb.pos, vb.is_flipped, vb.square_size, |tf, tr| {
            let sq = vb.mouse_input_sqs.down_right.unwrap();
            let (ff, fr) = pos::make_tuple(sq);

            if (ff, fr) != (tf, tr) && !vb.arrows.contains(&(ff, fr, tf, tr)) {
                vb.arrows.insert((ff, fr, tf, tr));
            } else if vb.arrows.contains(&(ff, fr, tf, tr)) {
                vb.arrows.remove(&(ff, fr, tf, tr));
            } else if !vb.selected_squares.contains(&(tf, tr)) {
                vb.selected_squares.insert((tf, tr));
            } else {
                vb.selected_squares.remove(&(tf, tr));
            }
        });

        vb.mouse_input_sqs.down_right = None;
    }
}

pub fn do_key_events(vb: &mut VisualBoard, position: &pos::Position) {
    if is_key_pressed(KeyCode::Left) {
        if vb.draw_ply > 0 {
            let mov = position.moves_opt()[vb.draw_ply].unwrap();
            vb.make_move(
                moves::Move::new(mov.to_sq(), mov.from_sq(), mov.type_of()),
                position.side_to_move(),
            );
            vb.draw_ply -= 1;
        }
    } else if is_key_pressed(KeyCode::Right) {
        if vb.draw_ply < position.ply() {
            vb.draw_ply += 1;
            let mov = position.moves_opt()[vb.draw_ply].unwrap();
            vb.make_move(mov, position.side_to_move());
        }
    } else if is_key_pressed(KeyCode::Up) {
        vb.draw_ply = position.ply();
        vb.sync_pieces(position);
    } else if is_key_pressed(KeyCode::Down) {
        vb.draw_ply = 0;
        vb.sync_pieces(&pos::Position::from_fen(
            pos::START_FEN,
            &libchess::init().1, // forgive me
        ));
    } else if is_key_pressed(KeyCode::F) {
        vb.is_flipped = !vb.is_flipped;
    }
}
