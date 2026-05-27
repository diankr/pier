use serde::{Deserialize, Serialize};
use ratatui::style::Color;
use std::sync::OnceLock;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Theme {
  #[serde(default)]
  pub component: Component,
  #[serde(default)]
  pub selection: Selection,
  #[serde(default)]
  pub p4: P4Colors,
  #[serde(default)]
  pub icon: Icons,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
  #[serde(deserialize_with = "deserialize_color")]
  pub pane_border: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub active_pane_border: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub default_text: Color,
}

impl Default for Component {
  fn default() -> Self {
    Self {
      pane_border: hex_to_color("#928374"),
      active_pane_border: hex_to_color("#F3BB3F"),
      default_text: hex_to_color("#B2A68B"),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
  #[serde(deserialize_with = "deserialize_color")]
  pub cursor_bg: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub cursor_fg: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub cursor_inactive: Color,
}

impl Default for Selection {
  fn default() -> Self {
    Self {
      cursor_bg: hex_to_color("#FBBD2F"),
      cursor_fg: hex_to_color("#15181D"),
      cursor_inactive: hex_to_color("#B7B947"),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P4Colors {
  #[serde(deserialize_with = "deserialize_color")]
  pub add: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub edit: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub delete: Color,
  #[serde(deserialize_with = "deserialize_color")]
  pub other_checkout: Color,
}

impl Default for P4Colors {
  fn default() -> Self {
    Self {
      add: hex_to_color("#85AB77"),
      edit: hex_to_color("#7DADA2"),
      delete: hex_to_color("#DA553F"),
      other_checkout: hex_to_color("#C8899B"),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icons {
  pub own_edit: String,
  pub other_checkout: String,
  pub mark_add: String,
  pub mark_delete: String,
  pub folder: String,
  pub folder_empty: String,
  pub folder_open: String,
  pub changelist_head: String,
  pub pending_default: String,
  pub client_root: String,
  pub untracked: String,
  pub file_default: String,
}

impl Default for Icons {
  fn default() -> Self {
    Self {
      own_edit: char_from_hex("f0208").to_string(),
      other_checkout: char_from_hex("f0420").to_string(),
      mark_add: char_from_hex("f4d0").to_string(),
      mark_delete: char_from_hex("f09e7").to_string(),
      folder: char_from_hex("f024b").to_string(),
      folder_empty: char_from_hex("f114").to_string(),
      folder_open: char_from_hex("f115").to_string(),
      changelist_head: char_from_hex("f0273").to_string(),
      pending_default: char_from_hex("f14b").to_string(),
      client_root: char_from_hex("f06d9").to_string(),
      untracked: char_from_hex("f1036").to_string(),
      file_default: char_from_hex("f0214").to_string(),
    }
  }
}

fn char_from_hex(hex: &str) -> char {
  u32::from_str_radix(hex, 16)
    .ok()
    .and_then(std::char::from_u32)
    .unwrap_or('?')
}

fn hex_to_color(hex: &str) -> Color {
  let hex = hex.trim_start_matches('#');
  if let Ok(c) = u32::from_str_radix(hex, 16) {
    Color::Rgb((c >> 16) as u8, (c >> 8 & 0xFF) as u8, (c & 0xFF) as u8)
  } else {
    Color::White
  }
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  if s.starts_with('#') {
    let hex = s.trim_start_matches('#');
    if let Ok(c) = u32::from_str_radix(hex, 16) {
      return Ok(Color::Rgb((c >> 16) as u8, (c >> 8 & 0xFF) as u8, (c & 0xFF) as u8));
    }
  }
  s.parse::<Color>().map_err(serde::de::Error::custom)
}

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn theme() -> &'static Theme {
  THEME.get_or_init(load_theme)
}

fn load_theme() -> Theme {
  let mut theme = Theme::default();
  
  let theme_path = dirs::config_dir()
    .map(|d| d.join("pier/theme.toml"));

  if let Some(path) = theme_path {
    if path.exists() {
      if let Ok(content) = fs::read_to_string(path) {
        if let Ok(user_theme) = toml::from_str::<Theme>(&content) {
          theme = user_theme;
        }
      }
    }
  }
  
  theme
}
