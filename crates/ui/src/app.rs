use super::{now_playing::*, res_handler::*, titlebar::Titlebar};
use crate::layout::Layout;
use backend::player::Controller;
use components::{button::Button, slider::Slider, theme::Theme};
use gpui::*;

#[derive(Clone)]
pub struct Reyvr {
    pub vol_slider: Entity<Slider>,
    pub layout: Layout,
    pub now_playing: Entity<NowPlaying>,
    pub theme: Theme,
    pub res_handler: Entity<ResHandler>,
}

impl Render for Reyvr {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.clone();
        let titlebar = cx.new(|_| Titlebar::new(self.now_playing.clone(), theme.clone()));

        div()
            .w_full()
            .h_full()
            .flex_col()
            .child(titlebar.clone())
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_8()
                    .bg(theme.background)
                    .size_full()
                    .justify_center()
                    .items_center()
                    .child(Button::new().text("Play").on_click({
                        // let now_playing = self.now_playing.clone();
                        move |_, _, cx| {
                            cx.global::<Controller>().play();
                        }
                    }))
                    .child(Button::new().text("Pause").on_click({
                        move |_, _, cx| {
                            cx.global::<Controller>().pause();
                        }
                    }))
                    .child(Button::new().text("Previous").on_click({
                        move |_, _, cx| {
                            cx.global::<Controller>().prev();
                        }
                    }))
                    .child(Button::new().text("Next").on_click({
                        move |_, _, cx| {
                            cx.global::<Controller>().next();
                        }
                    }))
                    .child(Button::new().text("Load Playlist").on_click({
                        move |_, _, cx| {
                            cx.global::<Controller>()
                                .load("E:\\music\\straight up liquid fire");
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h_auto()
                            .w_20()
                            .child(self.vol_slider.clone()),
                    )
                    .child({
                        if let Some(thumbnail) = self.now_playing.read(cx).thumbnail.clone() {
                            div()
                                .w(px(1280.))
                                .h(px(768.0))
                                .child(img(thumbnail).size_full())
                        } else {
                            div().size_128()
                        }
                    }),
            )
    }
}
