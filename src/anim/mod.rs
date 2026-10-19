use crate::storage::ThemeMode;
use crate::ui::theme::ThemePalette;
use egui::{Pos2, Vec2};
use rand::Rng;
use std::f32::consts::TAU;

/// Cubic ease-out: fast start, smooth deceleration (`1 - (1 - t)^3`).
#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in-out: smooth acceleration in the first half, smooth deceleration in the second half.
#[inline]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) * 0.5
    }
}

/// Light spring/overshoot easing (`ease_out_back`) for pop-in effects.
#[inline]
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const C1: f32 = 1.55;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// Computes `(y_offset_factor, scale)` for a falling flag with a subtle landing bounce.
/// `y_offset_factor` is multiplied by `cell_size` (negative = above center).
#[inline]
pub fn flag_drop_bounce(t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    if t < 0.65 {
        // Falling phase from -0.38 * cell_size to 0.0
        let p = t / 0.65;
        let eased = p * p;
        let y_factor = -0.38 * (1.0 - eased);
        let scale = 0.75 + 0.25 * ease_out_cubic(p);
        (y_factor, scale)
    } else {
        // Small bounce phase after landing
        let p = (t - 0.65) / 0.35;
        let bounce = (p * std::f32::consts::PI).sin();
        let y_factor = -0.09 * bounce;
        let scale = 1.0 + 0.08 * bounce;
        (y_factor, scale)
    }
}

/// Animation state for a single cell on the board.
#[derive(Clone, Copy, Debug)]
pub struct CellAnim {
    /// Hover transition progress `0.0..=1.0` (~120ms).
    pub hover_t: f32,
    /// Stagger delay before reveal animation begins (used for flood-fill wave).
    pub reveal_delay: f32,
    /// Reveal scale/fade progress `0.0..=1.0`.
    pub reveal_t: f32,
    /// Flag placement drop/bounce progress `0.0..=1.0`.
    pub flag_in_t: f32,
    /// Flag removal fade-out progress `0.0..=1.0` (`1.0` = just removed, `0.0` = finished).
    pub flag_out_t: f32,
    /// Delay before mine pops into view on loss.
    pub mine_pop_delay: f32,
    /// Mine pop-in progress `0.0..=1.0` on loss.
    pub mine_pop_t: f32,
    /// Stagger delay before board-reset fade-in starts (center outward).
    pub reset_delay: f32,
    /// Board-reset fade-in progress `0.0..=1.0`.
    pub reset_t: f32,
}

impl Default for CellAnim {
    fn default() -> Self {
        Self {
            hover_t: 0.0,
            reveal_delay: 0.0,
            reveal_t: 1.0,
            flag_in_t: 1.0,
            flag_out_t: 0.0,
            mine_pop_delay: 0.0,
            mine_pop_t: 1.0,
            reset_delay: 0.0,
            reset_t: 1.0,
        }
    }
}

impl CellAnim {
    pub fn is_active(&self) -> bool {
        (self.hover_t > 0.001 && self.hover_t < 0.999)
            || self.reveal_delay > 0.0
            || self.reveal_t < 0.999
            || self.flag_in_t < 0.999
            || self.flag_out_t > 0.001
            || self.mine_pop_delay > 0.0
            || self.mine_pop_t < 0.999
            || self.reset_delay > 0.0
            || self.reset_t < 0.999
    }
}

/// Animated rolling/fading character slot for the 3-digit mine counter and timer.
#[derive(Clone, Copy, Debug)]
pub struct DigitSlot {
    pub current: char,
    pub previous: char,
    pub t: f32,
    pub direction: f32,
}

impl Default for DigitSlot {
    fn default() -> Self {
        Self {
            current: '0',
            previous: '0',
            t: 1.0,
            direction: 1.0,
        }
    }
}

/// 4-slot rolling numeric display (supports negative sign or 3 digits).
#[derive(Clone, Debug)]
pub struct RollingDisplay {
    pub slots: [DigitSlot; 4],
    last_value: isize,
}

impl RollingDisplay {
    pub fn new(initial_value: isize) -> Self {
        let chars = Self::format_chars(initial_value);
        let mut slots = [DigitSlot::default(); 4];
        for (i, ch) in chars.into_iter().enumerate() {
            slots[i] = DigitSlot {
                current: ch,
                previous: ch,
                t: 1.0,
                direction: 1.0,
            };
        }
        Self {
            slots,
            last_value: initial_value,
        }
    }

