use camino::Utf8PathBuf;
use futures::{stream::FuturesUnordered, FutureExt};
use iced::{task::Handle, widget::{button, column, horizontal_rule, row, text_input}, Element, Length, Pixels, Size, Task, Theme};

use crate::gui::{tasks::{generate_or_retrieve_preview, run_index_query}, widgets::results_area};

const SINGLE_PAD : Pixels = Pixels(5.0);

pub fn run_fetch_application() -> iced::Result {
    iced::application(Landing::default, Landing::update, Landing::view)
        .title("Fetch")
        .window_size(Size::new(1075.0, 700.0))
        .theme(|_state| theme())
        .executor::<tokio::runtime::Runtime>()
        .run()
}

fn theme() -> Theme {
    Theme::Dark
}

#[derive(Clone, Default)]
pub struct Landing {
    query: Option<String>,
    page: u32,
    files: Option<Vec<FileWithPreview>>,
    loading_task_handle: Option<Handle>,
}

#[derive(Clone)]
pub struct FileWithPreview {
    path: Utf8PathBuf,
    preview: Option<Utf8PathBuf>,
}
impl FileWithPreview {
    pub fn new(path: Utf8PathBuf) -> Self {
        Self {
            path,
            preview: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LandingMessage {
    QueryChanged(String),
    QuerySet,
    QueryFinished(Result<Vec<Utf8PathBuf>, String>),
    PreviewCompleted(usize, Result<Option<Utf8PathBuf>, String>),
    FileClicked(Utf8PathBuf),
}

impl Landing {
    pub fn update(&mut self, message: LandingMessage) -> Task<LandingMessage> {
        match message {
            LandingMessage::QueryChanged(query) => {
                self.query = Some(query);
                Task::none()
            },
            LandingMessage::QuerySet => {
                let ref_query = self.query.as_ref();
                if ref_query == None || ref_query.unwrap().is_empty() {
                    self.files = None;
                    Task::none()
                } else {
                    Task::perform(run_index_query(ref_query.unwrap().to_string()), LandingMessage::QueryFinished)
                }
            },
            LandingMessage::QueryFinished(files) => {
                println!("Query finished with results: {:?}", files);

                // TODO: Disable spinner or loading indicator
        
                if files.is_err() {
                    eprintln!("Error querying files: {}", files.as_ref().err().unwrap());
                    self.files = None;
                    return Task::none();
                }

                self.files = Some(files.unwrap().into_iter().map(FileWithPreview::new).collect());

                // Start loading previews for each file
                let futures: FuturesUnordered<_> = FuturesUnordered::new();
                for (i, file) in self.files.as_ref().unwrap().iter().enumerate() {
                    futures.push(generate_or_retrieve_preview(file.path.clone())
                        .map(move |r| LandingMessage::PreviewCompleted(i, r)));
                }
                let (task, handle) = Task::run(futures, std::convert::identity).abortable();
                self.loading_task_handle = Some(handle);

                task
            },
            LandingMessage::PreviewCompleted(i, preview_result) => {
                if self.files.is_none() {
                    println!("Finished loading preview for file but no search results are stored? ignoring");
                    return Task::none();
                }
                if preview_result.is_err() {
                    eprintln!("Error generating preview for file at index {}: {}", i, preview_result.as_ref().err().unwrap());
                    return Task::none();
                }
                let preview_path = preview_result.unwrap();
                self.files.as_mut().unwrap()[i].preview = preview_path;
                Task::none()
            },
            LandingMessage::FileClicked(path) => {
                // Handle file double click, e.g., open the file or show details
                println!("Opening file location: {}", path);

                utility::show_file_location(&path)
                    .unwrap_or_else(|e| eprintln!("Error showing file location: {}", e));

                Task::none()
            },
        }
    }

    pub fn view(&self) -> Element<'_, LandingMessage> {
        let query_input = text_input("Enter query here...", 
                &self.query.as_ref().unwrap_or(&"".to_string()))
            .on_input(LandingMessage::QueryChanged)
            .on_submit(LandingMessage::QuerySet)
            .padding(SINGLE_PAD)
            .width(Length::Fill);
        let search_button = button("Search")
            .on_press(LandingMessage::QuerySet)
            .padding(SINGLE_PAD);

        let search_row = row![
            query_input,
            search_button,
        ].spacing(SINGLE_PAD);

        let results_area = results_area(&self.files);

        column![search_row, horizontal_rule(1), results_area]
            .width(Length::Fill)
            .height(900)
            .padding(SINGLE_PAD)
            .spacing(SINGLE_PAD)
            .into()
    }
}

mod tasks;
mod widgets;
mod utility;