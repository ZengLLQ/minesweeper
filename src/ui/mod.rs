pub mod icons;
pub mod theme;

use crate::anim::{
    ease_in_out_cubic, ease_out_back, ease_out_cubic, flag_drop_bounce, AnimationManager,
    RollingDisplay,
};
use crate::game::{
    Board, CellState, Difficulty, GameState, MAX_CUSTOM_HEIGHT, MAX_CUSTOM_WIDTH, MIN_CUSTOM_DIM,
};
use crate::storage::{BestTimes, ThemeMode};
use crate::ui::theme::{lerp_color, with_alpha, ThemePalette};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2,
};

/// Actions emitted by the UI to be processed by the application controller.
#[derive(Clone, Debug)]
pub enum UiAction {
    Reveal(usize, usize),
    ToggleFlag(usize, usize),
    Chord(usize, usize),
    Restart,
    TogglePause,
    ResumeFromPause,
    ToggleSettings,
    ToggleSound,
    SelectDifficulty(Difficulty),
    SetTheme(ThemeMode),
    SetReduceMotion(bool),
    SetSoundEnabled(bool),
    SetSoundVolume(f32),
}

/// Mutable draft state for the Custom difficulty form in the settings panel.
#[derive(Clone, Debug)]
pub struct CustomDraft {
    pub width_str: String,
    pub height_str: String,
    pub mines_str: String,
    pub error_msg: Option<String>,
}

impl CustomDraft {
    pub fn new(width: usize, height: usize, mines: usize) -> Self {
        Self {
            width_str: width.to_string(),
            height_str: height.to_string(),
            mines_str: mines.to_string(),
            error_msg: None,
        }
    }
}

