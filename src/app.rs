use std::{collections::HashSet, fs, sync, thread, time};

use macroquad::{
    prelude::*,
    ui::{self, hash, widgets},
};

use crate::{events, game, ui_skins, visual_board as vb};
use libchess::{self as lc, color as chess_color, pos};

const HUMAN_PLAYER: usize = 0;
// const ROUGHLY_THE_MAX_WIDTH_OF_CLOCK_TEXT: f32 = 261.34401;

static GAME: sync::LazyLock<sync::Mutex<Option<game::Game>>> =
    sync::LazyLock::new(|| sync::Mutex::new(None));

#[derive(Clone)]
pub struct MouseInputSquares {
    pub keys_down: HashSet<KeyCode>, // clearly doesn't belong, but...
    pub up_left: Option<pos::Square>,
    pub down_left: Option<pos::Square>,
    pub down_right: Option<pos::Square>,
}

#[derive(Default, Clone)]
pub struct EnginePath {
    pub path: String,
    pub name: String,
}

#[derive(Clone)]
pub struct LibchessData {
    pub masks: lc::AttackMasks,
    pub zb: lc::ZobristValues,
}

#[derive(Clone)]
pub enum State {
    AssetLoading {
        piece_paths: Vec<String>,
    },
    Home,
    GameSelection,
    TryStartGame {
        timer: time::Duration,
        max_time: time::Duration,
    },
    TryStartFailed {
        reason: String,
    },
    InGame,
    GameFinish {
        message: &'static str,
    },
}

pub struct App {
    pub state: State,
    // pub game: sync::Mutex<Option<game::Game>>,
    pub game_settings: game::Settings,
    pub engines_list: Vec<EnginePath>,
    pub vb: vb::VisualBoard,
    pub font: Font,
    pub ui_skin: ui::Skin,
    pub lc_data: LibchessData,
}

trait Clock {
    fn to_clock(&self) -> String;
}

impl Clock for time::Duration {
    fn to_clock(&self) -> String {
        if self.as_secs() >= 60 {
            let seconds = self.as_secs() % 60;
            let minutes = self.as_secs() / 60;

            format!("{minutes}:{seconds:02}")
        } else {
            format!("{}", self.as_secs_f32())
                .chars()
                .take(4)
                .collect::<String>()
                + "s"
        }
    }
}

impl App {
    pub async fn init() -> Self {
        let light = Color {
            r: 184.0 / 255.0,
            g: 135.0 / 255.0,
            b: 98.0 / 255.0,
            a: 255.0 / 255.0,
        };
        let dark = Color {
            r: 237.0 / 255.0,
            g: 214.0 / 255.0,
            b: 176.0 / 255.0,
            a: 255.0 / 255.0,
        };

        let (masks, zb) = lc::init();

        let mut engines_list = Vec::new();

        for entry in fs::read_dir("assets/engines/").unwrap() {
            let entry = entry.as_ref().unwrap().file_name().into_string().unwrap();

            if !entry.contains(".exe") {
                continue;
            }
            engines_list.push(EnginePath {
                path: format!("assets/engines/{}", entry),
                name: entry
                    .char_indices()
                    .map(|c| {
                        if c.0 == 0 {
                            c.1.to_ascii_uppercase()
                        } else {
                            c.1.to_ascii_lowercase()
                        }
                    })
                    .collect::<String>()
                    .strip_suffix(".exe")
                    .unwrap()
                    .to_string(),
            });
        }

        let mut a = Self {
            state: State::AssetLoading {
                piece_paths: fs::read_dir("assets/pieces/")
                    .unwrap()
                    .map(|e| e.unwrap().file_name().into_string().unwrap())
                    .collect::<Vec<String>>(),
            },
            // game: sync::Mutex::new(None),
            game_settings: game::Settings {
                position_fen: pos::START_FEN.to_string(),
                white_engine_path: None,
                black_engine_path: None,
                wtime_s: "60".to_string(),
                btime_s: "60".to_string(),
                wincrement_ms: "0".to_string(),
                bincrement_ms: "0".to_string(),
            },
            engines_list,
            vb: vb::VisualBoard::new(light, dark, false),
            font: load_ttf_font("assets/fonts/GoogleSansCode-Regular.ttf")
                .await
                .unwrap(),
            ui_skin: ui::root_ui().default_skin(),
            lc_data: LibchessData { masks, zb },
        };

        a.ui_skin = ui_skins::standard(&a.font);

        ui::root_ui().push_skin(&a.ui_skin);

        a.vb.sync_pieces(&pos::Position::from_fen(pos::START_FEN, &a.lc_data.zb));

        a
    }

