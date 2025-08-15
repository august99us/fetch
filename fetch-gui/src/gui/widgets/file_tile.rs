// TODO: organize iced imports
use std::{slice, time::{Duration, Instant}};
use iced::{advanced::{self, graphics::core::Rectangle as CoreRectangle, image::{self, Handle}, layout, mouse, renderer::{self, Quad}, widget::{tree, Widget}, Clipboard, Layout, Shell}, alignment::Horizontal, widget::{column, text}, Element, Event, Length, Rectangle, Size};

use crate::gui::SINGLE_PAD;
use state::{State, Status};
use styling::{Catalog};

pub struct FileTile<'a, Message, Theme, Renderer> {
    width: Length,
    height: Length,
    content: Element<'a, Message, Theme, Renderer>,
    selected: bool,
    on_click: Option<Box<dyn Fn() -> Message + 'a>>,
    on_double_click: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> FileTile<'a, Message, Theme, Renderer> 
where 
    Message: 'a,
    Theme: text::Catalog + Catalog + 'a,
    Renderer: image::Renderer<Handle = Handle> + advanced::text::Renderer + 'a,
{
    pub fn new(file_name: String, preview: &Handle, selected: bool) -> Self {
        let image = iced::widget::image(preview).height(Length::Fill);

        let content = column![image, text(file_name.clone())]
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(SINGLE_PAD).into();

        Self {
            width: Length::Fill,
            height: Length::Fill,
            content,
            selected,
            on_click: None,
            on_double_click: None,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }
    
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'a,
    {
        self.on_click = Some(Box::new(f));
        self
    }

    pub fn on_double_click<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Message + 'a,
    {
        self.on_double_click = Some(Box::new(f));
        self
    }
}

impl<'a, Message, Theme, Renderer> 
    Widget<Message, Theme, Renderer> 
for FileTile<'a, Message, Theme, Renderer> 
where 
    Theme: text::Catalog + Catalog,
    Renderer: image::Renderer<Handle = Handle> + advanced::text::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            last_click: None,
            click_count: 0,
            status: if self.selected { Status::Selected } else { Status::Default },
        })
    }

    fn diff(&self, tree: &mut tree::Tree) {
        let state = tree.state.downcast_mut::<State>();
        if self.selected != (state.status == Status::Selected) {
            state.status = if self.selected { Status::Selected } else { Status::Default };
        }

        tree.diff_children(slice::from_ref(&self.content));
    }

    fn layout(
        &self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget().layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );

        if shell.is_event_captured() {
            // If the event is captured, we don't need to handle it further
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        
        // Update hover state based on cursor position
        let is_over = cursor.is_over(layout.bounds());
        if self.selected {
            state.status = Status::Selected;
        } else {
            // If the tile is not selected, check that hover state is accurate and update if necessary
            // If it is selected, then mousing over the tile does not change how it looks
            if is_over != (state.status == Status::Hovered) {
                state.status = if is_over {
                    Status::Hovered
                } else {
                    Status::Default
                };

                shell.request_redraw();
            }
        }

        // Handle mouse events, clicks, etc.
        if let Event::Mouse(mouse_event) = event {
            if is_over {
                shell.capture_event();
                if let mouse::Event::ButtonPressed(mouse::Button::Left) = mouse_event {
                    let now = Instant::now();
                    let threshold = get_double_click_threshold();

                    let is_double_click = if let Some(last_click) = state.last_click {
                        now.duration_since(last_click) <= threshold && state.click_count == 1
                    } else {
                        false
                    };

                    if is_double_click {
                        // Double click detected
                        if let Some(on_double_click) = &self.on_double_click {
                            shell.publish(on_double_click());
                        }
                        state.click_count = 0;
                        state.last_click = None;
                    } else {
                        // Single click or first click of potential double click
                        state.click_count = 1;
                        state.last_click = Some(now);
                        
                        // Single click action for selection (handled by parent)
                        if let Some(on_click) = &self.on_click {
                            shell.publish(on_click());
                        }
                    }
                }
            }
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport
        );
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        let tile_style = <Theme as Catalog>::style(theme, &<Theme as Catalog>::default(), &state.status);
        
        // Draw background highlight based on state
        let background = Quad {
            bounds: CoreRectangle::new(bounds.position(), bounds.size()),
            border: tile_style.border,
            shadow: Default::default(),
            snap: Default::default(),
        };
            
        renderer.fill_quad(background, tile_style.background);
        
        // Draw the content on top
        self.content.as_widget().draw(
            &tree.children[0], 
            renderer, 
            theme, 
            style, 
            layout, 
            cursor, 
            viewport
        );
    }
}

impl<'a, Message, Theme, Renderer> 
    From<FileTile<'a, Message, Theme, Renderer>> 
for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: text::Catalog + Catalog + 'a,
    Renderer: image::Renderer<Handle = Handle> + advanced::text::Renderer + 'a,
{
    fn from(tile: FileTile<'a, Message, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(tile)
    }
}

// Private methods and state
const DEFAULT_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(300);

// todo: move platform specific stuff elsewhere
fn get_double_click_threshold() -> Duration {
    #[cfg(target_os = "windows")]
    {
        // On Windows, we could use GetDoubleClickTime() from user32
        // For now, fall back to default until we add the windows crate dependency
        DEFAULT_DOUBLE_CLICK_THRESHOLD
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, we could use [NSEvent doubleClickInterval]
        // For now, fall back to default until we add cocoa bindings
        DEFAULT_DOUBLE_CLICK_THRESHOLD
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, we could read from GTK settings or gsettings
        // For now, fall back to default until we add gtk-rs dependency
        DEFAULT_DOUBLE_CLICK_THRESHOLD
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        DEFAULT_DOUBLE_CLICK_THRESHOLD
    }
}

mod state;
mod styling;