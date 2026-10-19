use crate::ui::theme::with_alpha;
use egui::epaint::PathShape;
use egui::{Color32, Painter, Pos2, Shape, Stroke, Vec2};
use std::f32::consts::{PI, TAU};

/// Draws a minimalist vector mine: solid center circle with 8 short radial lines around it.
pub fn draw_mine(
    painter: &Painter,
    center: Pos2,
    cell_size: f32,
    color: Color32,
    scale: f32,
    alpha: f32,
) {
    let s = scale.max(0.0);
    if s <= 0.01 || alpha <= 0.01 {
        return;
    }
    let col = with_alpha(color, alpha);
    let core_radius = (cell_size * 0.22 * s).max(1.0);
    let inner_ray = core_radius * 0.85;
    let outer_ray = (cell_size * 0.36 * s).max(core_radius + 0.8);
    let stroke_w = (cell_size * 0.068 * s).clamp(1.0, 2.0);
    let stroke = Stroke::new(stroke_w, col);

    // 8 short radial lines around the core
    for i in 0..8 {
        let angle = (i as f32) * (TAU / 8.0);
        let dir = Vec2::angled(angle);
        painter.line_segment([center + dir * inner_ray, center + dir * outer_ray], stroke);
    }

    // Solid circle body
    painter.circle_filled(center, core_radius, col);

    // Subtle specular glint inside the mine
    let glint_col = with_alpha(Color32::WHITE, 0.35 * alpha);
    let glint_pos = center + Vec2::new(-core_radius * 0.32, -core_radius * 0.32);
    painter.circle_filled(glint_pos, (core_radius * 0.24).max(0.6), glint_col);
}

/// Draws a minimalist vector flag: thin pole with a sleek triangular/curved pennant and base.
#[allow(clippy::too_many_arguments)]
pub fn draw_flag(
    painter: &Painter,
    center: Pos2,
    cell_size: f32,
    pole_color: Color32,
    flag_color: Color32,
    scale: f32,
    y_offset: f32,
    alpha: f32,
) {
    let s = scale.max(0.0);
    if s <= 0.01 || alpha <= 0.01 {
        return;
    }
    let c = center + Vec2::new(0.0, y_offset);
    let pole_col = with_alpha(pole_color, alpha);
    let banner_col = with_alpha(flag_color, alpha);
    let stroke_w = (cell_size * 0.065 * s).clamp(1.0, 2.0);
    let pole_stroke = Stroke::new(stroke_w, pole_col);

    let pole_x = c.x - cell_size * 0.13 * s;
    let top_y = c.y - cell_size * 0.28 * s;
    let bot_y = c.y + cell_size * 0.27 * s;

    // Pennant wave polygon (sleek 4-point slightly curved pennant)
    let p_top = Pos2::new(pole_x, top_y);
    let p_mid_top = Pos2::new(c.x + cell_size * 0.04 * s, top_y + cell_size * 0.03 * s);
    let p_tip = Pos2::new(c.x + cell_size * 0.24 * s, top_y + cell_size * 0.14 * s);
    let p_bot = Pos2::new(pole_x, top_y + cell_size * 0.29 * s);

    painter.add(Shape::convex_polygon(
        vec![p_top, p_mid_top, p_tip, p_bot],
        banner_col,
        Stroke::new((stroke_w * 0.7).max(0.8), banner_col),
    ));

    // Thin vertical pole
    painter.line_segment(
        [Pos2::new(pole_x, top_y), Pos2::new(pole_x, bot_y)],
        pole_stroke,
    );

    // Subtle horizontal base
    let base_half = cell_size * 0.12 * s;
    painter.line_segment(
        [
            Pos2::new(pole_x - base_half * 0.6, bot_y),
            Pos2::new(pole_x + base_half, bot_y),
        ],
        pole_stroke,
    );
}

