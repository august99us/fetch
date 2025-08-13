use camino::Utf8PathBuf;
use iced::{task::Handle, widget::{button, column, container, horizontal_rule, row, stack, text, text_input, TextInput}, Element, Length, Pixels, Size, Task, Theme};

use crate::gui::{tasks::{generate_or_retrieve_preview, run_index_query}, widgets::results_area::{self, ResultsArea}};

const SINGLE_PAD : Pixels = Pixels(5.0);

pub fn run_fetch_application() -> iced::Result {
    iced::application(SearchPage::default, SearchPage::update, SearchPage::view)
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
pub struct SearchPage {
    query: Option<String>,
    page: u32,
    files: Option<Vec<Utf8PathBuf>>,
    results_area: ResultsArea,
    loading_task_handle: Option<Handle>,
    querying_index: bool,
}

#[derive(Clone, Debug)]
pub enum SearchPageMessage {
    QueryChanged(String),
    QuerySet,
    QueryFinished(Result<Vec<Utf8PathBuf>, String>),
    ResultsAreaMessage(results_area::Message),
    None,
}

impl SearchPage {
    pub fn update(&mut self, message: SearchPageMessage) -> Task<SearchPageMessage> {
        match message {
            SearchPageMessage::QueryChanged(query) => {
                        self.query = Some(query);
                        Task::none()
                    },
            SearchPageMessage::QuerySet => {
                        // Don't start a new query if already querying
                        if self.querying_index {
                            return Task::none();
                        }
                        
                        let ref_query = self.query.as_ref();
                        if ref_query == None || ref_query.unwrap().is_empty() {
                            self.update_results(None)
                        } else {
                            self.querying_index = true;
                            Task::perform(run_index_query(ref_query.unwrap().to_string()), SearchPageMessage::QueryFinished)
                        }
                    },
            SearchPageMessage::QueryFinished(result) => {
                        println!("Query finished with results: {:?}", result);
                        
                        self.querying_index = false;
        
                        if result.is_err() {
                            eprintln!("Error querying files: {}", result.as_ref().err().unwrap());
                            return self.update_results(None);
                        }

                        self.update_results(Some(result.unwrap()))
                    },
            SearchPageMessage::ResultsAreaMessage(message) => {
                        self.results_area.update(message).into()
                    },
            SearchPageMessage::None => {
                        Task::none()
                    },
        }
    }

    pub fn view(&self) -> Element<'_, SearchPageMessage> {
        let mut query_input = text_input("Enter query here...", 
            &self.query.as_ref().unwrap_or(&"".to_string()))
            .padding(SINGLE_PAD)
            .width(Length::Fill);
        if !self.querying_index {
            query_input = query_input
                .on_input(SearchPageMessage::QueryChanged)
                .on_submit(SearchPageMessage::QuerySet);
        }

        let search_button = if self.querying_index {
            button("Searching...")
                .padding(SINGLE_PAD)
        } else {
            button("Search")
                .on_press(SearchPageMessage::QuerySet)
                .padding(SINGLE_PAD)
        };

        let search_row = row![
            query_input,
            search_button,
        ].spacing(SINGLE_PAD);

        let results_area = self.results_area.view().map(SearchPageMessage::ResultsAreaMessage);
        
        let results_content = if self.querying_index {
            let loading_overlay = container(
                text("Searching...")
                    .size(24)
                    .color(iced::Color::WHITE)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(container::transparent)
            .clip(false);

            stack![results_area, loading_overlay].into()
        } else {
            results_area
        };

        column![search_row, horizontal_rule(1), results_content]
            .width(Length::Fill)
            .height(900)
            .padding(SINGLE_PAD)
            .spacing(SINGLE_PAD)
            .into()
    }

    fn update_results(&mut self, oresults: Option<Vec<Utf8PathBuf>>) -> Task<SearchPageMessage> {
        // Ensure loading state is always cleared when updating results
        self.querying_index = false;
        self.files = oresults;

        if let Some(results) = self.files.clone() {
            self.results_area.update(results_area::Message::UpdateResults(results)).into()
        } else {
            self.results_area.update(results_area::Message::UpdateResults(vec![])).into()
        }
    }
}

impl From<results_area::Action> for Task<SearchPageMessage> {
    fn from(value: results_area::Action) -> Self {
        match value {
            results_area::Action::LoadPreviews(requests) => {
                // map each request to a task
                let tasks: Vec<Task<SearchPageMessage>> = requests.into_iter()
                    .map(|lpr| Task::future((async move || {
                        // this closure calls the async fn we want and then maps the result to a message.
                        let ro = generate_or_retrieve_preview(&lpr.path).await;
                        match ro.transpose() {
                            Some(r) => SearchPageMessage::ResultsAreaMessage(results_area::Message::UpdatePreview { 
                                index: lpr.index, 
                                path: lpr.path, 
                                handle_result: r 
                            }),
                            None => SearchPageMessage::None,
                        }
                    })())) // extra () are to actually call the async closure for the future
                    .collect();

                Task::batch(tasks)
            },
            results_area::Action::OpenFile(path) => {
                Task::future(async move {
                    // TODO: Change this back to open_file_with_default_app once testing is complete
                    // Currently opening file location for testing double-click functionality
                    println!("Opening file location (testing double-click): {}", path);
                    utility::show_file_location(&path)
                        .unwrap_or_else(|e| eprintln!("Error opening file location: {}", e));
                    SearchPageMessage::None
                })
            },
            results_area::Action::OpenFileLocation(path) => {
                Task::future(async move {
                    println!("Opening file location: {}", path);
                    utility::show_file_location(&path)
                        .unwrap_or_else(|e| eprintln!("Error showing file location: {}", e));
                    SearchPageMessage::None
                })
            },
            results_area::Action::None => Task::none(),
        }
    }
}

mod tasks;
mod widgets;
mod utility;