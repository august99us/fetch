use std::{borrow::Cow, ops::{AddAssign, SubAssign}};

use camino::Utf8PathBuf;
use iced::{task::Handle, widget::{button, column, container, horizontal_rule, horizontal_space, row, stack, text, text_input, vertical_space}, Alignment, Element, Font, Length, Pixels, Size, Task, Theme};

use crate::gui::{tasks::{generate_or_retrieve_preview, run_index_query}, widgets::results_area::{self, ResultsArea}};

const SINGLE_PAD : Pixels = Pixels(5.0);

pub fn run_fetch_application() -> iced::Result {
    let mut settings = iced::Settings::default();
    let fonts = vec![Cow::from(include_bytes!("../artifacts/fonts/Inter/Inter_18pt-Regular.ttf"))];
    settings.fonts = fonts;
    settings.default_font = Font::with_name("Inter");

    iced::application(SearchPage::default, SearchPage::update, SearchPage::view)
        .title("Fetch")
        .window_size(Size::new(1075.0, 700.0))
        .settings(settings)
        .theme(|_state| theme())
        .executor::<tokio::runtime::Runtime>()
        .run()
}

fn theme() -> Theme {
    Theme::Dark
}

#[derive(Clone, Debug, Default)]
pub struct SearchPage {
    query: Option<String>,
    page: Page,
    files: Option<Vec<Utf8PathBuf>>,
    results_area: ResultsArea,
    loading_task_handle: Option<Handle>,
    querying_index: bool,
}

#[derive(Clone, Debug)]
pub enum SearchPageMessage {
    QueryChanged(String),
    QuerySet,
    NextPage,
    PreviousPage,
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
                println!("Query submitted: {:?}", self.query);
                if self.query_string_is_valid() {
                    self.run_query()
                } else {
                    self.update_results(None)
                }
            },
            SearchPageMessage::NextPage => {
                println!("Next page requested, current page: {}", self.page.0);
                if self.query_string_is_valid() {
                    self.page += 1;
                    self.run_query()
                } else {
                    Task::none()
                }
            },
            SearchPageMessage::PreviousPage => {
                println!("Previous page requested, current page: {}", self.page.0);
                if self.query_string_is_valid() && self.page > 1 {
                    self.page -= 1;
                    self.run_query()
                } else {
                    Task::none()
                }
            },
            SearchPageMessage::QueryFinished(result) => {
                self.querying_index = false;

                if result.is_err() {
                    eprintln!("Error querying files: {}", result.as_ref().err().unwrap());
                    return self.update_results(None);
                }

                let unwrapped = result.unwrap();
                println!("Query finished with {} results.", unwrapped.len());

                self.update_results(Some(unwrapped))
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
        let global_disable = self.querying_index;

        // Search row /////////////////////
        let mut query_input = text_input("Enter query here...", 
            self.query.as_ref().unwrap_or(&"".to_string()))
            .padding(SINGLE_PAD)
            .width(Length::Fill);
        if !global_disable {
            query_input = query_input
                .on_input(SearchPageMessage::QueryChanged)
                .on_submit(SearchPageMessage::QuerySet);
        }

        let search_button = if global_disable {
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

        // Results row /////////////////////
        let results_area = self.results_area.view().map(SearchPageMessage::ResultsAreaMessage);
        
        let results_content = if global_disable {
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

        // Pagination row /////////////////////
        let prev_text = text("◀").size(14).center();
        let mut prev_button = button(prev_text)
            .padding(SINGLE_PAD)
            .width(Length::Fixed(27.0))
            .height(Length::Shrink)
            .style(button::secondary);
        
        if !global_disable && self.page > 1 {
            prev_button = prev_button.on_press(SearchPageMessage::PreviousPage);
        }

        let page_info = column![vertical_space().height(Length::Fixed(2.0)),
            text(format!("Page {}", self.page.0)).size(18)];

        let next_text = text("▶").size(14).center();
        let mut next_button = button(next_text)
            .padding(SINGLE_PAD)
            .width(Length::Fixed(27.0))
            .height(Length::Shrink)
            .style(button::secondary);
        
        if !global_disable {
            next_button = next_button.on_press(SearchPageMessage::NextPage);
        }

        let pagination_row = row![
            horizontal_space().width(Length::Fill),
            prev_button,
            horizontal_space().width(Length::Fixed(8.0)),
            page_info,
            horizontal_space().width(Length::Fixed(8.0)),
            next_button,
        ].spacing(SINGLE_PAD)
         .width(Length::Fill)
         .height(Length::Shrink)
         .align_y(Alignment::Center);

        // Composition of elements /////////////////////
        column![search_row, horizontal_rule(1), results_content, pagination_row]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(SINGLE_PAD)
            .spacing(SINGLE_PAD)
            .into()
    }

    fn run_query(&mut self) -> Task<SearchPageMessage> {
        if !self.verify_query_allowed() {
            return Task::none();
        }

        let ref_query = self.query.as_ref();
        self.querying_index = true;
        Task::perform(run_index_query(ref_query.unwrap().clone(), self.page.into()), 
            SearchPageMessage::QueryFinished)
    }

    fn query_string_is_valid(&self) -> bool {
        self.query.as_ref().is_some_and(|q| !q.is_empty())
    }

    fn update_results(&mut self, oresults: Option<Vec<Utf8PathBuf>>) -> Task<SearchPageMessage> {
        // Ensure querying state is always cleared when updating results
        self.querying_index = false;
        self.files = oresults;

        if let Some(results) = self.files.clone() {
            self.results_area.update(results_area::Message::UpdateResults(results)).into()
        } else {
            self.results_area.update(results_area::Message::UpdateResults(vec![])).into()
        }
    }

    fn verify_query_allowed(&self) -> bool {
        // Don't start a new query if already querying
        !self.querying_index
    }
}

impl From<results_area::Action> for Task<SearchPageMessage> {
    fn from(value: results_area::Action) -> Self {
        match value {
            results_area::Action::LoadPreviews(requests) => {
                // map each request to a task
                let tasks: Vec<Task<SearchPageMessage>> = requests.into_iter()
                    .map(|lpr| Task::future(async move {
                        // this block calls the async fn we want and then maps the result to a message.
                        // LPRs are consumed anyway so moving into the async closure is fine
                        let ro = generate_or_retrieve_preview(&lpr.path).await;
                        match ro.transpose() {
                            Some(r) => SearchPageMessage::ResultsAreaMessage(
                                results_area::Message::UpdatePreview { 
                                    index: lpr.index, 
                                    path: lpr.path, 
                                    handle_result: r 
                                }),
                            None => SearchPageMessage::None,
                        }
                    }))
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

// private functions/structs/modules
#[derive(Copy, Clone, Debug)]
struct Page(u32);
impl Default for Page {
    fn default() -> Self {
        Page(1)
    }
}
impl AddAssign<u32> for Page {
    fn add_assign(&mut self, other: u32) {
        self.0 += other;
    }
}
impl SubAssign<u32> for Page {
    fn sub_assign(&mut self, other: u32) {
        self.0 -= other;
    }
}
impl PartialEq<u32> for Page {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}
impl PartialOrd<u32> for Page {
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(other))
    }
}
impl From<Page> for u32 {
    fn from(value: Page) -> Self {
        value.0
    }
}
impl From<u32> for Page {
    fn from(value: u32) -> Self {
        Page(value)
    }
}

mod tasks;
mod widgets;
mod utility;