/// Loads the embedded Inter fonts (`Inter-Regular.ttf` and `Inter-Bold.ttf`) into egui.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "Inter-Regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-Bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-Bold.ttf"
        ))),
    );

    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "Inter-Regular".to_owned());
    }
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "Inter-Bold".to_owned());
    }
    fonts.families.insert(
        FontFamily::Name("InterBold".into()),
        vec!["Inter-Bold".to_owned(), "Inter-Regular".to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// Formats seconds into `MM:SS.d` or `SS.ds` for best times and victory summary.
pub fn format_duration(secs: f64) -> String {
    let total = secs.max(0.0);
    let mins = (total / 60.0).floor() as u32;
    let rem = total - (mins as f64 * 60.0);
    if mins > 0 {
        format!("{mins}:{rem:04.1}s")
    } else {
        format!("{rem:.1}s")
    }
}

/// Draws a rolling 4-slot numeric counter with vertical roll + fade transition per character.
fn draw_rolling_number(
    painter: &egui::Painter,
    left_center: Pos2,
    display: &RollingDisplay,
    font_size: f32,
    color: Color32,
) {
    let char_advance = font_size * 0.62;
    let roll_dist = font_size * 0.58;
    let font_id = FontId::new(font_size, FontFamily::Name("InterBold".into()));

    for (i, slot) in display.slots.iter().enumerate() {
        let slot_center = Pos2::new(
            left_center.x + (i as f32 + 0.5) * char_advance,
            left_center.y,
        );
        let t = ease_out_cubic(slot.t);

        if t < 0.999 && slot.previous != slot.current {
            // Outgoing digit rolls and fades out
            if slot.previous != ' ' {
                let prev_y = slot_center.y - slot.direction * t * roll_dist;
                let prev_col = with_alpha(color, (1.0 - t).powi(2));
                painter.text(
                    Pos2::new(slot_center.x, prev_y),
                    Align2::CENTER_CENTER,
                    slot.previous.to_string(),
                    font_id.clone(),
                    prev_col,
                );
            }
            // Incoming digit rolls and fades in
            if slot.current != ' ' {
                let curr_y = slot_center.y + slot.direction * (1.0 - t) * roll_dist;
                let curr_col = with_alpha(color, t);
                painter.text(
                    Pos2::new(slot_center.x, curr_y),
                    Align2::CENTER_CENTER,
                    slot.current.to_string(),
                    font_id.clone(),
                    curr_col,
                );
            }
        } else if slot.current != ' ' {
            painter.text(
                slot_center,
                Align2::CENTER_CENTER,
                slot.current.to_string(),
                font_id.clone(),
                color,
            );
        }
    }
}

/// Renders the minimalist top bar (mine counter on left, pause/restart/sound/settings in center, timer on right).
#[allow(clippy::too_many_arguments)]
pub fn draw_top_bar(
    ui: &mut Ui,
    board: &Board,
    anim: &AnimationManager,
    best_times: &BestTimes,
    is_paused: bool,
    settings_open: bool,
    sound_enabled: bool,
    palette: &ThemePalette,
    actions: &mut Vec<UiAction>,
) {
    let available_w = ui.available_width();
    let bar_height = 54.0;
    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(available_w, bar_height), Sense::hover());
    let painter = ui.painter_at(bar_rect);

    painter.rect_filled(bar_rect, CornerRadius::same(12), palette.bg_top_bar);
    painter.rect_stroke(
        bar_rect,
        CornerRadius::same(12),
        Stroke::new(1.0_f32, palette.cell_border_subtle),
        StrokeKind::Inside,
    );

    // Left: Mine icon + rolling mine counter
    let mine_icon_pos = Pos2::new(bar_rect.left() + 24.0, bar_rect.center().y);
    icons::draw_mine(&painter, mine_icon_pos, 22.0, palette.accent, 1.0, 1.0);
    draw_rolling_number(
        &painter,
        Pos2::new(bar_rect.left() + 36.0, bar_rect.center().y),
        &anim.mine_counter,
        20.0,
        palette.text_primary,
    );

    // Optional Best Time badge next to mine counter when window is wide enough not to collide with center buttons
    if available_w >= 520.0 {
        if let Some(best) = best_times.get(board.difficulty()) {
            let trophy_x = bar_rect.left() + 112.0;
            icons::draw_trophy_icon(
                &painter,
                Pos2::new(trophy_x, bar_rect.center().y),
                16.0,
                palette.text_secondary,
            );
            painter.text(
                Pos2::new(trophy_x + 13.0, bar_rect.center().y),
                Align2::LEFT_CENTER,
                format_duration(best),
                FontId::new(13.0, FontFamily::Proportional),
                palette.text_secondary,
            );
        }
    }

    // Center controls: Pause, Restart, Sound/Mute, Settings
    let center = bar_rect.center();
    let btn_size = Vec2::splat(32.0);
    let pause_rect = Rect::from_center_size(center + Vec2::new(-60.0, 0.0), btn_size);
    let restart_rect = Rect::from_center_size(center + Vec2::new(-19.0, 0.0), Vec2::splat(36.0));
    let sound_rect = Rect::from_center_size(center + Vec2::new(22.0, 0.0), btn_size);
    let settings_rect = Rect::from_center_size(center + Vec2::new(60.0, 0.0), btn_size);

    // Pause button
    let pause_resp = ui
        .interact(pause_rect, ui.id().with("top_pause_btn"), Sense::click())
        .on_hover_text(if is_paused { "Resume (P)" } else { "Pause (P)" });
    if pause_resp.hovered() || is_paused {
        painter.rect_filled(
            pause_rect,
            CornerRadius::same(8),
            if is_paused {
                palette.accent_soft
            } else {
                palette.cell_hidden_hover
            },
        );
    }
    icons::draw_pause_or_play_icon(
        &painter,
        pause_rect.center(),
        18.0,
        if is_paused {
            palette.accent
        } else {
            palette.text_secondary
        },
        is_paused,
    );
    if pause_resp.clicked() {
        actions.push(UiAction::TogglePause);
    }

    // Restart button
    let restart_resp = ui
        .interact(
            restart_rect,
            ui.id().with("top_restart_btn"),
            Sense::click(),
        )
        .on_hover_text("Restart Game (N / F2)");
    painter.rect_filled(
        restart_rect,
        CornerRadius::same(10),
        if restart_resp.hovered() {
            palette.accent_hover
        } else {
            palette.accent
        },
    );
    icons::draw_restart_icon(&painter, restart_rect.center(), 18.0, Color32::WHITE);
    if restart_resp.clicked() {
        actions.push(UiAction::Restart);
    }

    // Sound / Mute toggle button
    let sound_resp = ui
        .interact(sound_rect, ui.id().with("top_sound_btn"), Sense::click())
        .on_hover_text(if sound_enabled {
            "Mute Sound Effects (M)"
        } else {
            "Unmute Sound Effects (M)"
        });
    if sound_resp.hovered() || !sound_enabled {
        painter.rect_filled(
            sound_rect,
            CornerRadius::same(8),
            if !sound_enabled {
                palette.accent_soft
            } else {
                palette.cell_hidden_hover
            },
        );
    }
    icons::draw_speaker_icon(
        &painter,
        sound_rect.center(),
        18.0,
        if sound_enabled {
            palette.text_secondary
        } else {
            palette.text_muted
        },
        sound_enabled,
    );
    if sound_resp.clicked() {
        actions.push(UiAction::ToggleSound);
    }

    // Settings button
    let settings_resp = ui
        .interact(
            settings_rect,
            ui.id().with("top_settings_btn"),
            Sense::click(),
        )
        .on_hover_text("Settings & Difficulty");
    if settings_resp.hovered() || settings_open {
        painter.rect_filled(
            settings_rect,
            CornerRadius::same(8),
            if settings_open {
                palette.accent_soft
            } else {
                palette.cell_hidden_hover
            },
        );
    }
    icons::draw_settings_icon(
        &painter,
        settings_rect.center(),
        18.0,
        if settings_open {
            palette.accent
        } else {
            palette.text_secondary
        },
    );
    if settings_resp.clicked() {
        actions.push(UiAction::ToggleSettings);
    }

    // Right: Clock icon + rolling timer display
    let number_width = 4.0 * (20.0 * 0.62);
    let number_left = bar_rect.right() - 16.0 - number_width;
    let clock_pos = Pos2::new(number_left - 12.0, bar_rect.center().y);
    icons::draw_clock_icon(&painter, clock_pos, 18.0, palette.accent);
    draw_rolling_number(
        &painter,
        Pos2::new(number_left, bar_rect.center().y),
        &anim.timer_display,
        20.0,
        palette.text_primary,
    );
}

/// Renders the quick difficulty selector bar and game state indicator below the top bar.
pub fn draw_difficulty_bar(
    ui: &mut Ui,
    board: &Board,
    custom_dims: (usize, usize, usize),
    palette: &ThemePalette,
    actions: &mut Vec<UiAction>,
) {
    let avail_w = ui.available_width();
    let bar_h = 32.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, bar_h), Sense::hover());
    let painter = ui.painter_at(rect);

    let presets = [
        ("Beginner", Difficulty::Beginner),
        ("Intermediate", Difficulty::Intermediate),
        ("Expert", Difficulty::Expert),
        (
            "Custom",
            Difficulty::Custom {
                width: custom_dims.0,
                height: custom_dims.1,
                mines: custom_dims.2,
            },
        ),
    ];

    let mut x_cursor = rect.left() + 4.0;
    for (label, diff) in presets {
        let is_active =
            std::mem::discriminant(&board.difficulty()) == std::mem::discriminant(&diff);
        let pill_w = (label.len() as f32 * 7.2 + 20.0).max(68.0);
        let pill_rect = Rect::from_min_size(
            Pos2::new(x_cursor, rect.center().y - 13.0),
            Vec2::new(pill_w, 26.0),
        );
        let resp = ui.interact(
            pill_rect,
            ui.id().with(("diff_pill", label)),
            Sense::click(),
        );

        let bg = if is_active {
            palette.accent_soft
        } else if resp.hovered() {
            palette.cell_hidden_hover
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(pill_rect, CornerRadius::same(7), bg);
        if is_active {
            painter.rect_stroke(
                pill_rect,
                CornerRadius::same(7),
                Stroke::new(1.0_f32, with_alpha(palette.accent, 0.55)),
                StrokeKind::Inside,
            );
        }

        painter.text(
            pill_rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::new(
                12.5,
                if is_active {
                    FontFamily::Name("InterBold".into())
                } else {
                    FontFamily::Proportional
                },
            ),
            if is_active {
                palette.accent
            } else {
                palette.text_secondary
            },
        );

        if resp.clicked() {
            actions.push(UiAction::SelectDifficulty(diff));
        }
        x_cursor += pill_w + 6.0;
    }

    // Right side: board dimensions summary when wide enough not to collide with the Custom pill
    if avail_w >= 468.0 {
        let info = format!(
            "{}×{} · {} mines",
            board.width(),
            board.height(),
            board.total_mines()
        );
        painter.text(
            Pos2::new(rect.right() - 6.0, rect.center().y),
            Align2::RIGHT_CENTER,
            info,
            FontId::new(12.0, FontFamily::Proportional),
            palette.text_muted,
        );
    }
}

