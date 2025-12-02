extern crate libchess;
use std::{collections::HashSet, io, time};

// use macroquad::prelude::*;

use libchess::{
    ZobristValues, color as chess_color, moves,
    piece::{self, bb},
    pos, uci,
};
use macroquad::input::KeyCode;

use crate::app;

#[derive(Clone, Copy, PartialEq)]
pub enum EngineInitPhase {
    SendUci,
    WaitUciOk,
    SendNewGame,
    WaitIsReady,
    End,
}

impl EngineInitPhase {
    fn cycle(&mut self) {
        *self = match self {
            EngineInitPhase::SendUci => EngineInitPhase::WaitUciOk,
            EngineInitPhase::WaitUciOk => EngineInitPhase::SendNewGame,
            EngineInitPhase::SendNewGame => EngineInitPhase::WaitIsReady,
            EngineInitPhase::WaitIsReady => EngineInitPhase::End,
            _ => EngineInitPhase::End,
        }
    }
}

#[derive(Default, Clone)]
pub struct Settings {
    pub position_fen: String,
    pub white_engine_path: Option<app::EnginePath>,
    pub black_engine_path: Option<app::EnginePath>,
    pub wtime_s: String,
    pub btime_s: String,
    pub wincrement_ms: String,
    pub bincrement_ms: String,
}

#[derive(Clone)]
pub struct PostRunInfo {
    pub position: pos::Position,
    pub app_state: app::State,
}

pub struct Game {
    sarting_fen: String,
    pub position: pos::Position,
    pub engines: [Option<uci::Engine>; 2],
    pub engine_init_phases: [EngineInitPhase; 2],
    pub wtime: time::Duration,
    pub btime: time::Duration,
    // pub wincrement: time::Duration,
    // pub bincrement: time::Duration,
    pub mouse_input_sqs: app::MouseInputSquares,
}

impl Game {
    pub fn new(options: &Settings, zb: &ZobristValues) -> io::Result<Self> {
        Ok(Self {
            sarting_fen: options.position_fen.clone(),
            position: pos::Position::from_fen(&options.position_fen, zb),
            engines: [
                match &options.white_engine_path {
                    Some(e) => Some(uci::Engine::new(&e.path)?),
                    None => None,
                },
                match &options.black_engine_path {
                    Some(e) => Some(uci::Engine::new(&e.path)?),
                    None => None,
                },
            ],
            engine_init_phases: [EngineInitPhase::SendUci; 2],
            mouse_input_sqs: app::MouseInputSquares {
                keys_down: HashSet::new(),
                down_left: None,
                up_left: None,
                down_right: None,
            },
            wtime: time::Duration::from_secs(options.wtime_s.parse().unwrap()),
            btime: time::Duration::from_secs(options.btime_s.parse().unwrap()),
            // wincrement: time::Duration::from_millis(options.wincrement_ms.parse().unwrap()),
            // bincrement: time::Duration::from_millis(options.bincrement_ms.parse().unwrap()),
        })
    }
}

impl Game {
    pub fn init_engines(&mut self) -> io::Result<()> {
        let mut idx = 0;
        for engine in &mut self.engines {
            if let Some(e) = engine {
                match self.engine_init_phases[idx] {
                    EngineInitPhase::SendUci => {
                        e.send(uci::UCI)?;
                        self.engine_init_phases[idx].cycle();
                    }
                    EngineInitPhase::WaitUciOk => {
                        if e.try_get(uci::UCI_OK).is_some() {
                            self.engine_init_phases[idx].cycle();
                        }
                    }
                    EngineInitPhase::SendNewGame => {
                        e.send(uci::NEW_GAME)?;
                        e.send(uci::IS_READY)?;
                        self.engine_init_phases[idx].cycle();
                    }
                    EngineInitPhase::WaitIsReady => {
                        if e.try_get(uci::READY_OK).is_some() {
                            self.engine_init_phases[idx].cycle();
                        }
                    }
                    _ => {}
                }
            }

            idx += 1;
        }

        Ok(())
    }