    pub async fn run(&mut self) {
        loop {
            match self.state {
                State::AssetLoading { .. } => self.load_assets().await,
                State::Home => self.home().await,
                State::GameSelection { .. } => self.game_selection().await,
                State::TryStartGame { .. } => self.try_start_game().await,
                State::TryStartFailed { .. } => self.try_start_failed().await,
                State::InGame => self.in_game().await,
                State::GameFinish { .. } => self.game_finish().await,
            }

            next_frame().await;
        }
    }

    fn ui_window(&self, title: &str) -> widgets::Window {
        widgets::Window::new(hash!(), self.ui_window_pos(), self.ui_window_size())
            .movable(false)
            .titlebar(true)
            .label(title)
    }

    fn ui_window_size(&self) -> Vec2 {
        let tiny_margin = 10.0;

        let width = if let State::InGame = self.state {
            self.vb.board_width() / 2.0
        } else {
            self.vb.board_width()
        };

        vec2(
            screen_width()
                - (self.vb.board_width() + self.vb.pos.x + self.vb.eval_bar_width + 10.0)
                - tiny_margin,
            width,
        )
    }

    fn ui_window_pos(&self) -> Vec2 {
        vec2(self.vb.pos.x + self.vb.board_width() + 10.0, self.vb.pos.y)
    }

    async fn load_assets(&mut self) {
        let piece_paths = if let State::AssetLoading { piece_paths } = &self.state {
            piece_paths
        } else {
            unreachable!()
        };

        self.vb.load_piece_assets(&piece_paths).await;
        self.vb.load_board_assets().await;

        self.state = State::Home;
    }

    async fn home(&mut self) {
        let mut should_break = false;

        loop {
            clear_background(DARKGRAY);
            self.vb.draw_board(None, &self.lc_data);

            self.ui_window("Home").ui(&mut ui::root_ui(), |mut ui| {
                if widgets::Button::new("New Game")
                    .position(vec2(0.0, 0.0))
                    .size(vec2(self.ui_window_size().x / 2.0 - 2.5, 100.0))
                    .ui(&mut ui)
                {
                    self.state = State::GameSelection;
                    should_break = true;
                }
                widgets::Button::new("Multiplayer (eventually)")
                    .position(vec2(self.ui_window_size().x / 2.0 + 5.0, 0.0))
                    .size(vec2(self.ui_window_size().x / 2.0 - 2.5, 100.0))
                    .ui(&mut ui);

                let tmp = fs::read_dir("assets/pieces/")
                    .unwrap()
                    .map(|e| e.unwrap().file_name().into_string().unwrap())
                    .collect::<Vec<String>>();

                for _ in 0..45 {
                    ui.separator();
                }

                self.vb.selected_piece_theme = ui.combo_box(
                    hash!(),
                    "Piece Theme",
                    &tmp.iter().map(|t| t.as_str()).collect::<Vec<&str>>(),
                    None,
                );

                let tmp = fs::read_dir("assets/boards/")
                    .unwrap()
                    .map(|e| {
                        let s = e.unwrap().file_name().into_string().unwrap();
                        s.strip_suffix(".png").unwrap().to_string()
                    })
                    .collect::<Vec<String>>();
                self.vb.selected_board_theme = ui.combo_box(
                    hash!(),
                    "Board Theme",
                    &tmp.iter().map(|t| t.as_str()).collect::<Vec<&str>>(),
                    None,
                );
            });

            if should_break {
                break;
            }

            next_frame().await
        }
    }