/// Renders the animated settings slide/fade card when `anim.settings_t > 0.001`.
#[allow(clippy::too_many_arguments)]
pub fn draw_settings_panel(
    ui: &mut Ui,
    board: &Board,
    anim: &AnimationManager,
    theme_mode: ThemeMode,
    reduce_motion: bool,
    sound_enabled: bool,
    sound_volume: f32,
    best_times: &BestTimes,
    draft: &mut CustomDraft,
    palette: &ThemePalette,
    actions: &mut Vec<UiAction>,
) {
    let t = ease_in_out_cubic(anim.settings_t);
    if t <= 0.001 {
        return;
    }

    let panel_full_h = 114.0;
    let alloc_h = panel_full_h * t;
    let avail_w = ui.available_width();
    let (outer_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, alloc_h), Sense::hover());

    let slide_offset = (1.0 - t) * -12.0;
    let card_rect = Rect::from_min_size(
        Pos2::new(outer_rect.left(), outer_rect.top() + slide_offset),
        Vec2::new(avail_w, panel_full_h - 4.0),
    );

    let clip_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(card_rect.intersect(outer_rect))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let painter = clip_ui.painter();

    painter.rect_filled(
        card_rect,
        CornerRadius::same(12),
        with_alpha(palette.bg_panel, t),
    );
    painter.rect_stroke(
        card_rect,
        CornerRadius::same(12),
        Stroke::new(1.0_f32, with_alpha(palette.cell_border_subtle, t)),
        StrokeKind::Inside,
    );

    let inner = card_rect.shrink2(Vec2::new(12.0, 10.0));
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner.intersect(outer_rect))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child_ui.set_opacity(t);

    let compact = inner.width() < 540.0;
    let ultra_compact = inner.width() < 430.0;

    child_ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(if ultra_compact {
                "Theme:"
            } else if compact {
                "Prefs:"
            } else {
                "Preferences:"
            })
            .font(FontId::new(12.5, FontFamily::Name("InterBold".into())))
            .color(palette.text_primary),
        );
        ui.add_space(if compact { 2.0 } else { 8.0 });

        let is_dark = theme_mode == ThemeMode::Dark;
        if ui.selectable_label(is_dark, "Dark").clicked() && !is_dark {
            actions.push(UiAction::SetTheme(ThemeMode::Dark));
        }
        if ui.selectable_label(!is_dark, "Light").clicked() && is_dark {
            actions.push(UiAction::SetTheme(ThemeMode::Light));
        }

        ui.add_space(if compact { 3.0 } else { 10.0 });
        let mut rm = reduce_motion;
        let rm_label = if compact { "Motion" } else { "Reduce motion" };
        if ui
            .checkbox(&mut rm, rm_label)
            .on_hover_text("Reduce motion and particle animations")
            .changed()
        {
            actions.push(UiAction::SetReduceMotion(rm));
        }

        ui.add_space(if compact { 3.0 } else { 10.0 });
        let mut sfx = sound_enabled;
        let sfx_label = if compact { "SFX" } else { "Sound (M)" };
        if ui.checkbox(&mut sfx, sfx_label).changed() {
            actions.push(UiAction::SetSoundEnabled(sfx));
        }

        let mut vol_pct = (sound_volume.clamp(0.0, 1.0) * 100.0).round() as i32;
        ui.spacing_mut().slider_width = if ultra_compact {
            42.0
        } else if compact {
            56.0
        } else {
            68.0
        };
        if ui
            .add(
                egui::Slider::new(&mut vol_pct, 0..=100)
                    .suffix("%")
                    .show_value(!ultra_compact),
            )
            .on_hover_text(format!("Sound effects volume ({vol_pct}%)"))
            .changed()
        {
            actions.push(UiAction::SetSoundVolume(vol_pct as f32 / 100.0));
        }
    });

    child_ui.add_space(6.0);

    // Custom dimensions row
    let compact_custom = inner.width() < 530.0;
    child_ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(if compact_custom {
                "Custom:"
            } else {
                "Custom Board:"
            })
            .font(FontId::new(12.5, FontFamily::Name("InterBold".into())))
            .color(palette.text_primary),
        );

        let w_label = if compact_custom {
            "W:".to_owned()
        } else {
            format!("W ({MIN_CUSTOM_DIM}-{MAX_CUSTOM_WIDTH}):")
        };
        ui.label(
            egui::RichText::new(w_label)
                .size(12.0)
                .color(palette.text_secondary),
        )
        .on_hover_text(format!("Width ({MIN_CUSTOM_DIM}..={MAX_CUSTOM_WIDTH})"));
        let w_resp = ui.add(
            egui::TextEdit::singleline(&mut draft.width_str).desired_width(if compact_custom {
                30.0
            } else {
                36.0
            }),
        );

        let h_label = if compact_custom {
            "H:".to_owned()
        } else {
            format!("H ({MIN_CUSTOM_DIM}-{MAX_CUSTOM_HEIGHT}):")
        };
        ui.label(
            egui::RichText::new(h_label)
                .size(12.0)
                .color(palette.text_secondary),
        )
        .on_hover_text(format!("Height ({MIN_CUSTOM_DIM}..={MAX_CUSTOM_HEIGHT})"));
        let h_resp =
            ui.add(
                egui::TextEdit::singleline(&mut draft.height_str)
                    .desired_width(if compact_custom { 30.0 } else { 36.0 }),
            );

        ui.label(
            egui::RichText::new(if compact_custom { "M:" } else { "Mines:" })
                .size(12.0)
                .color(palette.text_secondary),
        )
        .on_hover_text("Number of mines");
        let m_resp = ui.add(
            egui::TextEdit::singleline(&mut draft.mines_str).desired_width(if compact_custom {
                36.0
            } else {
                42.0
            }),
        );

        let enter_pressed = (w_resp.lost_focus() || h_resp.lost_focus() || m_resp.lost_focus())
            && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let btn_label = if compact_custom {
            "Apply"
        } else {
            "Apply Custom"
        };

        if ui.button(btn_label).clicked() || enter_pressed {
            let parsed_w = draft.width_str.trim().parse::<usize>();
            let parsed_h = draft.height_str.trim().parse::<usize>();
            let parsed_m = draft.mines_str.trim().parse::<usize>();

            match (parsed_w, parsed_h, parsed_m) {
                (Ok(w), Ok(h), Ok(m)) => match Difficulty::validate_custom(w, h, m) {
                    Ok(custom_diff) => {
                        draft.error_msg = None;
                        actions.push(UiAction::SelectDifficulty(custom_diff));
                    }
                    Err(err) => {
                        draft.error_msg = Some(err.to_string());
                    }
                },
                _ => {
                    draft.error_msg =
                        Some("Enter positive integers for width, height, and mines".to_owned());
                }
            }
        }
    });

    child_ui.add_space(5.0);

    if let Some(err) = &draft.error_msg {
        child_ui.label(egui::RichText::new(err).size(11.5).color(palette.danger));
    } else {
        // Best times row
        let b_str = best_times
            .beginner
            .map(format_duration)
            .unwrap_or_else(|| "—".to_owned());
        let i_str = best_times
            .intermediate
            .map(format_duration)
            .unwrap_or_else(|| "—".to_owned());
        let e_str = best_times
            .expert
            .map(format_duration)
            .unwrap_or_else(|| "—".to_owned());

        child_ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if compact { "Best:" } else { "Best Times:" })
                    .font(FontId::new(12.0, FontFamily::Name("InterBold".into())))
                    .color(palette.text_secondary),
            );
            let summary = if compact {
                format!("B: {b_str}  ·  I: {i_str}  ·  E: {e_str}")
            } else {
                format!("Beginner: {b_str}   ·   Intermediate: {i_str}   ·   Expert: {e_str}")
            };
            ui.label(
                egui::RichText::new(summary)
                    .size(12.0)
                    .color(palette.text_muted),
            );
            let _ = board.width();
        });
    }
}

