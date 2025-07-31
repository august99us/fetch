use std::{io::Cursor, sync::LazyLock};

use camino::{Utf8Path, Utf8PathBuf};
use image::{DynamicImage, ImageReader};

const PLACEHOLDER_IMAGE_BYTES: &[u8] = include_bytes!("../../../../artifacts/placeholder.png");
static PLACEHOLDER_IMAGE: LazyLock<DynamicImage> = LazyLock::new(|| {
    ImageReader::new(Cursor::new(PLACEHOLDER_IMAGE_BYTES)).with_guessed_format()
    .expect("constant image bytes included with the binary should always have correct format")
    .decode()
    .expect("constant image bytes included with the binary should always succeed decoding operation")
});

pub struct State {
    pub mouseover: bool,
    pub mouseclick: bool,
    pub disabled: bool,
    pub thumbnail: ThumbnailImage,
}

impl State {
    pub fn new(preview_path: Option<Utf8PathBuf>) -> Self {
        Self {
            mouseover: false,
            mouseclick: false,
            disabled: false,
            thumbnail: ThumbnailImage::new(preview_path),
        }
    }
}

pub enum ThumbnailImage {
    Preview(Utf8PathBuf, DynamicImage),
    Default,
    Broken(Utf8PathBuf),
}

impl ThumbnailImage {
    pub fn new(path: Option<Utf8PathBuf>) -> Self {
        match path {
            Some(p) => match image::open(&p) {
                    Ok(i) => ThumbnailImage::Preview(p, i),
                    Err(_) => ThumbnailImage::Broken(p),
                },
            None => ThumbnailImage::Default,
        }
    }

    pub fn get_image(&self) -> &DynamicImage {
        match self {
            Self::Preview(_, image) => image,
            Self::Default => &PLACEHOLDER_IMAGE,
            Self::Broken(_) => &PLACEHOLDER_IMAGE,
        }
    }

    pub fn get_preview_path(&self) -> Option<&Utf8Path> {
        match self {
            Self::Preview(path, _) => Some(path),
            Self::Default => None,
            Self::Broken(path) => Some(path),
        }
    }
}

// Equivalence for Preview variant of ThumbnailImage only checks the path member. All others
// work as expected.
impl PartialEq for ThumbnailImage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Preview(l0, _l1), Self::Preview(r0, _r1)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}