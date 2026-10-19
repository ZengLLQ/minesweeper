use crate::audio::sanitize_volume;
use crate::game::{Difficulty, MAX_CUSTOM_HEIGHT, MAX_CUSTOM_WIDTH, MIN_CUSTOM_DIM};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Visual theme selection. Dark is default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Best completion times (in seconds) per preset difficulty.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BestTimes {
    pub beginner: Option<f64>,
    pub intermediate: Option<f64>,
    pub expert: Option<f64>,
}

impl BestTimes {
    /// Returns the recorded best time for the given difficulty, if any.
    pub fn get(&self, difficulty: Difficulty) -> Option<f64> {
        match difficulty {
            Difficulty::Beginner => self.beginner,
            Difficulty::Intermediate => self.intermediate,
            Difficulty::Expert => self.expert,
            Difficulty::Custom { .. } => None,
        }
    }

    /// Updates the best time if `elapsed_secs` is faster than the previous record.
    /// Returns `true` if a new record was set.
    pub fn record(&mut self, difficulty: Difficulty, elapsed_secs: f64) -> bool {
        if !elapsed_secs.is_finite() || elapsed_secs <= 0.0 {
            return false;
        }
        let slot = match difficulty {
            Difficulty::Beginner => &mut self.beginner,
            Difficulty::Intermediate => &mut self.intermediate,
            Difficulty::Expert => &mut self.expert,
            Difficulty::Custom { .. } => return false,
        };

        match *slot {
            Some(prev) if elapsed_secs >= prev => false,
            _ => {
                *slot = Some(elapsed_secs);
                true
            }
        }
    }

    fn sanitize(&mut self) {
        for slot in [&mut self.beginner, &mut self.intermediate, &mut self.expert] {
            if let Some(val) = *slot {
                if !val.is_finite() || val <= 0.0 {
                    *slot = None;
                }
            }
        }
    }
}

/// Persistent application state (settings and best times).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistentData {
    pub theme: ThemeMode,
    pub reduce_motion: bool,
    pub sound_enabled: bool,
    pub sound_volume: f32,
    pub difficulty: Difficulty,
    pub custom_width: usize,
    pub custom_height: usize,
    pub custom_mines: usize,
    pub best_times: BestTimes,
}

impl Default for PersistentData {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            reduce_motion: false,
            sound_enabled: true,
            sound_volume: 0.75,
            difficulty: Difficulty::Beginner,
            custom_width: 12,
            custom_height: 12,
            custom_mines: 20,
            best_times: BestTimes::default(),
        }
    }
}

