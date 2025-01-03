use gpui::{MouseButton, MouseDownEvent, SharedString, WindowContext, div, prelude::*, px, rgb};

#[derive(IntoElement)]
pub struct Button {
    text: SharedString,
    width: f32,
    height: f32,
    bg_color: u32,
    text_color: u32,
    border_color: u32,
    rounded: f32,
    on_click: Box<dyn Fn(MouseDownEvent, &mut WindowContext) + 'static>,
}

#[allow(dead_code)]
impl Button {
    pub fn new() -> Self {
        Self {
            text: SharedString::from("Button"),
            width: 230.0,
            height: 40.0,
            bg_color: 0xcba6f7,
            text_color: 0x1e1e2d,
            border_color: 0x45475a,
            rounded: 8.0,
            on_click: Box::new(|_, _| println!("Clicked!")),
        }
    }

    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = text.into();
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn bg(mut self, color: u32) -> Self {
        self.bg_color = color;
        self
    }

    pub fn text_color(mut self, color: u32) -> Self {
        self.text_color = color;
        self
    }

    pub fn border_color(mut self, color: u32) -> Self {
        self.border_color = color;
        self
    }

    pub fn rounded(mut self, rounded: f32) -> Self {
        self.rounded = rounded;
        self
    }

    pub fn on_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(MouseDownEvent, &mut WindowContext) + 'static,
    {
        self.on_click = Box::new(callback);
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        let on_click = self.on_click;
        div()
            .flex()
            .w(px(self.width))
            .h(px(self.height))
            .bg(rgb(self.bg_color))
            .text_color(rgb(self.text_color))
            .border_2()
            .rounded(px(self.rounded))
            .border_color(rgb(self.border_color))
            .justify_center()
            .content_center()
            .items_center()
            .child(self.text)
            .on_mouse_down(MouseButton::Left, move |event, context| {
                (on_click)(event.clone(), context);
            })
            .into_element()
    }
}