    async fn game_selection(&mut self) {
        let mut list = vec!["None"];
        list.append(
            &mut self
                .engines_list
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<&str>>(),
        );

        let mut should_break = false;

        loop {
            clear_background(DARKGRAY);
            self.vb.draw_board(None, &self.lc_data);

            self.ui_window("Game Selection")
                .ui(&mut ui::root_ui(), |ui| {
                    let white_player = ui.combo_box(hash!(), "White Engine", &list, None);
                    let black_player = ui.combo_box(hash!(), "Black Engine", &list, None);

                    for _ in 0..10 {
                        // no idea how to make the separator larger
                        ui.separator();
                    }

                    ui.input_text(
                        hash!(),
                        "White Time (seconds)",
                        &mut self.game_settings.wtime_s,
                    );
                    ui.input_text(
                        hash!(),
                        "Black Time (seconds)",
                        &mut self.game_settings.btime_s,
                    );

                    if self.game_settings.wtime_s.len() > 7 {
                        self.game_settings.wtime_s.truncate(7);
                    }

                    if self.game_settings.btime_s.len() > 7 {
                        self.game_settings.btime_s.truncate(7);
                    }

                    for _ in 0..10 {
                        ui.separator();
                    }

                    ui.input_text(
                        hash!(),
                        "White Increment (milliseconds)",
                        &mut self.game_settings.wincrement_ms,
                    );

                    ui.input_text(
                        hash!(),
                        "Black Increment (milliseconds)",
                        &mut self.game_settings.bincrement_ms,
                    );

                    ui.input_text(hash!(), "FEN", &mut self.game_settings.position_fen);
                    ui.label(
                        None,
                        "(!) An incorrect fen string will crash the application and I am too lazy to fix it",
                    );

                    if self.game_settings.wincrement_ms.len() > 7 {
                        self.game_settings.wincrement_ms.truncate(7);
                    }

                    if self.game_settings.bincrement_ms.len() > 7 {
                        self.game_settings.bincrement_ms.truncate(7);
                    }

                    if self.game_settings.position_fen.replace(" ", "") == "".to_string() {
                        self.game_settings.position_fen = pos::START_FEN.to_string();
                    }

                    for _ in 0..10 {
                        ui.separator();
                    }

                    if ui.button(None, "Play")
                        && self.game_settings.wtime_s != ""
                        && self.game_settings.btime_s != ""
                        && self.game_settings.wincrement_ms != ""
                        && self.game_settings.bincrement_ms != ""
                        && self.game_settings.wtime_s.parse::<u64>().is_ok()
                        && self.game_settings.btime_s.parse::<u64>().is_ok()
                        && self.game_settings.wincrement_ms.parse::<u64>().is_ok()
                        && self.game_settings.bincrement_ms.parse::<u64>().is_ok()
                    {
                        self.game_settings.white_engine_path = if white_player == HUMAN_PLAYER {
                            None
                        } else {
                            Some(self.engines_list[white_player - 1].clone())
                        };

                        self.game_settings.black_engine_path = if black_player == HUMAN_PLAYER {
                            None
                        } else {
                            Some(self.engines_list[black_player - 1].clone())
                        };

                        self.state = State::TryStartGame {
                            timer: time::Duration::ZERO,
                            max_time: time::Duration::from_secs(15),
                        };

                        should_break = true;
                    }

                    ui.same_line(50.0);

                    if ui.button(None, "Back") {
                        self.state = State::Home;
                        should_break = true;
                    }
                });

            if should_break {
                break;
            }

            next_frame().await
        }
    }

    async fn try_start_game(&mut self) {
        let mut game = GAME.lock().unwrap();

        let (mut timer, max_time) = if let State::TryStartGame { timer, max_time } = self.state {
            (timer, max_time)
        } else {
            unreachable!();
        };

        match game::Game::new(&self.game_settings, &self.lc_data.zb) {
            Ok(g) => *game = Some(g),
            Err(e) => {
                self.state = State::TryStartFailed {
                    reason: e.to_string(),
                };
                return;
            }
        }

        self.vb.draw_ply = 0;

        loop {
            clear_background(DARKGRAY);
            self.vb.draw_board(None, &self.lc_data);

            self.ui_window("").ui(&mut ui::root_ui(), |ui| {
                widgets::Label::new("Starting Game...")
                    .size(vec2(200.0, 100.0))
                    .position(self.ui_window_size() / 2.0 - vec2(100.0, 70.0))
                    .ui(ui);

                widgets::Label::new(format!("{}s", timer.as_secs()))
                    .size(vec2(200.0, 100.0))
                    .position(self.ui_window_size() / 2.0 - vec2(100.0, 50.0))
                    .ui(ui);
            });

            if game.as_ref().unwrap().engines[0].is_some()
                || game.as_ref().unwrap().engines[1].is_some()
            {
                match game.as_mut().unwrap().init_engines() {
                    Err(e) => {
                        self.state = State::TryStartFailed {
                            reason: e.to_string(),
                        };
                        break;
                    }
                    _ => {}
                }
            }

            if (game.as_ref().unwrap().engine_init_phases[0] == game::EngineInitPhase::End
                || game.as_ref().unwrap().engines[0].is_none())
                && (game.as_ref().unwrap().engine_init_phases[1] == game::EngineInitPhase::End
                    || game.as_ref().unwrap().engines[1].is_none())
            {
                self.state = State::InGame;
                break;
            } else {
                timer += time::Duration::from_secs_f32(get_frame_time());
                if timer >= max_time {
                    self.state = State::TryStartFailed {
                        reason: "Couldn't Initialize Engines".to_string(),
                    };
                    break;
                }
            }

            self.state = State::TryStartGame {
                timer: timer,
                max_time: max_time,
            };

            next_frame().await
        }
    }