    fn format_chars(value: isize) -> [char; 4] {
        let clamped = value.clamp(-99, 999);
        let s = if clamped < 0 {
            format!("-{:02}", clamped.abs())
        } else {
            format!(" {:03}", clamped)
        };
        let mut out = [' '; 4];
        for (i, ch) in s.chars().take(4).enumerate() {
            out[i] = ch;
        }
        out
    }

    pub fn set_value(&mut self, new_value: isize, reduce_motion: bool) {
        if new_value == self.last_value {
            return;
        }
        let dir = if new_value > self.last_value {
            1.0
        } else {
            -1.0
        };
        self.last_value = new_value;
        let new_chars = Self::format_chars(new_value);
        for (slot, new_ch) in self.slots.iter_mut().zip(new_chars) {
            if slot.current != new_ch {
                slot.previous = slot.current;
                slot.current = new_ch;
                slot.direction = dir;
                slot.t = if reduce_motion { 1.0 } else { 0.0 };
            }
        }
    }

    pub fn tick(&mut self, dt: f32) {
        let speed = 1.0 / 0.18;
        for slot in &mut self.slots {
            if slot.t < 1.0 {
                slot.t = (slot.t + dt * speed).min(1.0);
            }
        }
    }

    pub fn is_animating(&self) -> bool {
        self.slots.iter().any(|s| s.t < 0.999)
    }
}

/// Minimalist vector particle for the win celebration effect.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Pos2,
    pub vel: Vec2,
    pub size: f32,
    pub rotation: f32,
    pub angular_vel: f32,
    pub color_idx: usize,
    pub life: f32,
    pub max_life: f32,
    pub is_sparkle: bool,
}

/// Central animation manager for the board, top-bar counters, particles, and UI panels.
#[derive(Clone, Debug)]
pub struct AnimationManager {
    width: usize,
    height: usize,
    cells: Vec<CellAnim>,
    pub mine_counter: RollingDisplay,
    pub timer_display: RollingDisplay,
    /// Shockwave ring origin in grid coordinates `(x, y)` and progress `0.0..=1.0`.
    pub shockwave_origin: Option<(usize, usize)>,
    pub shockwave_t: f32,
    /// Remaining screen shake intensity `0.0..=1.0`.
    pub shake_t: f32,
    /// Win celebration particles in normalized board coordinates `0.0..=1.0`.
    pub particles: Vec<Particle>,
    /// Win banner and timer count-up progress `0.0..=1.0`.
    pub win_countup_t: f32,
    pub win_banner_active: bool,
    /// Theme transition progress (`0.0` = Dark, `1.0` = Light).
    pub theme_t: f32,
    pub target_theme_t: f32,
    /// Settings panel open/close progress (`0.0` = closed, `1.0` = open).
    pub settings_t: f32,
    pub settings_target: f32,
}

