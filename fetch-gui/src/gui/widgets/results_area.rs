use std::sync::LazyLock;

use camino::Utf8PathBuf;
use iced::{widget::{column, container, image::Handle, row, scrollable, text}, window, Element, Length, Pixels, Subscription};

use crate::gui::{widgets::file_tile::FileTile, SINGLE_PAD};

const PLACEHOLDER_IMAGE_BYTES: &[u8] = include_bytes!("../../../artifacts/placeholder.png");
const TILE_WIDTH: Pixels = Pixels(200.0);
const TILE_HEIGHT: Pixels = Pixels(150.0);

static PLACEHOLDER_IMAGE: LazyLock<Handle> = LazyLock::new(|| Handle::from_bytes(PLACEHOLDER_IMAGE_BYTES));

#[derive(Clone, Debug, Default)]
pub struct ResultsArea {
    results: Vec<FileWithHandle>,
    selected_index: Option<u16>,
    area_width: Pixels,
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
    WidthResized(Pixels),
    ArrowKeyReleased(ArrowDirection),
}

#[derive(Clone, Copy, Debug)]
pub enum ArrowDirection {
    Left,
    Right,
    Up,
    Down,
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
            },
            Message::WidthResized(new_width) => {
                self.area_width = Pixels(new_width.0 - (10.0 + SINGLE_PAD.0 * 2.0)); // 10 for scrollbar, 2 for padding
                Action::None
            },
            Message::ArrowKeyReleased(direction) => {
                if self.results.is_empty() {
                    return Action::None;
                }
                // if nothing was selected previously, assume we start at the first item
                let current_index = self.selected_index.unwrap_or(0) as u16;
                let grid = layout_tile_grid(self.results.len(), self.area_width);
                let num_columns = grid[0].len() as u16;
                let mut row_idx = current_index / num_columns;
                let mut col_idx = current_index % num_columns;

                match direction {
                    ArrowDirection::Left => {
                        if col_idx > 0 {
                            col_idx -= 1;
                        }
                    },
                    ArrowDirection::Right => {
                        if col_idx < (num_columns - 1) {
                            col_idx += 1;
                        }
                    },
                    ArrowDirection::Up => {
                        if row_idx > 0 {
                            row_idx -= 1;
                        }
                    },
                    ArrowDirection::Down => {
                        if row_idx < (grid.len() as u16 - 1) {
                            row_idx += 1;
                        }
                    },
                }

                let new_index = grid[row_idx as usize][col_idx as usize];
                if new_index != -1 {
                    self.selected_index = Some(new_index as u16);
                }

                Action::None
            },
        }

    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.results.is_empty() {
            return iced::widget::text("No results to display").width(Length::Fill).height(Length::Fill).center().into();
        }

        // This is layout logic. there is probably a better way to do this, but i think it is not clear right now with how
        // iced has things setup
        let grid = layout_tile_grid(self.results.len(), self.area_width);
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

        scrollable(column(rows).width(Length::Fill).spacing(SINGLE_PAD))
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(SINGLE_PAD)
            .style(|theme, status| {
                let mut style = scrollable::default(theme, status);
                style.container = container::bordered_box(theme);
                style
            })
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
            HandleOrBroken::Handle(handle) => FileTile::new(file_name, handle, selected, TILE_WIDTH.into(), TILE_HEIGHT.into()),
            // TODO: replace with broken preview image
            HandleOrBroken::Broken => FileTile::new(file_name, &PLACEHOLDER_IMAGE, selected, TILE_WIDTH.into(), TILE_HEIGHT.into()),
        }
    } else {
        FileTile::new(file_name, &PLACEHOLDER_IMAGE, selected, TILE_WIDTH.into(), TILE_HEIGHT.into())
    };
    
    tile
        .on_click(move || Message::ResultSelected(index))
        .on_double_click(move || Message::FileOpened(path.clone()))
        .into()
}

// Will always return at least a 1x1 grid for num_items > 0
fn layout_tile_grid(num_items: usize, cont_width: Pixels) -> Vec<Vec<i16>> {
    // first take away | PAD TILE PAD |, not allowing negatives
    let remaining_space_for_repeats = std::cmp::max_by(cont_width.0 - (SINGLE_PAD.0 * 2.0) - TILE_WIDTH.0, 0.0, f32::total_cmp);
    // then all the remaining repetitions look like ...TILE PAD...
    let n_width = (remaining_space_for_repeats / (TILE_WIDTH.0 + SINGLE_PAD.0)) as usize + 1;
    let n_height = (num_items as f32 / n_width as f32).ceil() as usize;
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