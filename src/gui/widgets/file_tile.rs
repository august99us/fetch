// TODO: organize iced imports
use iced::{advanced::{self, image::{self, Handle}, layout, mouse, renderer::Style, widget::{tree, Widget}, Clipboard, Layout, Shell}, alignment::Horizontal, widget::{column, text}, Element, Event, Length, Rectangle, Size};

use crate::gui::SINGLE_PAD;

pub struct FileTile<'a, Message, Theme, Renderer> {
    width: Length,
    height: Length,
    file_name: &'a str,
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> FileTile<'a, Message, Theme, Renderer> 
where 
    Message: 'a,
    Theme: text::Catalog + 'a,
    Renderer: image::Renderer<Handle = Handle> + advanced::text::Renderer + 'a,
{
    pub fn new(file_name: &'a str, preview: &Handle) -> Self {
        let image = iced::widget::image(preview).height(Length::Fill);

        let content = column![image, text(file_name.clone())]
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(SINGLE_PAD).into();

        Self {
            width: Length::Fill,
            height: Length::Fill,
            file_name,
            content,
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
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for FileTile<'a, Message, Theme, Renderer> 
where 
    Theme: text::Catalog,
    Renderer: image::Renderer<Handle = Handle> + advanced::text::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    // provides fresh new state object?
    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    // I am assuming before calling this function, the engine has already figured out which tree node
    // this widget is assigned to and is diffing against the state in that tree node.
    fn diff(&self, tree: &mut tree::Tree) {
        self.content.as_widget().diff(tree)
    }

    fn layout(
        &self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget().layout(tree, renderer, limits)
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
        // Handle mouse events, clicks, etc.
        if let Event::Mouse(mouse_event) = event {
            match mouse_event {
                mouse::Event::CursorMoved { position } => {
                    println!("Mouse moved to: {:?}", position);
                    // todo
                }
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    println!("File clicked: {:?}", self.file_name);
                    // todo
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    println!("Button released: {:?}", self.file_name);
                    // todo
                }
                _ => {}
            }
        }

        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // print the current time, current state in the tree, to figure out how the engine is calling this.

        self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
}

impl<'a, Message, Theme, Renderer> 
    From<FileTile<'a, Message, Theme, Renderer>> 
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: text::Catalog + 'a,
    Renderer: image::Renderer<Handle = Handle> + advanced::text::Renderer + 'a,
{
    fn from(tile: FileTile<'a, Message, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(tile)
    }
}