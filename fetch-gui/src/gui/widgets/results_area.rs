use std::sync::LazyLock;

use camino::Utf8PathBuf;
use iced::{widget::{column, container, image::Handle, row, text}, Element, Length, Pixels};

use crate::gui::{widgets::file_tile::FileTile, SINGLE_PAD};

const PLACEHOLDER_IMAGE_BYTES: &[u8] = include_bytes!("../../../artifacts/placeholder.png");
const TILE_WIDTH: Pixels = Pixels(200.0);
const TILE_HEIGHT: Pixels = Pixels(150.0);

static PLACEHOLDER_IMAGE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(PLACEHOLDER_IMAGE_BYTES));

#[derive(Clone, Debug, Default)]
pub struct ResultsArea {
    results: Vec<FileWithHandle>,
    selected_index: Option<u16>,
}

pub struct LoadPreviewRequest {
    pub index: u16,
    pub path: Utf8PathBuf,
}

pub enum Action {
    LoadPreviews(Vec<LoadPreviewRequest>),
    OpenFile(Utf8PathBuf),
    OpenFileLocation(Utf8PathBuf),
    None,
}

#[derive(Clone, Debug)]
pub enum Message {
    UpdateResults(Vec<Utf8PathBuf>),
    UpdatePreview { index: u16, path: Utf8PathBuf, handle_result: Result<Handle, String> },
    ResultSelected(u16),
    FileOpened(Utf8PathBuf),
    FileLocationOpened(Utf8PathBuf),
}

impl ResultsArea {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::UpdateResults(results) => {
                self.selected_index = None;
                self.results = results.into_iter()
                    .map(|p| FileWithHandle { path: p, preview: None })
                    .collect();

                if self.results.is_empty() {
                    return Action::None;
                }

                let mut requests = Vec::with_capacity(self.results.len());
                for (i, fwp) in self.results.iter().enumerate() {
                    requests.push(LoadPreviewRequest { index: i as u16, path: fwp.path.clone() })
                }

                Action::LoadPreviews(requests)
            },
            Message::UpdatePreview { index, path, handle_result } => {
                let ofwp = self.results.get_mut(index as usize);

                if ofwp.as_ref().is_some_and(|fwp| fwp.path == path) {
                    ofwp.unwrap().preview = Some(handle_result.into());
                } else {
                    println!("Warning: received update preview message but state at index {index} either 
                        does not exist or does not match given path {path}. Dropping message");
                }

                Action::None
            },
            Message::ResultSelected(index) => {
                self.selected_index = Some(index);
                Action::None
            },
            Message::FileOpened(path) => {
                Action::OpenFile(path)
            },
            Message::FileLocationOpened(path) => {
                Action::OpenFileLocation(path)
            }
        }

    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.results.is_empty() {
            return iced::widget::text("No results to display").width(Length::Fill).height(Length::Fill).center().into();
        }
        // Temporary 5x3
        let grid = layout_tile_grid(self.results.len(), (TILE_WIDTH * 5.0, TILE_HEIGHT * 4.0));
        let rows: Vec<Element<'_, Message>> = grid.into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|index| {
                        if index >= 0 {
                            let idx = index as u16;
                            let selected = self.selected_index == Some(idx);
                            file_tile(&self.results[index as usize], idx, selected)
                        } else {
                            text("").into()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .map(|row_elements| row(row_elements).spacing(SINGLE_PAD).into())
            .collect::<Vec<_>>();

        container(column(rows).spacing(SINGLE_PAD))
            .clip(true)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(SINGLE_PAD)
            .style(container::bordered_box)
            .into()
    }
}

// Private methods and structs
#[derive(Clone, Debug)]
struct FileWithHandle {
    path: Utf8PathBuf,
    preview: Option<HandleOrBroken>,
}

#[derive(Clone, Debug)]
enum HandleOrBroken {
    Handle(Handle),
    Broken,
}

impl<E> From<Result<Handle, E>> for HandleOrBroken {
    fn from(value: Result<Handle, E>) -> Self {
        match value {
            Ok(h) => HandleOrBroken::Handle(h),
            Err(_) => HandleOrBroken::Broken,
        }
    }
}

fn file_tile<'a>(item: &'a FileWithHandle, index: u16, selected: bool) -> Element<'a, Message> {
    let path = &item.path;
    let file_name = path.file_name().unwrap_or("<invalid filename>").to_string();
    
    let tile = if let Some(handle) = &item.preview {
        match handle {
            HandleOrBroken::Handle(handle) => FileTile::new(file_name, handle, selected),
            // TODO: replace with broken preview image
            HandleOrBroken::Broken => FileTile::new(file_name, &PLACEHOLDER_IMAGE, selected),
        }
    } else {
        FileTile::new(file_name, &PLACEHOLDER_IMAGE, selected)
    };
    
    tile.on_click(move || Message::ResultSelected(index))
        .on_double_click(move || Message::FileOpened(path.clone()))
        .into()
}

fn layout_tile_grid(num_items: usize, cont_size: (Pixels, Pixels)) -> Vec<Vec<i16>> {
    let n_width = (cont_size.0 / TILE_WIDTH).0 as usize;
    let n_height = (cont_size.1 / TILE_HEIGHT).0 as usize;
    let mut grid = vec![vec![0; n_width]; n_height];

    let mut index = 0;
    #[allow(clippy::needless_range_loop)]
    for i in 0..n_height {
        for j in 0..n_width {
            if index < num_items {
                grid[i][j] = index as i16;
                index += 1;
            } else {
                grid[i][j] = -1; // Initialize with -1 to indicate empty
            }
        }
    }

    grid
}