impl AnimationManager {
    pub fn new(
        width: usize,
        height: usize,
        initial_mines: isize,
        theme: ThemeMode,
        reduce_motion: bool,
    ) -> Self {
        let theme_val = match theme {
            ThemeMode::Dark => 0.0,
            ThemeMode::Light => 1.0,
        };
        let mut mgr = Self {
            width,
            height,
            cells: vec![CellAnim::default(); width * height],
            mine_counter: RollingDisplay::new(initial_mines),
            timer_display: RollingDisplay::new(0),
            shockwave_origin: None,
            shockwave_t: 1.0,
            shake_t: 0.0,
            particles: Vec::new(),
            win_countup_t: 1.0,
            win_banner_active: false,
            theme_t: theme_val,
            target_theme_t: theme_val,
            settings_t: 0.0,
            settings_target: 0.0,
        };
        mgr.trigger_board_reset(width, height, initial_mines, reduce_motion);
        mgr
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    #[inline]
    pub fn cell(&self, x: usize, y: usize) -> Option<&CellAnim> {
        self.idx(x, y).and_then(|i| self.cells.get(i))
    }

    #[inline]
    pub fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut CellAnim> {
        self.idx(x, y).and_then(|i| self.cells.get_mut(i))
    }

    /// Triggers the radial center-outward stagger fade-in animation when resetting the board.
    pub fn trigger_board_reset(
        &mut self,
        width: usize,
        height: usize,
        initial_mines: isize,
        reduce_motion: bool,
    ) {
        self.width = width;
        self.height = height;
        self.cells = vec![CellAnim::default(); width * height];
        self.shockwave_origin = None;
        self.shockwave_t = 1.0;
        self.shake_t = 0.0;
        self.particles.clear();
        self.win_countup_t = 1.0;
        self.win_banner_active = false;
        self.mine_counter.set_value(initial_mines, reduce_motion);
        self.timer_display.set_value(0, reduce_motion);

        if reduce_motion {
            return;
        }

        let cx = (width.saturating_sub(1)) as f32 * 0.5;
        let cy = (height.saturating_sub(1)) as f32 * 0.5;
        let max_dist = cx.hypot(cy).max(1.0);

        for y in 0..height {
            for x in 0..width {
                let dist = (x as f32 - cx).hypot(y as f32 - cy) / max_dist;
                if let Some(anim) = self.cell_mut(x, y) {
                    anim.reset_delay = dist * 0.20;
                    anim.reset_t = 0.0;
                }
            }
        }
    }

    /// Updates hover target for cell `(x, y)` (called during board rendering).
    pub fn update_cell_hover(
        &mut self,
        x: usize,
        y: usize,
        is_hovered: bool,
        dt: f32,
        reduce_motion: bool,
    ) {
        let Some(anim) = self.cell_mut(x, y) else {
            return;
        };
        if reduce_motion {
            anim.hover_t = if is_hovered { 1.0 } else { 0.0 };
            return;
        }
        // ~120ms hover transition
        let rate = dt / 0.12;
        if is_hovered {
            anim.hover_t = (anim.hover_t + rate).min(1.0);
        } else {
            anim.hover_t = (anim.hover_t - rate).max(0.0);
        }
    }

    /// Triggers scale & fade-in reveal animations with a wave delay proportional to BFS distance.
    pub fn notify_cells_revealed(
        &mut self,
        revealed: &[(usize, usize, usize)],
        reduce_motion: bool,
    ) {
        if reduce_motion {
            for &(x, y, _) in revealed {
                if let Some(anim) = self.cell_mut(x, y) {
                    anim.reveal_delay = 0.0;
                    anim.reveal_t = 1.0;
                }
            }
            return;
        }

        for &(x, y, dist) in revealed {
            if let Some(anim) = self.cell_mut(x, y) {
                // Wave propagation delay capped so large flood-fills remain snappy
                anim.reveal_delay = ((dist as f32) * 0.024).min(0.40);
                anim.reveal_t = 0.0;
            }
        }
    }

    /// Triggers flag drop/bounce or flag fade-out animation on `(x, y)`.
    pub fn notify_flag_changed(&mut self, x: usize, y: usize, placed: bool, reduce_motion: bool) {
        let Some(anim) = self.cell_mut(x, y) else {
            return;
        };
        if reduce_motion {
            anim.flag_in_t = 1.0;
            anim.flag_out_t = 0.0;
            return;
        }
        if placed {
            anim.flag_in_t = 0.0;
            anim.flag_out_t = 0.0;
        } else {
            anim.flag_out_t = 1.0;
        }
    }

    /// Triggers loss animations: shockwave ring, screen shake, and sequential mine pop-in.
    pub fn notify_loss(
        &mut self,
        origin: (usize, usize),
        all_mines: &[(usize, usize)],
        reduce_motion: bool,
    ) {
        if reduce_motion {
            self.shockwave_origin = None;
            self.shockwave_t = 1.0;
            self.shake_t = 0.0;
            for &(mx, my) in all_mines {
                if let Some(anim) = self.cell_mut(mx, my) {
                    anim.mine_pop_delay = 0.0;
                    anim.mine_pop_t = 1.0;
                }
            }
            return;
        }

        self.shockwave_origin = Some(origin);
        self.shockwave_t = 0.0;
        self.shake_t = 1.0;

        let (ox, oy) = (origin.0 as f32, origin.1 as f32);
        for &(mx, my) in all_mines {
            let dist = (mx as f32 - ox).hypot(my as f32 - oy);
            if let Some(anim) = self.cell_mut(mx, my) {
                anim.mine_pop_delay = (dist * 0.036).min(0.75);
                anim.mine_pop_t = 0.0;
            }
        }
    }

    /// Triggers win animations: vector confetti/sparkles and timer/best-time count-up.
    pub fn notify_win(&mut self, reduce_motion: bool) {
        self.win_banner_active = true;
        if reduce_motion {
            self.win_countup_t = 1.0;
            self.particles.clear();
            return;
        }

        self.win_countup_t = 0.0;
        self.particles.clear();
        let mut rng = rand::thread_rng();
        for i in 0..48 {
            let x = rng.gen_range(0.15..0.85);
            let y = rng.gen_range(0.20..0.65);
            let angle = rng.gen_range(0.0..TAU);
            let speed = rng.gen_range(0.18..0.55);
            let vx = angle.cos() * speed * 0.7;
            let vy = -angle.sin().abs() * speed - rng.gen_range(0.10..0.30);
            let max_life = rng.gen_range(1.1..1.9);
            self.particles.push(Particle {
                pos: Pos2::new(x, y),
                vel: Vec2::new(vx, vy),
                size: rng.gen_range(4.5..8.5),
                rotation: rng.gen_range(0.0..TAU),
                angular_vel: rng.gen_range(-5.0..5.0),
                color_idx: i % 8,
                life: 0.0,
                max_life,
                is_sparkle: i % 3 == 0,
            });
        }
    }

    /// Sets the target theme and animates palette colors smoothly.
    pub fn set_theme(&mut self, mode: ThemeMode, reduce_motion: bool) {
        self.target_theme_t = match mode {
            ThemeMode::Dark => 0.0,
            ThemeMode::Light => 1.0,
        };
        if reduce_motion {
            self.theme_t = self.target_theme_t;
        }
    }

    /// Toggles or sets the settings panel open state.
    pub fn set_settings_open(&mut self, open: bool, reduce_motion: bool) {
        self.settings_target = if open { 1.0 } else { 0.0 };
        if reduce_motion {
            self.settings_t = self.settings_target;
        }
    }

    /// Snaps all in-flight animations to completion when `reduce_motion` is enabled.
    pub fn set_reduce_motion(&mut self, reduce_motion: bool) {
        if !reduce_motion {
            return;
        }
        self.shockwave_origin = None;
        self.shockwave_t = 1.0;
        self.shake_t = 0.0;
        self.particles.clear();
        self.win_countup_t = 1.0;
        self.theme_t = self.target_theme_t;
        self.settings_t = self.settings_target;
        for slot in &mut self.mine_counter.slots {
            slot.t = 1.0;
            slot.previous = slot.current;
        }
        for slot in &mut self.timer_display.slots {
            slot.t = 1.0;
            slot.previous = slot.current;
        }
        for cell in &mut self.cells {
            cell.reveal_delay = 0.0;
            cell.reveal_t = 1.0;
            cell.flag_in_t = 1.0;
            cell.flag_out_t = 0.0;
            cell.mine_pop_delay = 0.0;
            cell.mine_pop_t = 1.0;
            cell.reset_delay = 0.0;
            cell.reset_t = 1.0;
            cell.hover_t = if cell.hover_t >= 0.5 { 1.0 } else { 0.0 };
        }
    }

    /// Returns the currently interpolated `ThemePalette`.
    pub fn current_palette(&self) -> ThemePalette {
        let eased = ease_in_out_cubic(self.theme_t);
        ThemePalette::lerp(&ThemePalette::dark(), &ThemePalette::light(), eased)
    }

    /// Computes the current screen-shake pixel offset.
    pub fn board_shake_offset(&self) -> Vec2 {
        if self.shake_t <= 0.001 {
            return Vec2::ZERO;
        }
        let amp = 6.0 * self.shake_t * self.shake_t;
        let phase = (1.0 - self.shake_t) * 52.0;
        Vec2::new(phase.sin() * amp, (phase * 1.37).cos() * amp * 0.65)
    }

    /// Advances all active animations by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);

        for cell in &mut self.cells {
            if cell.reveal_delay > 0.0 {
                cell.reveal_delay = (cell.reveal_delay - dt).max(0.0);
            } else if cell.reveal_t < 1.0 {
                cell.reveal_t = (cell.reveal_t + dt / 0.18).min(1.0);
            }

            if cell.flag_in_t < 1.0 {
                cell.flag_in_t = (cell.flag_in_t + dt / 0.24).min(1.0);
            }
            if cell.flag_out_t > 0.0 {
                cell.flag_out_t = (cell.flag_out_t - dt / 0.16).max(0.0);
            }

            if cell.mine_pop_delay > 0.0 {
                cell.mine_pop_delay = (cell.mine_pop_delay - dt).max(0.0);
            } else if cell.mine_pop_t < 1.0 {
                cell.mine_pop_t = (cell.mine_pop_t + dt / 0.26).min(1.0);
            }

            if cell.reset_delay > 0.0 {
                cell.reset_delay = (cell.reset_delay - dt).max(0.0);
            } else if cell.reset_t < 1.0 {
                cell.reset_t = (cell.reset_t + dt / 0.22).min(1.0);
            }
        }

        self.mine_counter.tick(dt);
        self.timer_display.tick(dt);

        if self.shockwave_t < 1.0 {
            self.shockwave_t = (self.shockwave_t + dt / 0.65).min(1.0);
        }
        if self.shake_t > 0.0 {
            self.shake_t = (self.shake_t - dt / 0.36).max(0.0);
        }
        if self.win_countup_t < 1.0 {
            self.win_countup_t = (self.win_countup_t + dt / 0.85).min(1.0);
        }

        // Update particles
        for p in &mut self.particles {
            p.life += dt;
            p.vel.y += 0.65 * dt; // gentle gravity
            p.pos += p.vel * dt;
            p.rotation += p.angular_vel * dt;
        }
        self.particles.retain(|p| p.life < p.max_life);

        // Theme transition (~260ms)
        if (self.theme_t - self.target_theme_t).abs() > 0.001 {
            let step = dt / 0.26;
            if self.target_theme_t > self.theme_t {
                self.theme_t = (self.theme_t + step).min(self.target_theme_t);
            } else {
                self.theme_t = (self.theme_t - step).max(self.target_theme_t);
            }
        } else {
            self.theme_t = self.target_theme_t;
        }

        // Settings panel fade/slide (~200ms)
        if (self.settings_t - self.settings_target).abs() > 0.001 {
            let step = dt / 0.20;
            if self.settings_target > self.settings_t {
                self.settings_t = (self.settings_t + step).min(self.settings_target);
            } else {
                self.settings_t = (self.settings_t - step).max(self.settings_target);
            }
        } else {
            self.settings_t = self.settings_target;
        }
    }

