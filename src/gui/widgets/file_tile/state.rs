use std::{fmt, future::Future, io::Cursor, pin::Pin, sync::LazyLock, task::{Context, Poll, Waker}};

use camino::Utf8PathBuf;
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
    // state needs this to diff
    pub preview_path: Option<Utf8PathBuf>,
    pub image_state_machine: LoadingImageStateMachine,
}

impl State {
    pub fn new(preview_path: Option<Utf8PathBuf>) -> Self {
        Self {
            mouseover: false,
            mouseclick: false,
            disabled: false,
            // clone because this is a future state machine and may need to go across thread boundaries
            // TODO: maybe deal with future scope and make it a lifetime??
            image_state_machine: LoadingImageStateMachine::new(preview_path.clone()),
            preview_path,
        }
    }
}

pub enum LoadingImageStateMachine {
    NotStarted,
    Loading(Pin<Box<dyn Future<Output = Result<DynamicImage, anyhow::Error>>>>),
    Error(anyhow::Error),
    Completed(DynamicImage),
}

impl LoadingImageStateMachine {
    pub fn new(preview_path: Option<Utf8PathBuf>) -> Self {
        if let Some(path) = preview_path {
            let mut obj = LoadingImageStateMachine::Loading(Box::pin(async move {
                match image::open(path) {
                    Ok(image) => Ok(image),
                    Err(e) => Err(e.into()),
                }
            }));
            // call update to start the future.
            obj.update();
            obj
        } else {
            LoadingImageStateMachine::NotStarted
        }
    }

    // remember that futures do nothing until they are polled. this fn must be called in order to start loading the image.
    pub fn update(&mut self) {
        match self {
            LoadingImageStateMachine::Loading(pinned_future) => {
                println!("poll called");
                if let Poll::Ready(res) = pinned_future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
                    // If the future is ready, we can update ourself
                    println!("poll finished");
                    if let Ok(image) = res {
                        *self = LoadingImageStateMachine::Completed(image);
                    } else {
                        *self = LoadingImageStateMachine::Error(anyhow::anyhow!("Failed to load image"));
                    }
                } else { // else we remain in the Loading state
                    println!("poll still loading");
                }
            }
            _ => {} // this function is a no-op for all other states
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading(_))
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Error(_))
    }

    pub fn image_or_default(&self) -> &DynamicImage {
        match self {
            LoadingImageStateMachine::Completed(dynamic_image) => dynamic_image,
            _ => &PLACEHOLDER_IMAGE,
        }
    }
}

impl fmt::Debug for LoadingImageStateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
            Self::Loading(_arg0) => f.debug_tuple("Loading").field(&"a pinned boxed future").finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
            Self::Completed(arg0) => f.debug_tuple("Completed").field(&"a loaded in-memory image").finish(),
        }
    }
}