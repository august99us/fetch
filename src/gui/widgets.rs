use iced::{alignment::Horizontal, widget::{column, container, image::Handle, mouse_area, row, text, Image}, Element, Length};

use crate::gui::{widgets::file_tile::FileTile, FileWithPreview, LandingMessage, SINGLE_PAD};

const PLACEHOLDER_IMAGE: &[u8] = include_bytes!("../../artifacts/placeholder.png");
const TILE_WIDTH: u16 = 200;
const TILE_HEIGHT: u16 = 150;

pub fn file_tile(item: Option<&FileWithPreview>) -> Element<'_, LandingMessage> {
    let content: Element<'_, LandingMessage>;
    match item {
        Some(file) => {
            let preview_image: Image<Handle>;
            match &file.preview {
                Some(preview_path) => {
                    preview_image = iced::widget::image(Handle::from_path(preview_path)).height(Length::Fill);
                }
                None => {
                    // If no preview is available, just show the placeholder image
                    preview_image = iced::widget::image(Handle::from_bytes(PLACEHOLDER_IMAGE)).height(Length::Fill);
                }
            }
            let file_name = file.path.file_name().unwrap_or("<Invalid Name>");

            content = column![preview_image, text(file_name.to_string())]
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(SINGLE_PAD).into();
        }
        None => {
            content = text("").into();
        }
    }

    let inner_container = container(content).width(TILE_WIDTH).height(TILE_HEIGHT).style(container::bordered_box);

    match item {
        Some(file) => {
            mouse_area(inner_container)
                .on_release(LandingMessage::FileClicked(file.path.clone()))
                .into()
        },
        None => {
            // If no item is provided, just return a mouse area with no action
            mouse_area(inner_container).into()
        },
    }
}

fn file_tile_custom(item: Option<&FileWithPreview>) -> Element<'_, LandingMessage> {
    match item {
        Some(fwp) => FileTile::new(fwp).into(),
        None => text("").into()
    }
}

// TODO: Perhaps this would be better as a custom widget?
// as of now there does not seem to be a way to determine the actual size of the
// container when drawn (only Length::Fill size hints etc) so there is no way to
// figure out the size of the grid. Probably this needs to happen during the draw
// operation for a custom widget. currently this lays things out in a fixed 5x3 grid
pub fn results_area(files: &Option<Vec<FileWithPreview>>) -> Element<'_, LandingMessage> {
    let child_element: Element<'_, LandingMessage>;
    if let Some(files) = files {
        if files.is_empty() {
            return iced::widget::text("No results found").into();
        }
        // Temporary 5x3
        let grid = layout_tile_grid(files.len(), (TILE_WIDTH * 5, TILE_HEIGHT * 4));
        let rows: Vec<Element<'_, LandingMessage>> = grid.into_iter()
            .map(|row| {
                row.into_iter()
                    .filter_map(|index| {
                        if index >= 0 {
                            file_tile_custom(Some(&files[index as usize])).into()
                        } else {
                            file_tile_custom(None).into()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .map(|row_elements| row(row_elements).spacing(SINGLE_PAD).into())
            .collect::<Vec<_>>();
        child_element = column(rows).spacing(SINGLE_PAD).into();
    } else {
        child_element = text("No results to display").center().into();
    }

    container(child_element)
        .clip(true)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(SINGLE_PAD)
        .style(container::bordered_box)
        .into()
}

fn layout_tile_grid(num_items: usize, cont_size: (u16, u16)) -> Vec<Vec<i16>> {
    let n_width = cont_size.0 / TILE_WIDTH;
    let n_height = cont_size.1 / TILE_HEIGHT;
    let mut grid = vec![vec![0; n_width as usize]; n_height as usize];

    let mut index = 0;
    for i in 0..n_height {
        for j in 0..n_width {
            if index < num_items {
                grid[i as usize][j as usize] = index as i16;
                index += 1;
            } else {
                grid[i as usize][j as usize] = -1; // Initialize with -1 to indicate empty
            }
        }
    }

    grid
}

mod file_tile;