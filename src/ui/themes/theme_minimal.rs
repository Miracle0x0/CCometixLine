use super::palette::Palette;
use crate::config::{IconConfig, SegmentConfig, SegmentId, TextStyleConfig};
use std::collections::HashMap;

pub fn model_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Model,
        enabled: true,
        icon: IconConfig {
            plain: "✽".to_string(),
            nerd_font: "\u{f2d0}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Model),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn effort_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Effort,
        enabled: true,
        icon: IconConfig {
            plain: "⚡".to_string(),
            nerd_font: "\u{f0e7}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Effort),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn directory_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Directory,
        enabled: true,
        icon: IconConfig {
            plain: "◐".to_string(),
            nerd_font: "\u{f024b}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Directory),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn git_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Git,
        enabled: true,
        icon: IconConfig {
            plain: "※".to_string(),
            nerd_font: "\u{f02a2}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Git),
        styles: TextStyleConfig::default(),
        options: {
            let mut opts = HashMap::new();
            opts.insert("show_sha".to_string(), serde_json::Value::Bool(false));
            opts
        },
    }
}

pub fn context_window_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::ContextWindow,
        enabled: true,
        icon: IconConfig {
            plain: "◐".to_string(),
            nerd_font: "\u{f49b}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::ContextWindow),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn cost_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Cost,
        enabled: false,
        icon: IconConfig {
            plain: "💰".to_string(),
            nerd_font: "\u{eec1}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Cost),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn session_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Session,
        enabled: false,
        icon: IconConfig {
            plain: "⏱️".to_string(),
            nerd_font: "\u{f19bb}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Session),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn output_style_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::OutputStyle,
        enabled: false,
        icon: IconConfig {
            plain: "🎯".to_string(),
            nerd_font: "\u{f12f5}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::OutputStyle),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn usage_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Usage,
        enabled: false,
        icon: IconConfig {
            plain: "📊".to_string(),
            nerd_font: "\u{f0a9e}".to_string(),
        },
        colors: Palette::Minimal.colors(SegmentId::Usage),
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}
