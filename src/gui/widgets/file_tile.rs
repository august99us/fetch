use camino::Utf8PathBuf;
// TODO: organize iced imports
use iced::{advanced::{image::{self as iced_image, Handle}, layout, mouse, renderer::Style, widget::{tree, Widget}, Clipboard, Layout, Shell}, event, widget::image::FilterMethod, window, ContentFit, Element, Event, Length, Point, Rectangle, Rotation, Size, Vector};

use crate::gui::FileWithPreview;

use state::{State, ThumbnailImage};

pub struct FileTile {
    width: Length,
    height: Length,
    content_fit: ContentFit,
    path: Utf8PathBuf,
    preview_path: Option<Utf8PathBuf>,
}

impl FileTile {
    pub fn new(file_with_preview: &FileWithPreview) -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fill,
            content_fit: ContentFit::Contain,
            // TODO: figure out if these clones can be replaced with a lifetime
            path: file_with_preview.path.clone(),
            preview_path: file_with_preview.preview.clone(),
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

// comments are associated with iced 0.13.1. who knows if they will still be relevant in later versions.
impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for FileTile
where
    Renderer: iced_image::Renderer<Handle = Handle>,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    // provides identification for which tree node this widget should be assigned???
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    // provides fresh new state object?
    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.preview_path.clone()))
    }

    // I am assuming before calling this function, the engine has already figured out which tree node
    // this widget is assigned to and is diffing against the state in that tree node.
    fn diff(&self, tree: &mut tree::Tree) {
        let state = tree.state.downcast_mut::<State>();

        // update state if new preview_path has been provided
        if self.preview_path.as_ref().map(Utf8PathBuf::as_path) != state.thumbnail.get_preview_path() {
            let pp = self.preview_path.clone();
            state.thumbnail = ThumbnailImage::new(pp);
        }
        // how does it know if it needs redraw?
    }

    fn layout(
        &self,
        tree: &mut tree::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_ref::<State>();

        let image = state.thumbnail.get_image();
        let image_width = image.width() as f32;
        let image_height = image.height() as f32;
        let raw_size = limits.resolve(
            self.width,
            self.height,
            Size { width: image_width, height: image_height }
        );

        layout::Node::new(raw_size)
    }

    fn update(
            &mut self,
            tree: &mut tree::Tree,
            event: &Event,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _renderer: &Renderer,
            _clipboard: &mut dyn Clipboard,
            _shell: &mut Shell<'_, Message>,
            _viewport: &Rectangle,
        ) {
        // Handle mouse events, clicks, etc.
        let state = tree.state.downcast_mut::<State>();

        if let Event::Mouse(mouse_event) = event {
            match mouse_event {
                mouse::Event::CursorMoved { position } => {
                    println!("Mouse moved to: {:?}", position);
                    // todo
                }
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if state.mouseover {
                        state.mouseclick = true;

                        println!("File clicked: {:?}", self.path);

                        // shell.publish(LandingMessage::FileClicked(self.path.clone()));
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    println!("Button released: {:?}", self.path);
                    state.mouseclick = false;
                }
                _ => {}
            }
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // print the current time, current state in the tree, to figure out how the engine is calling this.
        let state = tree.state.downcast_ref::<State>();

        let image = state.thumbnail.get_image();
        let rgba = image.to_rgba8();
        let handle = Handle::from_rgba(image.width(), image.height(), rgba.into_vec());

        let Size { width, height } = renderer.measure_image(&handle);
        let image_size = Size::new(width as f32, height as f32);

        let bounds = layout.bounds();

        let adjusted_fit = self.content_fit.fit(image_size, bounds.size());

        let scale = Vector::new(
            adjusted_fit.width / image_size.width,
            adjusted_fit.height / image_size.height,
        );

        let final_size = image_size * scale;

        let position = match self.content_fit {
            ContentFit::None => Point::new(
                bounds.x + (image_size.width - adjusted_fit.width) / 2.0,
                bounds.y + (image_size.height - adjusted_fit.height) / 2.0,
            ),
            _ => Point::new(
                bounds.center_x() - final_size.width / 2.0,
                bounds.center_y() - final_size.height / 2.0,
            ),
        };

        let drawing_bounds = Rectangle::new(position, final_size);

        let render = |renderer: &mut Renderer| {
            renderer.draw_image(
                iced_image::Image {
                    handle: handle.clone(),
                    filter_method: FilterMethod::default(),
                    rotation: Rotation::default().radians(),
                    opacity: 1.0,
                    snap: true,
                },
                drawing_bounds,
            );
        };

        if adjusted_fit.width > bounds.width || adjusted_fit.height > bounds.height {
            renderer.with_layer(bounds, render);
        } else {
            render(renderer);
        }
    }
}

impl<Message, Theme, Renderer> From<FileTile> for Element<'_, Message, Theme, Renderer>
where
    Renderer: iced_image::Renderer<Handle = Handle>,
{
    fn from(tile: FileTile) -> Element<'static, Message, Theme, Renderer> {
        Element::new(tile)
    }
}

mod state;