use backend::player::Controller;
use components::theme::Theme;
use gpui::*;

use crate::{layout::Layout, now_playing::NowPlaying};

pub struct QueueList {
    pub now_playing: Entity<NowPlaying>,
    pub layout: Entity<Layout>,
}

impl Render for QueueList {
    fn render(&mut self, win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let window_width = win.window_bounds().get_bounds().size.width.0;
        let tracks = self.now_playing.read(cx).tracks.clone();

        if window_width < 600.0 {
            div().id("")
        } else {
            div()
                .bg(theme.background)
                .h_full()
                .w_1_3()
                .min_w(px(320.0))
                .border_l_1()
                .border_color(theme.secondary)
                .id("scrollview")
                .overflow_y_scroll()
                .children(tracks.into_iter().enumerate().map(|(id, track)| {
                    div()
                        .w_full()
                        .h_16()
                        .flex()
                        .px_3()
                        .gap_2()
                        .text_color(theme.text)
                        .items_center()
                        .justify_between()
                        .px_2()
                        .border_b_1()
                        .border_color(theme.secondary)
                        .rounded_md()
                        .hover(|this| this.bg(theme.secondary))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            let controller = cx.global::<Controller>().clone();
                            controller.play_id(id);
                        })
                        .child({
                            if let Some(thumbnail) = track.thumbnail.clone() {
                                img(ImageSource::Render(
                                    RenderImage::new(thumbnail.img.clone()).into(),
                                ))
                                .h(px(56.0))
                                .rounded_md()
                            } else {
                                img("")
                            }
                        })
                        .child(
                            div()
                                .w_full()
                                .h(px(56.0))
                                .flex()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .child(track.title.clone())
                                        .truncate()
                                        .text_ellipsis()
                                        .text_base()
                                        .font_weight(FontWeight::MEDIUM),
                                )
                                .child(
                                    div()
                                        .child(track.artists.join(", "))
                                        .truncate()
                                        .text_ellipsis()
                                        .text_sm()
                                        .font_weight(FontWeight::NORMAL),
                                ),
                        )
                }))
        }
    }
}

impl QueueList {
    pub fn new(now_playing: Entity<NowPlaying>, layout: Entity<Layout>) -> Self {
        QueueList {
            now_playing,
            layout,
        }
    }
}