    async fn try_start_failed(&mut self) {
        let reason = if let State::TryStartFailed { reason } = &self.state {
            reason.clone()
        } else {
            unreachable!();
        };

        let mut should_break = false;

        loop {
            clear_background(DARKGRAY);
            self.vb.draw_board(None, &self.lc_data);

            self.ui_window("").ui(&mut ui::root_ui(), |ui| {
                widgets::Label::new(format!("Failed To Start Game: {reason}"))
                    .size(vec2(200.0, 100.0))
                    .position(self.ui_window_size() / 2.0 - vec2(100.0, 70.0))
                    .ui(ui);

                if widgets::Button::new("Try Again")
                    .size(vec2(100.0, 20.0))
                    .position(self.ui_window_size() / 2.0 - vec2(100.0, 5.0))
                    .ui(ui)
                {
                    self.state = State::TryStartGame {
                        timer: time::Duration::ZERO,
                        max_time: time::Duration::from_secs(5),
                    };

                    should_break = true;
                }

                if widgets::Button::new("Back")
                    .size(vec2(100.0, 20.0))
                    .position(self.ui_window_size() / 2.0 - vec2(-20.0, 5.0))
                    .ui(ui)
                {
                    self.state = State::GameSelection;
                    should_break = true;
                }
            });

            if should_break {
                break;
            }

            next_frame().await
        }
    }

