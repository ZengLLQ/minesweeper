use crate::storage::ThemeMode;
use egui::Color32;

/// Complete color palette for the UI and board rendering.
/// Supports smooth linear interpolation (`lerp`) for animated theme transitions.
#[derive(Clone, Copy, Debug)]
pub struct ThemePalette {
    pub bg_app: Color32,
    pub bg_top_bar: Color32,
    pub bg_panel: Color32,
    pub cell_hidden: Color32,
    pub cell_hidden_hover: Color32,
    pub cell_revealed: Color32,
    pub cell_exploded: Color32,
    pub cell_wrong_flag: Color32,
    pub cell_border_subtle: Color32,
    pub cell_shadow: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub flag_color: Color32,
    pub flag_pole: Color32,
    pub mine_color: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub number_colors: [Color32; 8],
}

impl ThemePalette {
    /// Default minimalist dark theme (#0F1115 background, #1A1D24 cell surface, indigo accent).
    pub fn dark() -> Self {
        Self {
            bg_app: Color32::from_rgb(0x0F, 0x11, 0x15),
            bg_top_bar: Color32::from_rgb(0x15, 0x18, 0x1F),
            bg_panel: Color32::from_rgb(0x17, 0x1A, 0x22),
            cell_hidden: Color32::from_rgb(0x1A, 0x1D, 0x24),
            cell_hidden_hover: Color32::from_rgb(0x25, 0x2A, 0x35),
            cell_revealed: Color32::from_rgb(0x13, 0x15, 0x1B),
            cell_exploded: Color32::from_rgb(0x4C, 0x1D, 0x24),
            cell_wrong_flag: Color32::from_rgb(0x3B, 0x1D, 0x26),
            cell_border_subtle: Color32::from_rgba_premultiplied(255, 255, 255, 12),
            cell_shadow: Color32::from_rgba_premultiplied(0, 0, 0, 70),
            text_primary: Color32::from_rgb(0xEC, 0xF0, 0xF8),
            text_secondary: Color32::from_rgb(0x9E, 0xA6, 0xB8),
            text_muted: Color32::from_rgb(0x64, 0x6D, 0x80),
            accent: Color32::from_rgb(0x63, 0x66, 0xF1), // Harmonious Indigo
            accent_hover: Color32::from_rgb(0x79, 0x7C, 0xF6),
            accent_soft: Color32::from_rgba_premultiplied(99, 102, 241, 36),
            flag_color: Color32::from_rgb(0xF4, 0x60, 0x6E),
            flag_pole: Color32::from_rgb(0xB8, 0xC0, 0xD0),
            mine_color: Color32::from_rgb(0xE2, 0xE8, 0xF0),
            danger: Color32::from_rgb(0xF8, 0x71, 0x71),
            success: Color32::from_rgb(0x34, 0xD3, 0x99),
            number_colors: [
                Color32::from_rgb(0x60, 0xA5, 0xFA), // 1: Soft Sky Blue
                Color32::from_rgb(0x34, 0xD3, 0x99), // 2: Soft Emerald
                Color32::from_rgb(0xF8, 0x71, 0x71), // 3: Soft Coral
                Color32::from_rgb(0xA7, 0x8B, 0xFA), // 4: Soft Lavender
                Color32::from_rgb(0xFB, 0xBF, 0x24), // 5: Warm Amber
                Color32::from_rgb(0x22, 0xD3, 0xEE), // 6: Soft Teal/Cyan
                Color32::from_rgb(0xF4, 0x72, 0xB6), // 7: Soft Rose
                Color32::from_rgb(0x94, 0xA3, 0xB8), // 8: Cool Slate
            ],
        }
    }