    fn make_move(&mut self, mov: moves::Move, zb: &ZobristValues) {
        self.position.make_move(mov, zb);
    }

    pub fn run(&mut self, lc_data: &app::LibchessData) -> PostRunInfo {
        let legal_moves = moves::gen_legal(&mut self.position, &lc_data.masks, &lc_data.zb);

        let mut post_run_info = PostRunInfo {
            position: pos::Position::blank(),
            app_state: app::State::InGame,
        };

        if legal_moves.len() == 0 {
            if self.position.is_check(&lc_data.masks) {
                post_run_info.app_state = app::State::GameFinish {
                    message: match self.position.side_to_move() {
                        chess_color::WHITE => "Black Wins By Checkmate",
                        chess_color::BLACK => "White Wins By Checkmate",
                        _ => panic!(),
                    },
                };
            } else {
                post_run_info.app_state = app::State::GameFinish {
                    message: "Draw By Stalemate",
                };
            }
        }

        if self.position.is_3_rep() {
            post_run_info.app_state = app::State::GameFinish {
                message: "Draw By Three-Fold Repetition",
            };
        }

        if self.position.rule50() == pos::RULE_50_PLIES {
            post_run_info.app_state = app::State::GameFinish {
                message: "Draw By Fifty Move Rule",
            };
        }

        if self.position.insufficient_material(chess_color::WHITE)
            && self.position.insufficient_material(chess_color::BLACK)
        {
            post_run_info.app_state = app::State::GameFinish {
                message: "Draw By Insufficient Material",
            };
        }

        if self.wtime.is_zero() {
            post_run_info.app_state = app::State::GameFinish {
                message: "Black Wins On Time",
            };
        }

        if self.btime.is_zero() {
            post_run_info.app_state = app::State::GameFinish {
                message: "White Wins On Time",
            };
        }

        if let Some(mov) = self.get_move() {
            if legal_moves.contains(&mov) {
                self.make_move(mov, &lc_data.zb);
            }
        }

        PostRunInfo {
            position: self.position.clone(),
            ..post_run_info
        }
    }

    fn get_player_move(&mut self) -> Option<moves::Move> {
        match self.mouse_input_sqs.down_left {
            Some(f_sq) => match self.mouse_input_sqs.up_left {
                Some(t_sq) => {
                    self.mouse_input_sqs.up_left = None;
                    self.mouse_input_sqs.down_left = None;
                    Some(moves::Move::from_str_move(
                        &format!(
                            "{}{}{}",
                            pos::to_algn(f_sq),
                            pos::to_algn(t_sq),
                            if (pos::to_algn(t_sq).ends_with("8")
                                || pos::to_algn(t_sq).ends_with("1"))
                                && self.position.piece_on(f_sq) & piece::PAWN != 0
                            {
                                if self.mouse_input_sqs.keys_down.contains(&KeyCode::Q) {
                                    "q"
                                } else if self.mouse_input_sqs.keys_down.contains(&KeyCode::R) {
                                    "r"
                                } else if self.mouse_input_sqs.keys_down.contains(&KeyCode::B) {
                                    "b"
                                } else if self.mouse_input_sqs.keys_down.contains(&KeyCode::N) {
                                    "n"
                                } else {
                                    ""
                                }
                            } else {
                                ""
                            }
                        ),
                        &self.position,
                    ))
                }
                None => None,
            },
            None => None,
        }
    }

    fn get_engine_move(&mut self) -> Option<moves::Move> {
        let e = self.engines[bb::c_to_idx(self.position.side_to_move())]
            .as_mut()
            .unwrap();

        e.request_move(
            &self.position,
            &self.sarting_fen,
            self.wtime.as_millis(),
            self.btime.as_millis(),
        )
        .ok()?;

        let mut mov = e.try_get_move(&self.position);
        while mov.is_none() {
            mov = e.try_get_move(&self.position);
        }

        return mov.unwrap();
    }

    fn get_move(&mut self) -> Option<moves::Move> {
        match self.engines[bb::c_to_idx(self.position.side_to_move())] {
            Some(_) => self.get_engine_move(),
            None => self.get_player_move(),
        }
    }
}