    async fn in_game(&mut self) {
        static POST_RUN_INFO: sync::LazyLock<sync::Mutex<game::PostRunInfo>> =
            sync::LazyLock::new(|| {
                sync::Mutex::new(game::PostRunInfo {
                    position: pos::Position::blank(),
                    app_state: State::InGame,
                })
            });

        static GAME_INPUT_SQUARES: sync::LazyLock<sync::Mutex<MouseInputSquares>> =
            sync::LazyLock::new(|| {
                sync::Mutex::new(MouseInputSquares {
                    keys_down: HashSet::new(),
                    up_left: None,
                    down_left: None,
                    down_right: None,
                })
            });

        static LC_DATA_MTX: sync::LazyLock<sync::Mutex<LibchessData>> = sync::LazyLock::new(|| {
            let (masks, zb) = libchess::init();
            sync::Mutex::new(LibchessData { masks, zb })
        });

        static WTIME_MTX: sync::LazyLock<sync::Mutex<time::Duration>> =
            sync::LazyLock::new(|| sync::Mutex::new(time::Duration::ZERO));

        static BTIME_MTX: sync::LazyLock<sync::Mutex<time::Duration>> =
            sync::LazyLock::new(|| sync::Mutex::new(time::Duration::ZERO));

        static BREAK_THREAD_LOOP: sync::LazyLock<sync::Mutex<bool>> =
            sync::LazyLock::new(|| sync::Mutex::new(false));

        *POST_RUN_INFO.lock().unwrap() = game::PostRunInfo {
            position: pos::Position::from_fen(&self.game_settings.position_fen, &self.lc_data.zb),
            app_state: State::InGame,
        };

        *WTIME_MTX.lock().unwrap() =
            time::Duration::from_secs(self.game_settings.wtime_s.parse().unwrap());
        *BTIME_MTX.lock().unwrap() =
            time::Duration::from_secs(self.game_settings.btime_s.parse().unwrap());

        let wincrement =
            time::Duration::from_millis(self.game_settings.wincrement_ms.parse().unwrap());
        let bincrement =
            time::Duration::from_millis(self.game_settings.bincrement_ms.parse().unwrap());

        *BREAK_THREAD_LOOP.lock().unwrap() = false;

        let game_run_thread = thread::spawn(|| {
            loop {
                if *BREAK_THREAD_LOOP.lock().unwrap() {
                    break;
                }

                let mut binding = GAME.lock().unwrap();
                let game = binding.as_mut().unwrap();

                game.wtime = *WTIME_MTX.lock().unwrap();
                game.btime = *BTIME_MTX.lock().unwrap();

                game.mouse_input_sqs = (*GAME_INPUT_SQUARES.lock().unwrap()).clone();
                let p = game.run(&LC_DATA_MTX.lock().unwrap());
                *POST_RUN_INFO.lock().unwrap() = p;
            }
        });

        let white = match &self.game_settings.white_engine_path {
            Some(p) => p.name.as_str(),
            None => "White",
        };

        let black = match &self.game_settings.black_engine_path {
            Some(p) => p.name.as_str(),
            None => "Black",
        };

        let mut last_moves_count = 1;

        let mut clock_text_width = 0.0;

        self.vb
            .sync_pieces(&POST_RUN_INFO.lock().unwrap().clone().position);

        loop {
            let post_run_info_cpy = POST_RUN_INFO.lock().unwrap().clone();

            events::do_board_mouse_events(
                &mut self.vb,
                &post_run_info_cpy.position,
                &mut GAME_INPUT_SQUARES.lock().unwrap(),
                [
                    &self.game_settings.white_engine_path,
                    &self.game_settings.black_engine_path,
                ],
            );

            let input_sqs = GAME_INPUT_SQUARES.lock().unwrap().clone();
            let (down, up) = (input_sqs.down_left, input_sqs.up_left);

            // this is necessary because otherwise user input breaks
            if down.is_some() && up.is_some() {
                thread::sleep(time::Duration::from_micros(50));
                *GAME_INPUT_SQUARES.lock().unwrap() = MouseInputSquares {
                    keys_down: HashSet::new(),
                    down_left: None,
                    up_left: None,
                    down_right: None,
                };
            }

            // need to sleep for a tiny bit to avoid piece flickering when making moves
            thread::sleep(time::Duration::from_micros(50));

            events::do_key_events(&mut self.vb, &post_run_info_cpy.position);

            let mut post_run_info_cpy = POST_RUN_INFO.lock().unwrap().clone();

            let current_move_count = post_run_info_cpy.position.moves_opt().len();

            let mut wtime_lock = WTIME_MTX.lock().unwrap();
            let mut btime_lock = BTIME_MTX.lock().unwrap();

            if last_moves_count != current_move_count {
                last_moves_count = current_move_count;
                if self.vb.draw_ply + 1 < post_run_info_cpy.position.moves().len() {
                    self.vb.sync_pieces(&post_run_info_cpy.position);
                }
                self.vb.draw_ply = current_move_count - 1;

                if post_run_info_cpy.position.side_to_move() == chess_color::WHITE {
                    *btime_lock += bincrement;
                } else {
                    *wtime_lock += wincrement;
                }

                if let Some(mov) = post_run_info_cpy.position.move_played() {
                    self.vb
                        .make_move(mov, post_run_info_cpy.position.side_to_move());
                }
            }

            self.state = post_run_info_cpy.app_state;

            if post_run_info_cpy.position.side_to_move() == chess_color::WHITE {
                *wtime_lock =
                    wtime_lock.saturating_sub(time::Duration::from_secs_f32(get_frame_time()));
            } else {
                *btime_lock =
                    btime_lock.saturating_sub(time::Duration::from_secs_f32(get_frame_time()));
            }

            drop(wtime_lock);
            drop(btime_lock);

            clear_background(DARKGRAY);
            self.vb
                .draw_board(Some(&mut post_run_info_cpy.position), &self.lc_data);

            let moves = post_run_info_cpy.position.moves();

            self.ui_window(format!("{white}  Vs  {black}").as_str())
                .ui(&mut ui::root_ui(), |ui| {
                    let mut idx = 0;
                    let mut fullmoves = 1;
                    loop {
                        if idx >= moves.len() {
                            break;
                        }

                        let mut fmt = format!("{fullmoves}. {} ({white})", moves[idx].to_uci_fmt());
                        if idx + 1 < moves.len() {
                            fmt += format!(", {} ({black})", moves[idx + 1].to_uci_fmt()).as_str();
                            fullmoves += 1;
                        }

                        ui.label(None, &fmt);

                        idx += 2;

                        for _ in 0..5 {
                            ui.separator();
                        }
                    }
                });

            let params = TextParams {
                font: Some(&self.font),
                font_size: (screen_height() + screen_height()) as u16 / 15,
                font_scale: 1.0,
                font_scale_aspect: 1.0,
                rotation: 0.0,
                color: WHITE,
            };

            let clock_text_y = (self.ui_window_pos().y + self.ui_window_size().y)
                + (screen_height() - self.ui_window_size().y) / 2.0;

            if widgets::Button::new("Leave")
                .position(vec2(
                    self.ui_window_pos().x + self.ui_window_size().x / 2.0 - 32.5,
                    clock_text_y + 50.0,
                ))
                .size(vec2(75.0, 30.0))
                .ui(&mut ui::root_ui())
            {
                self.state = State::GameSelection;
                *BREAK_THREAD_LOOP.lock().unwrap() = true;
                break;
            }

            let wtime_lock = WTIME_MTX.lock().unwrap();
            let wtime_dimensions = draw_text_ex(
                wtime_lock.to_clock().as_str(),
                self.ui_window_pos().x,
                clock_text_y,
                TextParams {
                    color: if post_run_info_cpy.position.side_to_move() == chess_color::WHITE {
                        if wtime_lock.as_secs() < 60 {
                            RED
                        } else {
                            WHITE
                        }
                    } else {
                        GRAY
                    },
                    ..params
                },
            );

            drop(wtime_lock);

            draw_rectangle(
                self.ui_window_pos().x,
                clock_text_y + 20.0,
                wtime_dimensions.width,
                10.0,
                WHITE,
            );

            let btime_lock = BTIME_MTX.lock().unwrap();
            let btime_dimensions = draw_text_ex(
                btime_lock.to_clock().as_str(),
                screen_width() - clock_text_width,
                clock_text_y,
                TextParams {
                    color: if post_run_info_cpy.position.side_to_move() == chess_color::BLACK {
                        if btime_lock.as_secs() < 60 {
                            RED
                        } else {
                            WHITE
                        }
                    } else {
                        GRAY
                    },
                    ..params
                },
            );

            drop(btime_lock);

            draw_rectangle(
                screen_width() - clock_text_width,
                clock_text_y + 20.0,
                btime_dimensions.width,
                10.0,
                BLACK,
            );

            let material_diff = post_run_info_cpy.position.material_diff();

            let x = if material_diff == 0 {
                120000.0 // just so the material text is out of sight if no side has the advantage
            } else if material_diff < 0 {
                screen_width() - clock_text_width
            } else {
                self.ui_window_pos().x
            };

            draw_text_ex(
                format!("+{}", material_diff.abs()).as_str(),
                x,
                clock_text_y + 90.0,
                TextParams {
                    font_size: (screen_height() + screen_height()) as u16 / 30,
                    ..params
                },
            );

            clock_text_width = btime_dimensions.width + 10.0;

            if let State::GameFinish { .. } = self.state {
                *BREAK_THREAD_LOOP.lock().unwrap() = true;
                break;
            }

            next_frame().await;
        }

        game_run_thread.join().unwrap();
    }

