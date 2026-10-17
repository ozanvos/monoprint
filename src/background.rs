use crate::{Appearance, Error, InvalidOption, assets};
use std::{fmt, str::FromStr};

/// Decoration outside the code window. All modes retain the theme's panel fill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Background {
    /// The preset's original artwork or matching gradient.
    #[default]
    Theme,
    /// A flat color chosen to complement the palette.
    Solid,
    /// A fine square grid over the matching solid color.
    Lattice,
    /// A quiet dot pattern over the matching solid color.
    Halftone,
    /// Transparent surroundings, preserving the code window.
    None,
}
impl Background {
    /// All available treatments, in display order.
    pub const ALL: [Self; 5] = [
        Self::Theme,
        Self::Solid,
        Self::Lattice,
        Self::Halftone,
        Self::None,
    ];
    /// Stable lowercase identifier used by the CLI and bindings.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Solid => "solid",
            Self::Lattice => "lattice",
            Self::Halftone => "halftone",
            Self::None => "none",
        }
    }
}
impl fmt::Display for Background {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for Background {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self, Error> {
        Self::ALL
            .into_iter()
            .find(|b| b.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| InvalidOption::Background(value.into()).into())
    }
}

pub(crate) fn css(theme: &assets::Theme, appearance: Appearance, background: Background) -> String {
    let gradient = theme.background.as_ref();
    let solid = theme
        .editor
        .as_ref()
        .map(|e| e.solid.as_str())
        .or_else(|| {
            theme.syntax[appearance.as_str()]
                .get("--monoprint-stage")
                .map(String::as_str)
        })
        // These frame presets have no separate code-panel fill. Their manifest
        // gradient describes only the dark canvas; light mode paints white
        // through frame CSS instead. Custom backdrops replace that frame CSS,
        // so retain its light surface or the dark text becomes unreadable.
        .or_else(|| {
            (appearance == Appearance::Light
                && matches!(
                    theme.id.as_str(),
                    "removed-frame" | "removed-frame" | "removed-frame" | "removed-frame" | "removed-frame"
                ))
            .then_some("#ffffff")
        })
        .or_else(|| gradient.map(|g| g.from.as_str()))
        .unwrap_or("#242424");
    let ink = if appearance == Appearance::Light {
        "#00000016"
    } else {
        "#ffffff18"
    };
    let declaration = match background {
        Background::Theme if theme.editor.is_some() => {
            let g = gradient.expect("editor companion gradient");
            format!(
                "background-image:linear-gradient(135deg,{},{});background-color:{solid}",
                g.from, g.to
            )
        }
        Background::Theme | Background::None => return String::new(),
        Background::Solid => format!("background-color:{solid}"),
        Background::Lattice => format!(
            "background-color:{solid};background-image:linear-gradient({ink} 1px,transparent 1px),linear-gradient(90deg,{ink} 1px,transparent 1px);background-size:24px 24px;background-repeat:repeat"
        ),
        Background::Halftone => format!(
            "background-color:{solid};background-image:radial-gradient(circle at 1px 1px,{ink} 1px,transparent 1.25px);background-size:8px 8px;background-repeat:repeat"
        ),
    };
    format!("#capture{{{declaration}}}")
}
