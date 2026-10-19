#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use minesweeper::anim::AnimationManager;
use minesweeper::audio::{sanitize_volume, select_reveal_sound, SoundEffect, SoundManager};
use minesweeper::game::{Board, Difficulty, FlagOutcome, GameState, RevealOutcome};
use minesweeper::storage::{PersistentData, ThemeMode};
use minesweeper::ui::{self, CustomDraft, UiAction};
use std::time::{Duration, Instant};

struct MinesweeperApp {
    board: Board,
    anim: AnimationManager,
    sound: SoundManager,
    data: PersistentData,
    custom_draft: CustomDraft,
    elapsed_secs: f64,
    last_tick: Instant,
    is_paused: bool,
    settings_open: bool,
    was_focused: bool,
    was_animating: bool,
}

impl MinesweeperApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::configure_fonts(&cc.egui_ctx);

        let data = PersistentData::load();
        let board = Board::new(data.difficulty);
        let anim = AnimationManager::new(
            board.width(),
            board.height(),
            board.remaining_mines(),
            data.theme,
            data.reduce_motion,
        );
        let sound = SoundManager::new(data.sound_enabled, data.sound_volume);
        let was_animating = anim.is_animating();
        let custom_draft =
            CustomDraft::new(data.custom_width, data.custom_height, data.custom_mines);

        Self {
            board,
            anim,
            sound,
            data,
            custom_draft,
            elapsed_secs: 0.0,
            last_tick: Instant::now(),
            is_paused: false,
            settings_open: false,
            was_focused: true,
            was_animating,
        }
    }

    fn restart_game(&mut self) {
        self.board.reset();
        self.elapsed_secs = 0.0;
        self.is_paused = false;
        self.anim.trigger_board_reset(
            self.board.width(),
            self.board.height(),
            self.board.remaining_mines(),
            self.data.reduce_motion,
        );
    }

    fn change_difficulty(&mut self, difficulty: Difficulty) {
        if let Difficulty::Custom {
            width,
            height,
            mines,
        } = difficulty
        {
            self.data.custom_width = width;
            self.data.custom_height = height;
            self.data.custom_mines = mines;
            self.custom_draft = CustomDraft::new(width, height, mines);
        }
        self.data.difficulty = difficulty;
        let _ = self.data.save();

        self.board = Board::new(difficulty);
        self.elapsed_secs = 0.0;
        self.is_paused = false;
        self.anim.trigger_board_reset(
            self.board.width(),
            self.board.height(),
            self.board.remaining_mines(),
            self.data.reduce_motion,
        );
    }

    fn handle_reveal_outcome(
        &mut self,
        click_origin: (usize, usize),
        outcome: RevealOutcome,
        is_chord: bool,
    ) {
        if !outcome.revealed.is_empty() {
            self.anim
                .notify_cells_revealed(&outcome.revealed, self.data.reduce_motion);
        }
        for &(fx, fy) in &outcome.auto_flagged {
            self.anim
                .notify_flag_changed(fx, fy, true, self.data.reduce_motion);
        }
        self.anim
            .mine_counter
            .set_value(self.board.remaining_mines(), self.data.reduce_motion);

        match outcome.state_changed_to {
            Some(GameState::Won) => {
                // Ensure even an instant first-click win has a positive finite duration
                self.elapsed_secs = self.elapsed_secs.max(0.05);
                self.data
                    .best_times
                    .record(self.board.difficulty(), self.elapsed_secs);
                let _ = self.data.save();
                self.anim.notify_win(self.data.reduce_motion);
                self.anim.mine_counter.set_value(0, self.data.reduce_motion);
                self.sound.play(SoundEffect::Win);
            }
            Some(GameState::Lost) => {
                let origin = outcome.hit_mines.first().copied().unwrap_or(click_origin);
                let mut all_mines = Vec::new();
                for y in 0..self.board.height() {
                    for x in 0..self.board.width() {
                        if let Some(cell) = self.board.cell(x, y) {
                            if cell.has_mine {
                                all_mines.push((x, y));
                            }
                        }
                    }
                }
                self.anim
                    .notify_loss(origin, &all_mines, self.data.reduce_motion);
                self.sound.play(SoundEffect::Lose);
            }
            _ => {
                if let Some(effect) = select_reveal_sound(&outcome.revealed, is_chord) {
                    self.sound.play(effect);
                }
            }
        }
    }

    fn process_action(&mut self, action: UiAction) {
        match action {
            UiAction::Reveal(x, y) => {
                let outcome = self.board.reveal(x, y);
                self.handle_reveal_outcome((x, y), outcome, false);
            }
            UiAction::ToggleFlag(x, y) => {
                let res = self.board.toggle_flag(x, y);
                match res {
                    FlagOutcome::Placed => {
                        self.anim
                            .notify_flag_changed(x, y, true, self.data.reduce_motion);
                        self.anim
                            .mine_counter
                            .set_value(self.board.remaining_mines(), self.data.reduce_motion);
                        self.sound.play(SoundEffect::FlagPlace);
                    }
                    FlagOutcome::Removed => {
                        self.anim
                            .notify_flag_changed(x, y, false, self.data.reduce_motion);
                        self.anim
                            .mine_counter
                            .set_value(self.board.remaining_mines(), self.data.reduce_motion);
                        self.sound.play(SoundEffect::FlagRemove);
                    }
                    FlagOutcome::Ignored => {}
                }
            }
            UiAction::Chord(x, y) => {
                let outcome = self.board.chord(x, y);
                self.handle_reveal_outcome((x, y), outcome, true);
            }
            UiAction::Restart => {
                self.restart_game();
                self.sound.play(SoundEffect::UiClick);
            }
            UiAction::TogglePause => {
                if self.board.state() == GameState::Playing {
                    self.is_paused = !self.is_paused;
                    self.sound.play(SoundEffect::UiClick);
                }
            }
            UiAction::ResumeFromPause => {
                self.is_paused = false;
                self.sound.play(SoundEffect::UiClick);
            }
            UiAction::ToggleSettings => {
                self.settings_open = !self.settings_open;
                self.anim
                    .set_settings_open(self.settings_open, self.data.reduce_motion);
                self.sound.play(SoundEffect::UiClick);
            }
            UiAction::ToggleSound => {
                let enabled = !self.data.sound_enabled;
                self.data.sound_enabled = enabled;
                self.sound.set_enabled(enabled);
                if enabled {
                    self.sound.play(SoundEffect::UiClick);
                }
                let _ = self.data.save();
            }
            UiAction::SelectDifficulty(diff) => {
                let was_custom = matches!(self.board.difficulty(), Difficulty::Custom { .. });
                self.change_difficulty(diff);
                if matches!(diff, Difficulty::Custom { .. }) && !was_custom && !self.settings_open {
                    self.settings_open = true;
                    self.anim.set_settings_open(true, self.data.reduce_motion);
                }
                self.sound.play(SoundEffect::UiClick);
            }
            UiAction::SetTheme(mode) => {
                self.data.theme = mode;
                self.anim.set_theme(mode, self.data.reduce_motion);
                self.sound.play(SoundEffect::UiClick);
                let _ = self.data.save();
            }
            UiAction::SetReduceMotion(rm) => {
                self.data.reduce_motion = rm;
                self.anim.set_reduce_motion(rm);
                self.sound.play(SoundEffect::UiClick);
                let _ = self.data.save();
            }
            UiAction::SetSoundEnabled(enabled) => {
                self.data.sound_enabled = enabled;
                self.sound.set_enabled(enabled);
                if enabled {
                    self.sound.play(SoundEffect::UiClick);
                }
                let _ = self.data.save();
            }
            UiAction::SetSoundVolume(vol) => {
                let v = sanitize_volume(vol);
                self.data.sound_volume = v;
                self.sound.set_volume(v);
                if v > 0.001 && !self.data.sound_enabled {
                    self.data.sound_enabled = true;
                    self.sound.set_enabled(true);
                }
                let _ = self.data.save();
            }
        }
    }
}