    /// Returns `true` if any animation is currently active and needs continuous 60fps repainting.
    pub fn is_animating(&self) -> bool {
        self.cells.iter().any(CellAnim::is_active)
            || self.mine_counter.is_animating()
            || self.timer_display.is_animating()
            || self.shockwave_t < 0.999
            || self.shake_t > 0.001
            || !self.particles.is_empty()
            || self.win_countup_t < 0.999
            || (self.theme_t - self.target_theme_t).abs() > 0.001
            || (self.settings_t - self.settings_target).abs() > 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_boundaries() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 1e-5);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-5);
        assert!(ease_out_cubic(0.5) > 0.5);

        assert!((ease_in_out_cubic(0.0) - 0.0).abs() < 1e-5);
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-5);
        assert!((ease_in_out_cubic(1.0) - 1.0).abs() < 1e-5);

        assert!((ease_out_back(0.0) - 0.0).abs() < 1e-5);
        assert!((ease_out_back(1.0) - 1.0).abs() < 1e-5);
        // Overshoot in the upper-middle range
        assert!(ease_out_back(0.6) > 1.0);
    }

    #[test]
    fn test_rolling_display_transitions() {
        let mut disp = RollingDisplay::new(10);
        assert!(!disp.is_animating());

        disp.set_value(9, false);
        assert!(disp.is_animating());

        disp.tick(0.5);
        assert!(!disp.is_animating());
    }

    #[test]
    fn test_reduce_motion_snaps_active_animations() {
        let mut mgr = AnimationManager::new(9, 9, 10, ThemeMode::Dark, false);
        assert!(mgr.is_animating());

        mgr.notify_win(false);
        assert!(!mgr.particles.is_empty());
        assert!(mgr.is_animating());

        mgr.set_reduce_motion(true);
        assert!(mgr.particles.is_empty());
        assert!(!mgr.is_animating());
    }
}
