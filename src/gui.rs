use camino::Utf8PathBuf;
use iced::{task::Handle, widget::{button, column, horizontal_rule, row, text_input}, Element, Length, Pixels, Size, Task, Theme};

use crate::gui::{tasks::{generate_or_retrieve_preview, run_index_query}, widgets::results_area::{self, ResultsArea}};

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
    files: Option<Vec<Utf8PathBuf>>,
    results_area: ResultsArea,
    loading_task_handle: Option<Handle>,
}

#[derive(Clone, Debug)]
pub enum LandingMessage {
    QueryChanged(String),
    QuerySet,
    QueryFinished(Result<Vec<Utf8PathBuf>, String>),
    ResultsAreaMessage(results_area::Message),
    FileClicked(Utf8PathBuf),
    None,
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
                            self.update_results(None)
                        } else {
                            Task::perform(run_index_query(ref_query.unwrap().to_string()), LandingMessage::QueryFinished)
                        }
                    },
            LandingMessage::QueryFinished(result) => {
                        println!("Query finished with results: {:?}", result);

                        // TODO: Disable spinner or loading indicator
        
                        if result.is_err() {
                            eprintln!("Error querying files: {}", result.as_ref().err().unwrap());
                            return self.update_results(None);
                        }

                        self.update_results(Some(result.unwrap()))
                    },
            LandingMessage::FileClicked(path) => {
                        // Handle file double click, e.g., open the file or show details
                        println!("Opening file location: {}", path);

                        utility::show_file_location(&path)
                            .unwrap_or_else(|e| eprintln!("Error showing file location: {}", e));

                        Task::none()
                    },
            LandingMessage::ResultsAreaMessage(message) => {
                        self.results_area.update(message).into()
                    },
            LandingMessage::None => {
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

        let results_area = self.results_area.view();

        column![search_row, horizontal_rule(1), results_area.map(LandingMessage::ResultsAreaMessage)]
            .width(Length::Fill)
            .height(900)
            .padding(SINGLE_PAD)
            .spacing(SINGLE_PAD)
            .into()
    }

    fn update_results(&mut self, oresults: Option<Vec<Utf8PathBuf>>) -> Task<LandingMessage> {
        self.files = oresults;

        if let Some(results) = self.files.clone() {
            self.results_area.update(results_area::Message::UpdateResults(results)).into()
        } else {
            self.results_area.update(results_area::Message::UpdateResults(vec![])).into()
        }
    }
}

impl From<results_area::Action> for Task<LandingMessage> {
    fn from(value: results_area::Action) -> Self {
        match value {
            results_area::Action::LoadPreviews(requests) => {
                // map each request to a task
                let tasks: Vec<Task<LandingMessage>> = requests.into_iter()
                    .map(|lpr| Task::future((async move || {
                        // this closure calls the async fn we want and then maps the result to a message.
                        let ro = generate_or_retrieve_preview(&lpr.path).await;
                        match ro.transpose() {
                            Some(r) => LandingMessage::ResultsAreaMessage(results_area::Message::UpdatePreview { 
                                index: lpr.index, 
                                path: lpr.path, 
                                handle_result: r 
                            }),
                            None => LandingMessage::None,
                        }
                    })())) // extra () are to actually call the async closure for the future
                    .collect();

                Task::batch(tasks)
            },
            results_area::Action::None => Task::none(),
        }
    }
}

mod tasks;
mod widgets;
mod utility;