/// Draws a crisp diagonal cross over a misplaced flag when the game is lost.
pub fn draw_wrong_flag_cross(
    painter: &Painter,
    center: Pos2,
    cell_size: f32,
    color: Color32,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let col = with_alpha(color, alpha);
    let half = cell_size * 0.27;
    let stroke = Stroke::new((cell_size * 0.075).clamp(1.1, 1.9), col);
    painter.line_segment(
        [
            Pos2::new(center.x - half, center.y - half),
            Pos2::new(center.x + half, center.y + half),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(center.x + half, center.y - half),
            Pos2::new(center.x - half, center.y + half),
        ],
        stroke,
    );
}

/// Draws a circular restart arrow icon (stroke 1.75px).
pub fn draw_restart_icon(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let radius = size * 0.36;
    let stroke = Stroke::new(1.75_f32, color);
    let segments = 22;
    let start_angle = -PI * 0.65;
    let sweep = TAU * 0.78;

    let mut pts = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = start_angle + sweep * t;
        pts.push(center + Vec2::angled(a) * radius);
    }
    painter.add(PathShape::line(pts.clone(), stroke));

    // Arrowhead at the end of the arc
    if let Some(&tip) = pts.last() {
        let end_angle = start_angle + sweep;
        let tangent = Vec2::angled(end_angle + PI * 0.5);
        let inward = Vec2::angled(end_angle + PI);
        let head_len = size * 0.19;
        let p1 = tip - tangent * head_len + inward * (head_len * 0.45);
        let p2 = tip - tangent * head_len - inward * (head_len * 0.55);
        painter.line_segment([tip, p1], stroke);
        painter.line_segment([tip, p2], stroke);
    }
}

/// Draws a minimalist gear/settings icon (stroke 1.65px).
pub fn draw_settings_icon(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.65_f32, color);
    let inner_r = size * 0.16;
    let body_r = size * 0.30;
    let tooth_r = size * 0.40;

    painter.circle_stroke(center, inner_r, stroke);
    painter.circle_stroke(center, body_r, stroke);

    for i in 0..6 {
        let angle = i as f32 * (TAU / 6.0);
        let dir = Vec2::angled(angle);
        painter.line_segment(
            [center + dir * body_r, center + dir * tooth_r],
            Stroke::new(1.85_f32, color),
        );
    }
}

/// Draws either a pause icon (two vertical bars) or a play icon (triangle) depending on `is_paused`.
pub fn draw_pause_or_play_icon(
    painter: &Painter,
    center: Pos2,
    size: f32,
    color: Color32,
    is_paused: bool,
) {
    let stroke = Stroke::new(1.8_f32, color);
    if is_paused {
        // Play triangle
        let half_h = size * 0.34;
        let left_x = center.x - size * 0.22;
        let right_x = center.x + size * 0.30;
        let pts = vec![
            Pos2::new(left_x, center.y - half_h),
            Pos2::new(right_x, center.y),
            Pos2::new(left_x, center.y + half_h),
        ];
        painter.add(PathShape::closed_line(pts, stroke));
    } else {
        // Two vertical pause bars
        let dx = size * 0.16;
        let half_h = size * 0.32;
        painter.line_segment(
            [
                Pos2::new(center.x - dx, center.y - half_h),
                Pos2::new(center.x - dx, center.y + half_h),
            ],
            stroke,
        );
        painter.line_segment(
            [
                Pos2::new(center.x + dx, center.y - half_h),
                Pos2::new(center.x + dx, center.y + half_h),
            ],
            stroke,
        );
    }
}

/// Draws a minimalist vector clock icon (stroke 1.65px).
pub fn draw_clock_icon(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let radius = size * 0.40;
    let stroke = Stroke::new(1.65_f32, color);
    painter.circle_stroke(center, radius, stroke);
    // Minute hand pointing up
    painter.line_segment([center, center + Vec2::new(0.0, -radius * 0.62)], stroke);
    // Hour hand pointing down-right
    painter.line_segment(
        [center, center + Vec2::new(radius * 0.45, radius * 0.25)],
        stroke,
    );
}

