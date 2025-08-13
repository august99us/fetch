// Styling functions and traits

use iced::{Background, Border, Color, Theme};

use super::state::Status;

type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, &Status) -> Style + 'a>;

/// The style of a FileTile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the tile.
    pub background: Background,
    /// The text [`Color`] of the tile.
    pub text_color: Color,
    /// The [`Border`] of the tile.
    pub border: Border,
}

/// Theme catalog for a FileTile
pub trait Catalog: Sized {
    /// The item class of this [`Catalog`].
    type Class<'a>;

    /// The default class produced by this [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, item: &Self::Class<'_>, status: &Status) -> Style;
}

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(style)
    }

    fn style(&self, class: &Self::Class<'_>, status: &Status) -> Style {
        class(self, status)
    }
}

pub fn style(theme: &Theme, status: &Status) -> Style {
    let palette = theme.extended_palette();

    match status {
        Status::Default => Style {
            background: palette.background.base.color.into(),
            text_color: palette.background.base.text.into(),
            border: border(palette.background.weak.color.into()),
        },
        Status::Hovered => Style {
            background: palette.background.strong.color.into(),
            text_color: palette.background.strong.text.into(),
            border: border(palette.background.strongest.color.into()),
        },
        Status::Selected => Style {
            background: palette.background.strongest.color.into(),
            text_color: palette.background.strongest.text.into(),
            border: border(palette.primary.weak.color.into()),
        },
    }
}

fn border(color: Color) -> Border {
    Border {
        color,
        width: 1.0,
        radius: (0.0).into(),
    }
}