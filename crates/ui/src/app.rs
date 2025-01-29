use super::{now_playing::*, res_handler::*, titlebar::Titlebar};
use crate::{control_bar::ControlBar, layout::Layout, main_view::MainView, sidebar::*};
use components::theme::Theme;
use gpui::*;

#[derive(Clone)]
pub struct Reyvr {
    pub titlebar: Entity<Titlebar>,
    pub left_sidebar: Entity<LeftSidebar>,
    pub control_bar: Entity<ControlBar>,
    pub main_view: Entity<MainView>,
    pub layout: Layout,
    pub now_playing: Entity<NowPlaying>,
    pub res_handler: Entity<ResHandler>,
}

impl Render for Reyvr {
    fn render(&mut self, win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let titlebar = self.clone().titlebar;
        let sidebar = self.clone().left_sidebar;
        let control_bar = self.clone().control_bar;
        let main_view = self.clone().main_view;
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .h_full()
            .flex_col()
            .child(titlebar.clone())
            .child(
                div()
                    .w_full()
                    .h(px(
                        win.window_bounds().get_bounds().size.height.0 - (32.0 + 72.0)
                    ))
                    .bg(theme.background)
                    .flex()
                    .overflow_hidden()
                    .child(sidebar)
                    .child(main_view),
            )
            .child(control_bar)
    }
}
