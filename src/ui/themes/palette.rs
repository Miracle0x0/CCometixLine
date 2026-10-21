use crate::config::{AnsiColor, ColorConfig, SegmentId};

/// Built-in segment accents. Saved configurations never pass through this palette.
#[derive(Clone, Copy)]
pub(super) enum Palette {
    Default,
    Minimal,
    Gruvbox,
    Nord,
    PowerlineDark,
    PowerlineLight,
    RosePine,
    TokyoNight,
}

impl Palette {
    pub(super) fn colors(self, id: SegmentId) -> ColorConfig {
        // Model, effort, agents, directory, git, context, cache, usage, cost, duration,
        // output style, session name, worktree, pull request, update.
        let accents = match self {
            Self::Default => [
                0x67d4e8, 0xbd93f9, 0xffad66, 0x90d878, 0x6ca8ff, 0xf58bd5, 0x53cfb2, 0xff808b,
                0xe8c75a, 0xa5b5c5, 0xa8a4f4, 0xe6d9bb, 0xc5d76a, 0xd977b4, 0xe99877,
            ],
            Self::Minimal => [
                0x8ecad4, 0xb7a4da, 0xd6aa80, 0x9fbe8c, 0x8baad4, 0xdba9c7, 0x83b9aa, 0xd98f94,
                0xcbb781, 0x9ca7b6, 0xa6a4cd, 0xd3cabb, 0xb6bf83, 0xc58eaf, 0xcfa48f,
            ],
            Self::Gruvbox => [
                0x83c7be, 0xd386b9, 0xfea054, 0xb8bb26, 0x83a8ba, 0xf3a4bb, 0x8ec07c, 0xfb7c68,
                0xfabd2f, 0xa89984, 0xb6a3cb, 0xebdbb2, 0xd2d680, 0xc68f83, 0xd5b28c,
            ],
            Self::Nord => [
                0x88c0d0, 0xc4a5d8, 0xdca77b, 0xa3be8c, 0x96b6de, 0xd7a1bb, 0x8fbcbb, 0xee9ba4,
                0xebcb8b, 0xb0bac8, 0xb5b8de, 0xe5e9f0, 0xc5ca90, 0xec98cb, 0xe0b7a5,
            ],
            Self::PowerlineDark => [
                0x62cde0, 0xc29aff, 0xffaf70, 0x93d17c, 0x79aaff, 0xf29acf, 0x58c7b0, 0xf28688,
                0xe8c66a, 0xa7b4c7, 0xa6a4ef, 0xe6d7ba, 0xc3d279, 0xd784b8, 0xdda183,
            ],
            Self::PowerlineLight => [
                0x006479, 0x7044a8, 0xa14b0b, 0x3b7029, 0x285daa, 0xa3317b, 0x006b57, 0xb02e43,
                0x82610c, 0x536278, 0x5653a6, 0x71543a, 0x606d13, 0x81395e, 0x8e4938,
            ],
            Self::RosePine => [
                0x9ccfd8, 0xc4a7e7, 0xeaab86, 0xa6c89a, 0x91b4db, 0xeab8d7, 0x83c7b2, 0xeb8a9b,
                0xf6c177, 0xa5a1bb, 0xb2b5e8, 0xe0def4, 0xc6cc95, 0xd58db6, 0xd6b4a3,
            ],
            Self::TokyoNight => [
                0x7dcfff, 0xbb9af7, 0xffa86c, 0x9ece6a, 0x7aa2f7, 0xf0a0d4, 0x73daca, 0xf7768e,
                0xe0af68, 0x9aa5ce, 0xa0a7f7, 0xc0caf5, 0xc3d77a, 0xcc85c6, 0xdca994,
            ],
        };
        let index = match id {
            SegmentId::Model => 0,
            SegmentId::Effort => 1,
            SegmentId::Agents => 2,
            SegmentId::Directory => 3,
            SegmentId::Git => 4,
            SegmentId::ContextWindow => 5,
            SegmentId::PromptCache => 6,
            SegmentId::Usage => 7,
            SegmentId::Cost => 8,
            SegmentId::Session => 9,
            SegmentId::OutputStyle => 10,
            SegmentId::SessionName => 11,
            SegmentId::Worktree => 12,
            SegmentId::PullRequest => 13,
            SegmentId::Update => 14,
        };
        let accent = accents[index];
        let foreground = rgb(accent);
        ColorConfig {
            icon: Some(foreground.clone()),
            text: Some(foreground),
            background: self.surface().map(|base| rgb(tint(base, accent))),
        }
    }

    fn surface(self) -> Option<u32> {
        match self {
            Self::Default | Self::Minimal | Self::Gruvbox => None,
            Self::Nord => Some(0x2e3440),
            Self::PowerlineDark => Some(0x171b22),
            Self::PowerlineLight => Some(0xfafafa),
            Self::RosePine => Some(0x191724),
            Self::TokyoNight => Some(0x1a1b26),
        }
    }
}

fn rgb(hex: u32) -> AnsiColor {
    AnsiColor::Rgb {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
    }
}

/// Keep the theme's surface while giving every segment its own subtle background tint.
fn tint(base: u32, accent: u32) -> u32 {
    [16, 8, 0].into_iter().fold(0, |color, shift| {
        let channel = (((base >> shift) & 255) * 88 + ((accent >> shift) & 255) * 12) / 100;
        color | (channel << shift)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::themes::ThemePresets;
    use std::collections::HashSet;

    fn luminance(color: &AnsiColor) -> f64 {
        let AnsiColor::Rgb { r, g, b } = *color else {
            panic!("built-in palette must use RGB")
        };
        let channels = [r, g, b].map(|c| {
            let value = f64::from(c) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
    }

    #[test]
    fn every_theme_has_unique_accents_and_readable_text_on_its_surface() {
        for name in ThemePresets::BUILTIN_THEMES {
            let config = ThemePresets::builtin(name).unwrap();
            let mut accents = HashSet::new();
            for segment in config.segments {
                let text = segment.colors.text.as_ref().unwrap();
                assert!(
                    accents.insert(serde_json::to_string(text).unwrap()),
                    "duplicate accent: {name} {:?}",
                    segment.id
                );
                assert_eq!(segment.colors.icon.as_ref(), Some(text));
                // Unfilled terminal themes are designed against a dark terminal background.
                let background = segment.colors.background.unwrap_or(rgb(0x1e1e2e));
                let (fg, bg) = (luminance(text), luminance(&background));
                let ratio = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
                assert!(ratio >= 4.5, "{name} {:?}: contrast {ratio:.2}", segment.id);
            }
        }
    }
}
