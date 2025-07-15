use iced::{alignment::Horizontal, widget::{column, container, image::Handle, text, Container, Image}, Element, Length};

use crate::gui::{FileWithPreview, LandingMessage, SINGLE_PAD};

const PLACEHOLDER_IMAGE: &[u8] = include_bytes!("../../artifacts/placeholder.png");

pub fn file_tile(file: &FileWithPreview) -> Element<'_, LandingMessage> {
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
    println!("Creating file tile for: {}", file_name);

    container(column![preview_image, text(file_name.to_string())]
        .width(200)
        .height(150)
        .align_x(Horizontal::Center)
        .padding(SINGLE_PAD)).style(container::bordered_box).into()
}

pub fn results_area(files: &Option<Vec<FileWithPreview>>) -> Element<'_, LandingMessage> {
    let cont: Container<_>;
    if let Some(files) = files {
        if files.is_empty() {
            return iced::widget::text("No results found").into();
        }
        let file_elements: Vec<Element<LandingMessage>> = files.iter()
            .map(|file| file_tile(file).into())
            .collect();
        cont = container(column(file_elements).spacing(SINGLE_PAD)).clip(true)
    } else {
        cont = container("No results to display").center(Length::Fill)
    }

    cont.width(Length::Fill)
        .height(Length::Fill)
        .padding(SINGLE_PAD)
        .style(container::bordered_box)
        .into()
}