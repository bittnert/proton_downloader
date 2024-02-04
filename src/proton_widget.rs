use iced::{
    advanced::{
        layout, renderer,
        widget::{self, Widget},
    },
    Color, Element, Length, Size,
};

pub struct ProtonWidget {
    name: String,
    installed: bool,
}

enum states {
    Installed,
    Uninstalled,
    Installing,
}

pub fn proton_widget(name: String, installed: bool) -> ProtonWidget {
    ProtonWidget::new(name, installed)
}

impl<Message, Renderer> Widget<Message, Renderer> for ProtonWidget
where
    Renderer: renderer::Renderer,
{
    fn width(&self) -> iced::Length {
        Length::Shrink
    }

    fn height(&self) -> iced::Length {
        Length::Shrink
    }

    fn layout(
        &self,
        _renderer: &Renderer,
        _limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        layout::Node::new(Size::new(100.0, 100.0))
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &<Renderer as iced::advanced::Renderer>::Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border_radius: 100.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Color::BLACK,
        );
    }
}

impl<'a, Message, Renderer> From<ProtonWidget> for Element<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn from(proton_widget: ProtonWidget) -> Self {
        Self::new(proton_widget)
    }
}

impl ProtonWidget {
    pub fn new(name: String, installed: bool) -> ProtonWidget {
        Self { name, installed }
    }
}