    /// Clean minimalist light theme option.
    pub fn light() -> Self {
        Self {
            bg_app: Color32::from_rgb(0xF3, 0xF5, 0xF9),
            bg_top_bar: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            bg_panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            cell_hidden: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            cell_hidden_hover: Color32::from_rgb(0xEC, 0xF0, 0xFA),
            cell_revealed: Color32::from_rgb(0xE6, 0xEA, 0xF2),
            cell_exploded: Color32::from_rgb(0xFE, 0xCA, 0xCA),
            cell_wrong_flag: Color32::from_rgb(0xFD, 0xE2, 0xE4),
            cell_border_subtle: Color32::from_rgba_premultiplied(15, 23, 42, 18),
            cell_shadow: Color32::from_rgba_premultiplied(15, 23, 42, 22),
            text_primary: Color32::from_rgb(0x1E, 0x29, 0x3B),
            text_secondary: Color32::from_rgb(0x47, 0x55, 0x69),
            text_muted: Color32::from_rgb(0x94, 0xA3, 0xB8),
            accent: Color32::from_rgb(0x4F, 0x46, 0xE5),
            accent_hover: Color32::from_rgb(0x43, 0x38, 0xCA),
            accent_soft: Color32::from_rgba_premultiplied(79, 70, 229, 28),
            flag_color: Color32::from_rgb(0xE1, 0x1D, 0x48),
            flag_pole: Color32::from_rgb(0x47, 0x55, 0x69),
            mine_color: Color32::from_rgb(0x1E, 0x29, 0x3B),
            danger: Color32::from_rgb(0xE1, 0x1D, 0x48),
            success: Color32::from_rgb(0x05, 0x96, 0x69),
            number_colors: [
                Color32::from_rgb(0x25, 0x63, 0xEB), // 1: Soft Royal Blue
                Color32::from_rgb(0x05, 0x96, 0x69), // 2: Soft Sage Green
                Color32::from_rgb(0xE1, 0x1D, 0x48), // 3: Soft Crimson Rose
                Color32::from_rgb(0x7C, 0x3A, 0xED), // 4: Soft Violet
                Color32::from_rgb(0xD9, 0x77, 0x06), // 5: Soft Ochre
                Color32::from_rgb(0x08, 0x91, 0xB2), // 6: Soft Cyan
                Color32::from_rgb(0xDB, 0x27, 0x77), // 7: Soft Mulberry
                Color32::from_rgb(0x47, 0x55, 0x69), // 8: Soft Charcoal Slate
            ],
        }
    }

    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }

    /// Interpolates between two palettes with `t` in `0.0..=1.0`.
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        if t <= 0.0001 {
            return *a;
        }
        if t >= 0.9999 {
            return *b;
        }
        let mut nums = [Color32::TRANSPARENT; 8];
        for (i, slot) in nums.iter_mut().enumerate() {
            *slot = lerp_color(a.number_colors[i], b.number_colors[i], t);
        }
        Self {
            bg_app: lerp_color(a.bg_app, b.bg_app, t),
            bg_top_bar: lerp_color(a.bg_top_bar, b.bg_top_bar, t),
            bg_panel: lerp_color(a.bg_panel, b.bg_panel, t),
            cell_hidden: lerp_color(a.cell_hidden, b.cell_hidden, t),
            cell_hidden_hover: lerp_color(a.cell_hidden_hover, b.cell_hidden_hover, t),
            cell_revealed: lerp_color(a.cell_revealed, b.cell_revealed, t),
            cell_exploded: lerp_color(a.cell_exploded, b.cell_exploded, t),
            cell_wrong_flag: lerp_color(a.cell_wrong_flag, b.cell_wrong_flag, t),
            cell_border_subtle: lerp_color(a.cell_border_subtle, b.cell_border_subtle, t),
            cell_shadow: lerp_color(a.cell_shadow, b.cell_shadow, t),
            text_primary: lerp_color(a.text_primary, b.text_primary, t),
            text_secondary: lerp_color(a.text_secondary, b.text_secondary, t),
            text_muted: lerp_color(a.text_muted, b.text_muted, t),
            accent: lerp_color(a.accent, b.accent, t),
            accent_hover: lerp_color(a.accent_hover, b.accent_hover, t),
            accent_soft: lerp_color(a.accent_soft, b.accent_soft, t),
            flag_color: lerp_color(a.flag_color, b.flag_color, t),
            flag_pole: lerp_color(a.flag_pole, b.flag_pole, t),
            mine_color: lerp_color(a.mine_color, b.mine_color, t),
            danger: lerp_color(a.danger, b.danger, t),
            success: lerp_color(a.success, b.success, t),
            number_colors: nums,
        }
    }

    /// Returns the harmonious color for neighbor counts `1..=8`.
    pub fn number_color(&self, n: u8) -> Color32 {
        if (1..=8).contains(&n) {
            self.number_colors[(n - 1) as usize]
        } else {
            self.text_primary
        }
    }
}

/// Linearly interpolates two `Color32` values channel-by-channel.
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    let r = (a.r() as f32 * inv + b.r() as f32 * t).round() as u8;
    let g = (a.g() as f32 * inv + b.g() as f32 * t).round() as u8;
    let bl = (a.b() as f32 * inv + b.b() as f32 * t).round() as u8;
    let al = (a.a() as f32 * inv + b.a() as f32 * t).round() as u8;
    Color32::from_rgba_premultiplied(r, g, bl, al)
}

/// Multiplies the alpha channel of a `Color32` by `alpha_factor` (in `0.0..=1.0`).
pub fn with_alpha(c: Color32, alpha_factor: f32) -> Color32 {
    let factor = alpha_factor.clamp(0.0, 1.0);
    let [r, g, b, a] = c.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, (a as f32 * factor).round() as u8)
}