    async fn game_finish(&mut self) {
        let message = if let State::GameFinish { message } = &self.state {
            message.to_string()
        } else {
            unreachable!();
        };

        loop {
            clear_background(DARKGRAY);
            self.vb.draw_board(
                Some(&mut GAME.lock().unwrap().as_mut().unwrap().position),
                &self.lc_data,
            );
            let mut should_break = false;

            self.ui_window("").ui(&mut ui::root_ui(), |ui| {
                widgets::Label::new(&message)
                    .size(vec2(200.0, 100.0))
                    .position(self.ui_window_size() / 2.0 - vec2(100.0, 70.0))
                    .ui(ui);

                if widgets::Button::new("Play Again")
                    .size(vec2(100.0, 20.0))
                    .position(self.ui_window_size() / 2.0 - vec2(100.0, 5.0))
                    .ui(ui)
                {
                    self.state = State::TryStartGame {
                        timer: time::Duration::ZERO,
                        max_time: time::Duration::from_secs(5),
                    };

                    should_break = true;
                }

                if widgets::Button::new("Back")
                    .size(vec2(100.0, 20.0))
                    .position(self.ui_window_size() / 2.0 - vec2(-20.0, 5.0))
                    .ui(ui)
                {
                    self.state = State::GameSelection;
                    should_break = true;
                }
            });

            events::do_key_events(
                &mut self.vb,
                &GAME.lock().unwrap().as_mut().unwrap().position,
            );

            if should_break {
                break;
            }

            next_frame().await
        }
    }
}