impl eframe::App for MinesweeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let raw_dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // When waking from an idle state, clamp animation step to one 60fps frame so hover
        // transitions start smoothly from 0 instead of jumping by the idle sleep duration.
        let max_anim_dt = if self.was_animating { 0.1 } else { 1.0 / 60.0 };
        let dt = (raw_dt as f32).clamp(0.0, max_anim_dt);

        // Detect window focus loss and automatically pause when playing
        let is_focused = ctx.input(|i| i.focused);
        if self.was_focused && !is_focused && self.board.state() == GameState::Playing {
            self.is_paused = true;
        }
        self.was_focused = is_focused;

        // Update active game timer using real wall-clock elapsed time
        if self.board.state() == GameState::Playing && !self.is_paused {
            self.elapsed_secs += raw_dt.clamp(0.0, 5.0);
            self.anim
                .timer_display
                .set_value(self.elapsed_secs.floor() as isize, self.data.reduce_motion);
        }

        // Advance animations
        self.anim.tick(dt);

        // Keyboard shortcuts (N or F2 for restart, P for pause, M for sound toggle) when not editing text inputs
        let mut actions = Vec::new();
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::N) || i.key_pressed(egui::Key::F2) {
                    actions.push(UiAction::Restart);
                }
                if i.key_pressed(egui::Key::P) {
                    actions.push(UiAction::TogglePause);
                }
                if i.key_pressed(egui::Key::M) {
                    actions.push(UiAction::ToggleSound);
                }
            });
        }

        let palette = self.anim.current_palette();

        // Sync egui visuals with current interpolated theme
        let mut visuals = if self.data.theme == ThemeMode::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = palette.bg_app;
        visuals.window_fill = palette.bg_panel;
        visuals.override_text_color = Some(palette.text_primary);
        ctx.set_visuals(visuals);

        let panel_frame = egui::Frame::new()
            .fill(palette.bg_app)
            .inner_margin(egui::Margin::symmetric(16, 14));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                ui::draw_top_bar(
                    ui,
                    &self.board,
                    &self.anim,
                    &self.data.best_times,
                    self.is_paused,
                    self.settings_open,
                    self.data.sound_enabled,
                    &palette,
                    &mut actions,
                );

                ui.add_space(6.0);

                ui::draw_difficulty_bar(
                    ui,
                    &self.board,
                    (
                        self.data.custom_width,
                        self.data.custom_height,
                        self.data.custom_mines,
                    ),
                    &palette,
                    &mut actions,
                );

                if self.anim.settings_t > 0.001 {
                    ui.add_space(6.0);
                    ui::draw_settings_panel(
                        ui,
                        &self.board,
                        &self.anim,
                        self.data.theme,
                        self.data.reduce_motion,
                        self.data.sound_enabled,
                        self.data.sound_volume,
                        &self.data.best_times,
                        &mut self.custom_draft,
                        &palette,
                        &mut actions,
                    );
                }

                ui.add_space(4.0);

                ui::draw_board(
                    ui,
                    &self.board,
                    &mut self.anim,
                    self.elapsed_secs,
                    &self.data.best_times,
                    self.is_paused,
                    self.data.reduce_motion,
                    dt,
                    &palette,
                    &mut actions,
                );
            });

        for act in actions {
            self.process_action(act);
        }

        // Request repaint ONLY when animations are active (60fps) or when the next integer second is reached
        let animating = self.anim.is_animating();
        self.was_animating = animating;
        if animating {
            ctx.request_repaint();
        } else if self.board.state() == GameState::Playing && !self.is_paused {
            let next_sec_rem = (1.0 - self.elapsed_secs.fract()).clamp(0.02, 1.0);
            ctx.request_repaint_after(Duration::from_secs_f64(next_sec_rem));
        }
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Minesweeper")
            .with_inner_size([680.0, 740.0])
            .with_min_inner_size([400.0, 460.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Minesweeper",
        native_options,
        Box::new(|cc| Ok(Box::new(MinesweeperApp::new(cc)))),
    )
}
