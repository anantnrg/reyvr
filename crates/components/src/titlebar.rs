use super::icon::*;
use gpui::*;

pub struct Titlebar {
    title: Model<SharedString>,
}

impl Render for Titlebar {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_9()
            .bg(rgb(0x1e1d2d))
            .flex()
            .items_center()
            .justify_between()
            .child("close")
            .child(format!("Reyvr - Playing {}", self.title.read(cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .w_auto()
                    .h_full()
                    .child(
                        div()
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|this| this.bg(rgb(0x45475a)))
                            .child(Icon::new(Icons::Minimize).size(20.0).color(0xffffff)),
                    )
                    .child(
                        div()
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|this| this.bg(rgb(0x45475a)))
                            .child({
                                if cx.is_maximized() {
                                    Icon::new(Icons::Restore).size(20.0).color(0xffffff)
                                } else {
                                    Icon::new(Icons::Maximize).size(20.0).color(0xffffff)
                                }
                            }),
                    )
                    .child(
                        div()
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|this| this.bg(rgb(0xe81123)))
                            .child(Icon::new(Icons::Close).size(20.0).color(0xffffff)),
                    ),
            )
    }
}

impl Titlebar {
    pub fn new(title: Model<SharedString>) -> Titlebar {
        Titlebar { title }
    }
}