/// Draws a minimalist vector trophy icon (stroke 1.65px).
pub fn draw_trophy_icon(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.65_f32, color);
    let top_y = center.y - size * 0.36;
    let bowl_bot_y = center.y + size * 0.04;
    let base_y = center.y + size * 0.36;
    let half_w = size * 0.26;

    // Cup rim and U-shaped bowl
    let cup_pts = vec![
        Pos2::new(center.x - half_w, top_y),
        Pos2::new(center.x + half_w, top_y),
        Pos2::new(center.x + half_w * 0.82, bowl_bot_y - size * 0.10),
        Pos2::new(center.x, bowl_bot_y),
        Pos2::new(center.x - half_w * 0.82, bowl_bot_y - size * 0.10),
    ];
    painter.add(PathShape::closed_line(cup_pts, stroke));

    // Left & right handles
    let handle_out = half_w * 1.42;
    painter.add(PathShape::line(
        vec![
            Pos2::new(center.x - half_w, top_y + size * 0.06),
            Pos2::new(center.x - handle_out, top_y + size * 0.14),
            Pos2::new(center.x - half_w * 0.85, bowl_bot_y - size * 0.12),
        ],
        stroke,
    ));
    painter.add(PathShape::line(
        vec![
            Pos2::new(center.x + half_w, top_y + size * 0.06),
            Pos2::new(center.x + handle_out, top_y + size * 0.14),
            Pos2::new(center.x + half_w * 0.85, bowl_bot_y - size * 0.12),
        ],
        stroke,
    ));

    // Stem and pedestal base
    painter.line_segment(
        [Pos2::new(center.x, bowl_bot_y), Pos2::new(center.x, base_y)],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(center.x - half_w * 0.75, base_y),
            Pos2::new(center.x + half_w * 0.75, base_y),
        ],
        stroke,
    );
}

/// Draws a minimalist vector speaker icon with sound waves (when enabled) or a mute cross (when disabled).
pub fn draw_speaker_icon(
    painter: &Painter,
    center: Pos2,
    size: f32,
    color: Color32,
    sound_enabled: bool,
) {
    let stroke = Stroke::new(1.65_f32, color);
    let left_x = center.x - size * 0.36;
    let neck_x = center.x - size * 0.18;
    let cone_x = center.x + size * 0.04;
    let box_h = size * 0.14;
    let cone_h = size * 0.32;

    // Speaker body + cone polygon
    let body_pts = vec![
        Pos2::new(left_x, center.y - box_h),
        Pos2::new(neck_x, center.y - box_h),
        Pos2::new(cone_x, center.y - cone_h),
        Pos2::new(cone_x, center.y + cone_h),
        Pos2::new(neck_x, center.y + box_h),
        Pos2::new(left_x, center.y + box_h),
    ];
    painter.add(PathShape::closed_line(body_pts, stroke));

    if sound_enabled {
        // Two curved sound wave arcs on the right
        for &(radius_factor, half_angle) in &[(0.18_f32, 0.65_f32), (0.31_f32, 0.72_f32)] {
            let r = size * radius_factor;
            let segments = 8;
            let mut arc_pts = Vec::with_capacity(segments + 1);
            for i in 0..=segments {
                let t = (i as f32 / segments as f32) * 2.0 - 1.0;
                let angle = t * half_angle;
                arc_pts.push(Pos2::new(cone_x + size * 0.02, center.y) + Vec2::angled(angle) * r);
            }
            painter.add(PathShape::line(arc_pts, stroke));
        }
    } else {
        // Crisp mute cross '×' on the right
        let cx = center.x + size * 0.24;
        let d = size * 0.13;
        painter.line_segment(
            [
                Pos2::new(cx - d, center.y - d),
                Pos2::new(cx + d, center.y + d),
            ],
            stroke,
        );
        painter.line_segment(
            [
                Pos2::new(cx + d, center.y - d),
                Pos2::new(cx - d, center.y + d),
            ],
            stroke,
        );
    }
}