/// Renders the Minesweeper board with full 60fps delta-time animations, vector icons,
/// shockwaves, particles, and win/loss/pause overlays.
#[allow(clippy::too_many_arguments)]
pub fn draw_board(
    ui: &mut Ui,
    board: &Board,
    anim: &mut AnimationManager,
    elapsed_secs: f64,
    best_times: &BestTimes,
    is_paused: bool,
    reduce_motion: bool,
    dt: f32,
    palette: &ThemePalette,
    actions: &mut Vec<UiAction>,
) {
    let avail = ui.available_size();
    let cols = board.width() as f32;
    let rows = board.height() as f32;

    // Reserve space at bottom for win/loss summary pill when game is finished
    let bottom_banner_space = if matches!(board.state(), GameState::Won | GameState::Lost) {
        54.0
    } else {
        6.0
    };

    let padding_x = 8.0;
    let padding_y = 6.0;
    let usable_w = (avail.x - padding_x * 2.0).max(40.0);
    let usable_h = (avail.y - padding_y * 2.0 - bottom_banner_space).max(40.0);

    // DPI-aware proportional cell scaling that always fits inside usable_w x usable_h
    // even for a 50x35 custom board on the minimum 400x460 window.
    let cell_pitch = (usable_w / cols).min(usable_h / rows).clamp(4.0, 50.0);
    let gap = (cell_pitch * 0.095).clamp(1.0, 4.5).min(cell_pitch * 0.22);
    let cell_size = (cell_pitch - gap).max(2.0);
    let corner_px = (cell_size * 0.20).clamp(1.5, 8.0) as u8;

    let total_w = cols * cell_pitch - gap;
    let total_h = rows * cell_pitch - gap;

    let (full_rect, _) = ui.allocate_exact_size(avail, Sense::hover());
    let shake = if reduce_motion {
        Vec2::ZERO
    } else {
        anim.board_shake_offset()
    };

    let board_center_y = full_rect.top() + (avail.y - bottom_banner_space) * 0.5;
    let start_x = full_rect.center().x - total_w * 0.5 + shake.x;
    let start_y = board_center_y - total_h * 0.5 + shake.y;
    let board_rect = Rect::from_min_size(Pos2::new(start_x, start_y), Vec2::new(total_w, total_h));

    let painter = ui.painter_at(full_rect);
    let paused_active = is_paused && board.state() == GameState::Playing;

    for y in 0..board.height() {
        for x in 0..board.width() {
            let Some(cell) = board.cell(x, y).copied() else {
                continue;
            };

            let base_x = start_x + x as f32 * cell_pitch;
            let base_y = start_y + y as f32 * cell_pitch;
            let hit_rect = Rect::from_min_size(Pos2::new(base_x, base_y), Vec2::splat(cell_size));

            let cell_id = ui.id().with(("ms_cell", x, y));
            let resp = ui.interact(hit_rect, cell_id, Sense::click());

            let can_interact =
                !is_paused && matches!(board.state(), GameState::Ready | GameState::Playing);
            let is_hovered = resp.hovered() && can_interact && cell.state != CellState::Revealed;
            anim.update_cell_hover(x, y, is_hovered, dt, reduce_motion);

            if can_interact {
                if resp.clicked() {
                    if cell.state == CellState::Revealed && cell.adjacent_mines > 0 {
                        actions.push(UiAction::Chord(x, y));
                    } else if cell.state == CellState::Hidden {
                        actions.push(UiAction::Reveal(x, y));
                    }
                } else if resp.secondary_clicked() {
                    actions.push(UiAction::ToggleFlag(x, y));
                } else if resp.middle_clicked() {
                    actions.push(UiAction::Chord(x, y));
                }
            }

            let cell_anim = anim.cell(x, y).copied().unwrap_or_default();

            // Board reset radial stagger fade-in
            if cell_anim.reset_delay > 0.0 {
                continue;
            }
            let reset_alpha = ease_out_cubic(cell_anim.reset_t);
            let reset_scale = if cell_anim.reset_t < 1.0 {
                0.76 + 0.24 * ease_out_back(cell_anim.reset_t)
            } else {
                1.0
            };

            // Conceal all cell states uniformly while paused
            if paused_active {
                let draw_rect =
                    Rect::from_center_size(hit_rect.center(), Vec2::splat(cell_size * reset_scale));
                painter.rect_filled(
                    draw_rect,
                    CornerRadius::same(corner_px),
                    with_alpha(palette.cell_hidden, reset_alpha),
                );
                continue;
            }

            // Hover lift and brightness interpolation
            let hover_ease = ease_out_cubic(cell_anim.hover_t);
            let hover_lift = (-2.2 * hover_ease).max(-cell_size * 0.18);

            // Flood-fill wave delay check (or secondary exploded mine pop delay check)
            let is_exploded = board.is_exploded_mine(x, y);
            let waiting_for_reveal_wave = cell.state == CellState::Revealed
                && (cell_anim.reveal_delay > 0.0
                    || (is_exploded && cell_anim.mine_pop_delay > 0.0));
            let reveal_progress = if waiting_for_reveal_wave {
                0.0
            } else {
                ease_out_cubic(cell_anim.reveal_t)
            };

            let show_lost_mine = board.state() == GameState::Lost
                && cell.has_mine
                && cell.state != CellState::Flagged
                && cell_anim.mine_pop_delay <= 0.0;
            let wrong_flag = board.is_wrong_flag(x, y);

            // Compute animated cell rectangle
            let scaled_size = cell_size * reset_scale;
            let draw_center = hit_rect.center()
                + Vec2::new(
                    0.0,
                    if cell.state == CellState::Revealed && !waiting_for_reveal_wave {
                        0.0
                    } else {
                        hover_lift
                    },
                );
            let draw_rect = Rect::from_center_size(draw_center, Vec2::splat(scaled_size));
            let corner = CornerRadius::same(corner_px);

            // Soft drop shadow beneath unrevealed/covered cells
            let cover_alpha = if cell.state == CellState::Revealed {
                1.0 - reveal_progress
            } else {
                1.0
            };
            if cover_alpha > 0.05 && !show_lost_mine && cell_size >= 10.0 {
                let shadow_offset = Vec2::new(0.0, (2.0 + hover_ease * 1.4).min(cell_size * 0.16));
                let shadow_col = with_alpha(palette.cell_shadow, reset_alpha * cover_alpha);
                painter.rect_filled(draw_rect.translate(shadow_offset), corner, shadow_col);
            }

            // Base cell surface
            if cell.state == CellState::Revealed && !waiting_for_reveal_wave {
                let rev_scale = 0.84 + 0.16 * ease_out_back(cell_anim.reveal_t);
                let rev_rect =
                    Rect::from_center_size(draw_center, Vec2::splat(scaled_size * rev_scale));
                let base_col = if is_exploded {
                    palette.cell_exploded
                } else {
                    palette.cell_revealed
                };
                painter.rect_filled(rev_rect, corner, with_alpha(base_col, reset_alpha));

                // Cross-fade the hidden cover during the first moments of reveal
                if cover_alpha > 0.02 && !is_exploded {
                    painter.rect_filled(
                        draw_rect,
                        corner,
                        with_alpha(palette.cell_hidden, reset_alpha * cover_alpha),
                    );
                }
            } else {
                let hidden_col =
                    lerp_color(palette.cell_hidden, palette.cell_hidden_hover, hover_ease);
                let bg = if wrong_flag {
                    palette.cell_wrong_flag
                } else if show_lost_mine {
                    lerp_color(
                        hidden_col,
                        palette.cell_revealed,
                        ease_out_cubic(cell_anim.mine_pop_t),
                    )
                } else {
                    hidden_col
                };
                painter.rect_filled(draw_rect, corner, with_alpha(bg, reset_alpha));
                painter.rect_stroke(
                    draw_rect,
                    corner,
                    Stroke::new(1.0_f32, with_alpha(palette.cell_border_subtle, reset_alpha)),
                    StrokeKind::Inside,
                );
            }

            // Draw cell foreground icons or numbers
            if wrong_flag {
                icons::draw_flag(
                    &painter,
                    draw_center,
                    scaled_size,
                    palette.flag_pole,
                    palette.flag_color,
                    1.0,
                    0.0,
                    reset_alpha * 0.65,
                );
                icons::draw_wrong_flag_cross(
                    &painter,
                    draw_center,
                    scaled_size,
                    palette.danger,
                    reset_alpha,
                );
            } else if cell.state == CellState::Flagged {
                let (y_off_factor, flag_scale) = flag_drop_bounce(cell_anim.flag_in_t);
                icons::draw_flag(
                    &painter,
                    draw_center,
                    scaled_size,
                    palette.flag_pole,
                    palette.flag_color,
                    flag_scale,
                    y_off_factor * scaled_size,
                    reset_alpha,
                );
            } else if cell.state == CellState::Hidden && cell_anim.flag_out_t > 0.01 {
                // Flag fade-out animation when unflagged
                let fade = ease_out_cubic(cell_anim.flag_out_t);
                icons::draw_flag(
                    &painter,
                    draw_center,
                    scaled_size,
                    palette.flag_pole,
                    palette.flag_color,
                    0.72 + 0.28 * fade,
                    (1.0 - fade) * -scaled_size * 0.12,
                    reset_alpha * fade,
                );
            } else if show_lost_mine {
                let pop_scale = ease_out_back(cell_anim.mine_pop_t).max(0.05);
                let pop_alpha = ease_out_cubic(cell_anim.mine_pop_t) * reset_alpha;
                let mine_col = if is_exploded {
                    palette.danger
                } else {
                    palette.mine_color
                };
                icons::draw_mine(
                    &painter,
                    draw_center,
                    scaled_size,
                    mine_col,
                    pop_scale,
                    pop_alpha,
                );
            } else if cell.state == CellState::Revealed
                && !waiting_for_reveal_wave
                && cell.adjacent_mines > 0
            {
                let num_scale = (0.68 + 0.32 * ease_out_back(cell_anim.reveal_t)).clamp(0.4, 1.15);
                let num_alpha = reveal_progress * reset_alpha;
                let num_col = with_alpha(palette.number_color(cell.adjacent_mines), num_alpha);
                let font_px = (scaled_size * 0.56 * num_scale).max(4.5);
                painter.text(
                    draw_center,
                    Align2::CENTER_CENTER,
                    cell.adjacent_mines.to_string(),
                    FontId::new(font_px, FontFamily::Name("InterBold".into())),
                    num_col,
                );
            }
        }
    }

    // Loss shockwave ring expanding from the exploded mine
    if let Some((ox, oy)) = anim.shockwave_origin {
        if anim.shockwave_t < 0.999 {
            let origin_pos = Pos2::new(
                start_x + ox as f32 * cell_pitch + cell_size * 0.5,
                start_y + oy as f32 * cell_pitch + cell_size * 0.5,
            );
            let max_radius = total_w.hypot(total_h) * 0.72;
            let eased = ease_out_cubic(anim.shockwave_t);
            let radius = cell_size * 0.4 + max_radius * eased;
            let alpha = (1.0 - eased).powi(2);
            let stroke_w = (4.0 * (1.0 - eased)).max(1.2);
            painter.circle_stroke(
                origin_pos,
                radius,
                Stroke::new(stroke_w, with_alpha(palette.danger, alpha * 0.85)),
            );
            if anim.shockwave_t > 0.12 {
                let inner_eased = ease_out_cubic((anim.shockwave_t - 0.12) / 0.88);
                let inner_r = cell_size * 0.3 + max_radius * 0.55 * inner_eased;
                let inner_alpha = (1.0 - inner_eased).powi(2) * 0.55;
                painter.circle_stroke(
                    origin_pos,
                    inner_r,
                    Stroke::new(1.6_f32, with_alpha(palette.accent, inner_alpha)),
                );
            }
        }
    }

    // Win celebration vector particles (minimalist confetti & 4-point sparkles)
    for p in &anim.particles {
        let progress = (p.life / p.max_life).clamp(0.0, 1.0);
        let alpha = 1.0 - ease_out_cubic(progress);
        let col = with_alpha(palette.number_colors[p.color_idx % 8], alpha);
        let screen_pos = Pos2::new(
            board_rect.left() + p.pos.x * board_rect.width(),
            board_rect.top() + p.pos.y * board_rect.height(),
        );

        if p.is_sparkle {
            // 4-point vector star sparkle
            let r_long = p.size * 1.25;
            let dir1 = Vec2::angled(p.rotation);
            let dir2 = Vec2::angled(p.rotation + std::f32::consts::FRAC_PI_2);
            let stroke = Stroke::new(1.7_f32, col);
            painter.line_segment(
                [screen_pos - dir1 * r_long, screen_pos + dir1 * r_long],
                stroke,
            );
            painter.line_segment(
                [screen_pos - dir2 * r_long, screen_pos + dir2 * r_long],
                stroke,
            );
        } else {
            // Rotated minimalist confetti diamond/rectangle
            let u = Vec2::angled(p.rotation) * p.size;
            let v = Vec2::angled(p.rotation + std::f32::consts::FRAC_PI_2) * (p.size * 0.52);
            let poly = vec![
                screen_pos - u - v,
                screen_pos + u - v,
                screen_pos + u + v,
                screen_pos - u + v,
            ];
            painter.add(Shape::convex_polygon(poly, col, Stroke::NONE));
        }
    }

    // Paused overlay card
    if paused_active {
        painter.rect_filled(
            board_rect,
            CornerRadius::same(10),
            with_alpha(palette.bg_app, 0.78),
        );
        let card = Rect::from_center_size(board_rect.center(), Vec2::new(220.0, 82.0));
        painter.rect_filled(card, CornerRadius::same(12), palette.bg_top_bar);
        painter.rect_stroke(
            card,
            CornerRadius::same(12),
            Stroke::new(1.2_f32, palette.accent),
            StrokeKind::Inside,
        );
        painter.text(
            card.center() + Vec2::new(0.0, -12.0),
            Align2::CENTER_CENTER,
            "Game Paused",
            FontId::new(16.5, FontFamily::Name("InterBold".into())),
            palette.text_primary,
        );
        painter.text(
            card.center() + Vec2::new(0.0, 13.0),
            Align2::CENTER_CENTER,
            "Click anywhere to resume",
            FontId::new(12.5, FontFamily::Proportional),
            palette.text_secondary,
        );

        let resume_resp = ui.interact(board_rect, ui.id().with("pause_overlay"), Sense::click());
        if resume_resp.clicked() {
            actions.push(UiAction::ResumeFromPause);
        }
    }

    // Bottom Win / Loss summary banner with animated count-up timer & best time
    if matches!(board.state(), GameState::Won | GameState::Lost) {
        let is_win = board.state() == GameState::Won;
        let count_ease = ease_out_cubic(anim.win_countup_t);
        let shown_time = elapsed_secs * (count_ease as f64);

        let banner_w = (avail.x - 8.0).clamp(240.0, 540.0);
        let compact_banner = banner_w < 430.0;
        let banner_rect = Rect::from_center_size(
            Pos2::new(full_rect.center().x, full_rect.bottom() - 24.0),
            Vec2::new(banner_w, 40.0),
        );

        painter.rect_filled(banner_rect, CornerRadius::same(10), palette.bg_top_bar);
        painter.rect_stroke(
            banner_rect,
            CornerRadius::same(10),
            Stroke::new(
                1.2_f32,
                if is_win {
                    palette.success
                } else {
                    palette.danger
                },
            ),
            StrokeKind::Inside,
        );

        let status_label = if is_win {
            "Cleared!"
        } else if compact_banner {
            "Detonated"
        } else {
            "Mine Detonated"
        };
        let status_col = if is_win {
            palette.success
        } else {
            palette.danger
        };

        painter.text(
            Pos2::new(banner_rect.left() + 14.0, banner_rect.center().y),
            Align2::LEFT_CENTER,
            status_label,
            FontId::new(
                if compact_banner { 13.0 } else { 14.0 },
                FontFamily::Name("InterBold".into()),
            ),
            status_col,
        );

        // Play again button inside banner
        let btn_w = if compact_banner { 72.0 } else { 88.0 };
        let again_rect = Rect::from_center_size(
            Pos2::new(
                banner_rect.right() - 10.0 - btn_w * 0.5,
                banner_rect.center().y,
            ),
            Vec2::new(btn_w, 28.0),
        );
        let again_resp = ui.interact(again_rect, ui.id().with("banner_again_btn"), Sense::click());
        painter.rect_filled(
            again_rect,
            CornerRadius::same(7),
            if again_resp.hovered() {
                palette.accent_hover
            } else {
                palette.accent
            },
        );
        painter.text(
            again_rect.center(),
            Align2::CENTER_CENTER,
            if compact_banner {
                "Restart"
            } else {
                "Restart (N)"
            },
            FontId::new(12.0, FontFamily::Name("InterBold".into())),
            Color32::WHITE,
        );
        if again_resp.clicked() {
            actions.push(UiAction::Restart);
        }

        // Center the time summary between the left status label and the right button
        let left_reserved = if is_win {
            78.0
        } else if compact_banner {
            86.0
        } else {
            128.0
        };
        let text_center_x = (banner_rect.left() + left_reserved + again_rect.left() - 6.0) * 0.5;

        let best_text = if let Some(best) = best_times.get(board.difficulty()) {
            let shown_best = best * (count_ease as f64);
            if compact_banner {
                format!(
                    "{} · Best {}",
                    format_duration(shown_time),
                    format_duration(shown_best)
                )
            } else {
                format!(
                    "Time: {}  ·  Best: {}",
                    format_duration(shown_time),
                    format_duration(shown_best)
                )
            }
        } else {
            format!("Time: {}", format_duration(shown_time))
        };

        painter.text(
            Pos2::new(text_center_x, banner_rect.center().y),
            Align2::CENTER_CENTER,
            best_text,
            FontId::new(
                if compact_banner { 12.0 } else { 13.0 },
                FontFamily::Proportional,
            ),
            palette.text_primary,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_renders_50x35_board_at_min_400x460_window_with_settings_open() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);

        let mut board = Board::new(Difficulty::Custom {
            width: MAX_CUSTOM_WIDTH,
            height: MAX_CUSTOM_HEIGHT,
            mines: 200,
        });
        board.reveal(25, 17);

        let mut anim = AnimationManager::new(
            board.width(),
            board.height(),
            board.remaining_mines(),
            ThemeMode::Dark,
            false,
        );
        anim.set_settings_open(true, true);
        anim.tick(0.5);

        let mut best_times = BestTimes::default();
        best_times.record(Difficulty::Beginner, 12.4);
        best_times.record(Difficulty::Intermediate, 48.9);
        best_times.record(Difficulty::Expert, 132.1);

        let mut draft = CustomDraft::new(50, 35, 200);
        let palette = anim.current_palette();
        let mut actions = Vec::new();

        let raw_input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 460.0))),
            ..Default::default()
        };

        let full_output = ctx.run(raw_input, |ctx| {
            let panel_frame = egui::Frame::new()
                .fill(palette.bg_app)
                .inner_margin(egui::Margin::symmetric(16, 14));
            egui::CentralPanel::default()
                .frame(panel_frame)
                .show(ctx, |ui| {
                    draw_top_bar(
                        ui,
                        &board,
                        &anim,
                        &best_times,
                        false,
                        true,
                        true,
                        &palette,
                        &mut actions,
                    );
                    ui.add_space(6.0);
                    draw_difficulty_bar(ui, &board, (50, 35, 200), &palette, &mut actions);
                    ui.add_space(6.0);
                    draw_settings_panel(
                        ui,
                        &board,
                        &anim,
                        ThemeMode::Dark,
                        false,
                        true,
                        0.75,
                        &best_times,
                        &mut draft,
                        &palette,
                        &mut actions,
                    );
                    ui.add_space(4.0);
                    let before_board_top = ui.cursor().top();
                    let avail_before_board = ui.available_size();
                    draw_board(
                        ui,
                        &board,
                        &mut anim,
                        15.3,
                        &best_times,
                        false,
                        false,
                        0.016,
                        &palette,
                        &mut actions,
                    );
                    assert!(
                        before_board_top < 460.0 && avail_before_board.y > 150.0,
                        "Board must have ample vertical space even at 400x460 with settings open (top={before_board_top}, avail_h={})",
                        avail_before_board.y
                    );
                });
        });

        assert!(!full_output.shapes.is_empty());
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 460.0));
        for clipped in &full_output.shapes {
            let bounds = clipped.shape.visual_bounding_rect();
            if bounds.is_finite() && bounds.width() > 0.0 && bounds.height() > 0.0 {
                assert!(
                    bounds.left() >= viewport.left() - 2.0
                        && bounds.right() <= viewport.right() + 2.0
                        && bounds.top() >= viewport.top() - 2.0
                        && bounds.bottom() <= viewport.bottom() + 2.0,
                    "Shape visual bounds {bounds:?} overflowed 400x460 viewport"
                );
            }
        }
    }

    #[test]
    fn test_ui_renders_win_and_loss_banners_at_min_width_without_overflow() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);

        let mut best_times = BestTimes::default();
        best_times.record(Difficulty::Beginner, 9.8);
        let palette = ThemePalette::dark();
        let mut actions = Vec::new();
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 460.0));

        // 1. Test Lost state with wrong flag and exploded mine
        let mut lost_board = Board::from_mine_positions(9, 9, &[(0, 0), (4, 4)]);
        lost_board.toggle_flag(1, 0); // wrong flag
        lost_board.reveal(0, 0); // explode mine
        assert_eq!(lost_board.state(), GameState::Lost);

        let mut anim = AnimationManager::new(9, 9, 1, ThemeMode::Dark, true);
        anim.notify_loss((0, 0), &[(0, 0), (4, 4)], true);

        let out_lost = ctx.run(
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_top_bar(
                        ui,
                        &lost_board,
                        &anim,
                        &best_times,
                        false,
                        false,
                        false,
                        &palette,
                        &mut actions,
                    );
                    draw_difficulty_bar(ui, &lost_board, (9, 9, 10), &palette, &mut actions);
                    draw_board(
                        ui,
                        &lost_board,
                        &mut anim,
                        14.2,
                        &best_times,
                        false,
                        true,
                        0.016,
                        &palette,
                        &mut actions,
                    );
                });
            },
        );
        assert!(!out_lost.shapes.is_empty());

        // 2. Test Won state with particles and countup banner
        let mut won_board = Board::from_mine_positions(5, 5, &[(4, 4)]);
        won_board.reveal(0, 0);
        assert_eq!(won_board.state(), GameState::Won);
        anim.notify_win(false);
        anim.tick(0.2);

        let out_won = ctx.run(
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_board(
                        ui,
                        &won_board,
                        &mut anim,
                        8.5,
                        &best_times,
                        false,
                        false,
                        0.016,
                        &palette,
                        &mut actions,
                    );
                });
            },
        );
        assert!(!out_won.shapes.is_empty());

        // 3. Test wide viewport (680x740) with settings open and volume slider visible
        let wide_viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(680.0, 740.0));
        anim.set_settings_open(true, true);
        let out_wide = ctx.run(
            egui::RawInput {
                screen_rect: Some(wide_viewport),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut draft = CustomDraft::new(12, 12, 20);
                    draw_top_bar(
                        ui,
                        &won_board,
                        &anim,
                        &best_times,
                        false,
                        true,
                        true,
                        &palette,
                        &mut actions,
                    );
                    draw_settings_panel(
                        ui,
                        &won_board,
                        &anim,
                        ThemeMode::Light,
                        false,
                        true,
                        0.75,
                        &best_times,
                        &mut draft,
                        &ThemePalette::light(),
                        &mut actions,
                    );
                });
            },
        );
        assert!(!out_wide.shapes.is_empty());
    }
}