impl PersistentData {
    /// Clamps custom dimensions/mines, audio volume, and validates best times after loading from disk.
    pub fn sanitized(mut self) -> Self {
        self.sound_volume = sanitize_volume(self.sound_volume);
        self.custom_width = self.custom_width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_WIDTH);
        self.custom_height = self.custom_height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_HEIGHT);
        let max_custom_mines = (self.custom_width * self.custom_height)
            .saturating_sub(9)
            .max(1);
        self.custom_mines = self.custom_mines.clamp(1, max_custom_mines);

        if let Difficulty::Custom {
            width,
            height,
            mines,
        } = self.difficulty
        {
            let w = width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_WIDTH);
            let h = height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_HEIGHT);
            let max_m = (w * h).saturating_sub(9).max(1);
            let m = mines.clamp(1, max_m);
            self.difficulty = Difficulty::Custom {
                width: w,
                height: h,
                mines: m,
            };
        }

        self.best_times.sanitize();
        self
    }

    /// Returns the default path for saving persistent game data.
    pub fn default_save_path() -> PathBuf {
        if let Some(mut dir) = dirs::config_dir() {
            dir.push("minesweeper_egui");
            dir.push("settings_and_scores.json");
            dir
        } else {
            PathBuf::from("minesweeper_settings.json")
        }
    }

    /// Loads persistent data from the default path, falling back to `Default` if unavailable.
    pub fn load() -> Self {
        let path = Self::default_save_path();
        Self::load_from_path(&path).unwrap_or_default()
    }

    /// Saves persistent data to the default path.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::default_save_path();
        self.save_to_path(&path)
    }

    /// Loads persistent data from a specific file path and sanitizes out-of-range values.
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let parsed: Self = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(parsed.sanitized())
    }

    /// Saves persistent data as formatted JSON to a specific file path.
    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_best_times_recording() {
        let mut times = BestTimes::default();
        assert_eq!(times.get(Difficulty::Beginner), None);

        assert!(times.record(Difficulty::Beginner, 42.5));
        assert_eq!(times.get(Difficulty::Beginner), Some(42.5));

        // Slower time should not overwrite
        assert!(!times.record(Difficulty::Beginner, 50.0));
        assert_eq!(times.get(Difficulty::Beginner), Some(42.5));

        // Faster time should overwrite
        assert!(times.record(Difficulty::Beginner, 31.2));
        assert_eq!(times.get(Difficulty::Beginner), Some(31.2));

        // Custom difficulty does not record preset best time
        assert!(!times.record(
            Difficulty::Custom {
                width: 10,
                height: 10,
                mines: 10
            },
            10.0
        ));
    }

    #[test]
    fn test_serialization_roundtrip_and_sanitization() {
        let mut data = PersistentData {
            theme: ThemeMode::Light,
            reduce_motion: true,
            sound_enabled: false,
            sound_volume: 0.6,
            difficulty: Difficulty::Expert,
            custom_width: 20,
            custom_height: 15,
            custom_mines: 45,
            best_times: BestTimes::default(),
        };
        data.best_times.record(Difficulty::Expert, 118.4);

        let temp_path = std::env::temp_dir().join("minesweeper_test_roundtrip.json");
        data.save_to_path(&temp_path).expect("save should succeed");

        let loaded = PersistentData::load_from_path(&temp_path).expect("load should succeed");
        let _ = fs::remove_file(&temp_path);

        assert_eq!(loaded.theme, ThemeMode::Light);
        assert!(loaded.reduce_motion);
        assert!(!loaded.sound_enabled);
        assert!((loaded.sound_volume - 0.6).abs() < 1e-6);
        assert_eq!(loaded.difficulty, Difficulty::Expert);
        assert_eq!(loaded.custom_width, 20);
        assert_eq!(loaded.best_times.get(Difficulty::Expert), Some(118.4));

        // Out-of-range custom values, invalid sound volume, and invalid best times are sanitized safely
        let unsanitized = PersistentData {
            theme: ThemeMode::Dark,
            reduce_motion: false,
            sound_enabled: true,
            sound_volume: 4.5,
            difficulty: Difficulty::Custom {
                width: 9999,
                height: 0,
                mines: 999999,
            },
            custom_width: 1,
            custom_height: 500,
            custom_mines: 0,
            best_times: BestTimes {
                beginner: Some(-5.0),
                intermediate: Some(0.0),
                expert: Some(95.0),
            },
        }
        .sanitized();

        assert_eq!(
            unsanitized.difficulty,
            Difficulty::Custom {
                width: MAX_CUSTOM_WIDTH,
                height: MIN_CUSTOM_DIM,
                mines: MAX_CUSTOM_WIDTH * MIN_CUSTOM_DIM - 9,
            }
        );
        assert!((unsanitized.sound_volume - 1.0).abs() < 1e-6);
        assert_eq!(unsanitized.custom_width, MIN_CUSTOM_DIM);
        assert_eq!(unsanitized.custom_height, MAX_CUSTOM_HEIGHT);
        assert_eq!(unsanitized.custom_mines, 1);
        assert_eq!(unsanitized.best_times.beginner, None);
        assert_eq!(unsanitized.best_times.intermediate, None);
        assert_eq!(unsanitized.best_times.expert, Some(95.0));
    }
}
