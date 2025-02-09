pub mod app;
pub mod assets;
pub mod control_bar;
pub mod layout;
pub mod main_view;
pub mod now_playing;
pub mod queue_list;
pub mod res_handler;
pub mod sidebar;
pub mod titlebar;

use app::Reyvr;
use assets::*;
use backend::{
    Backend,
    playback::{Playlist, SavedPlaylists},
    player::{Controller, Player, Response},
};
use components::{
    slider::{Slider, SliderEvent},
    theme::Theme,
};
use control_bar::ControlBar;
use gpui::*;
use layout::Layout;
use main_view::MainView;
use now_playing::{NowPlaying, NowPlayingEvent, Thumbnail, Track};
use queue_list::QueueList;
use res_handler::ResHandler;
use sidebar::LeftSidebar;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use titlebar::Titlebar;

pub fn run_app(backend: Arc<dyn Backend>) -> anyhow::Result<()> {
    let app = Application::new().with_assets(Assets {
        base: PathBuf::from("assets"),
    });

    app.run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.0), px(500.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                app_id: Some(String::from("reyvr")),
                focus: true,
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    let theme = Theme::default();
                    let now_playing = NowPlaying::new();
                    let np = cx.new(|_| now_playing.clone());
                    let res_handler = cx.new(|_| ResHandler {});
                    let arc_res = Arc::new(res_handler.clone());
                    let (mut player, controller) =
                        Player::new(backend.clone(), Arc::new(Mutex::new(Playlist::default())));
                    let vol_slider = cx.new(|_| {
                        Slider::new(theme)
                            .min(0.0)
                            .max(1.0)
                            .step(0.005)
                            .default(0.2)
                    });
                    let recv_controller = controller.clone();
                    let saved_playlists = cx.new(|_| SavedPlaylists::default());
                    let playlists = saved_playlists.clone();

                    cx.set_global(controller);
                    cx.set_global(theme);
                    cx.background_executor()
                        .spawn(async move {
                            player.run().await;
                        })
                        .detach();
                    cx.spawn(|_, cx: AsyncApp| async move {
                        let res_handler = arc_res.clone();
                        loop {
                            while let Ok(res) = recv_controller.rx.try_recv() {
                                res_handler
                                    .update(&mut cx.clone(), |res_handler, cx| {
                                        res_handler.handle(cx, res);
                                    })
                                    .expect("Could not update");
                            }
                            cx.background_executor()
                                .timer(Duration::from_millis(10))
                                .await;
                        }
                    })
                    .detach();
                    cx.subscribe(
                        &vol_slider,
                        move |this: &mut Reyvr, _, event: &SliderEvent, cx| match event {
                            SliderEvent::Change(vol) => {
                                let volume = (vol * 100.0).round() as f64 / 100.0;
                                cx.global::<Controller>().volume(volume);
                                this.now_playing.update(cx, |this, cx| {
                                    this.update_vol(cx, volume.clone());
                                });
                                cx.notify();
                            }
                        },
                    )
                    .detach();
                    cx.subscribe(
                        &np,
                        |this: &mut Reyvr, _, event: &NowPlayingEvent, cx: &mut Context<Reyvr>| {
                            match event {
                                NowPlayingEvent::Meta(title, album, artists, duration) => {
                                    this.now_playing.update(cx, |this, _| {
                                        this.title = title.clone();
                                        this.album = album.clone();
                                        this.artists = artists.clone();
                                        this.duration = duration.clone();
                                    });
                                    cx.notify();
                                }
                                NowPlayingEvent::Position(pos) => {
                                    this.now_playing.update(cx, |this, _| {
                                        this.position = *pos;
                                    });
                                    cx.notify();
                                }
                                NowPlayingEvent::Thumbnail(img) => {
                                    this.now_playing.update(cx, |this, _| {
                                        this.thumbnail = Some(img.clone());
                                    });
                                    cx.notify();
                                }
                                NowPlayingEvent::State(state) => {
                                    this.now_playing.update(cx, |this, _| {
                                        this.state = state.clone();
                                    });
                                    cx.notify();
                                }
                                NowPlayingEvent::Volume(vol) => {
                                    this.now_playing.update(cx, |this, _| {
                                        this.volume = vol.clone();
                                    });
                                    cx.notify();
                                }
                                NowPlayingEvent::Tracks(tracks) => {
                                    this.now_playing.update(cx, |this, _| {
                                        this.tracks = tracks.clone();
                                    });
                                    cx.notify();
                                }
                            }
                        },
                    )
                    .detach();
                    cx.subscribe(
                        &res_handler,
                        move |this: &mut Reyvr, _, event: &Response, cx| match event {
                            Response::Eos => {
                                println!("End of stream");
                                cx.global::<Controller>().next();
                            }
                            Response::Position(pos) => this.now_playing.update(cx, |np, cx| {
                                np.update_pos(cx, *pos);
                            }),
                            Response::StreamStart => cx.global::<Controller>().get_meta(),
                            Response::Metadata(track) => {
                                this.now_playing.update(cx, |np, cx| {
                                    let track = track.clone();
                                    np.update_meta(
                                        cx,
                                        track.title.into(),
                                        track.album.into(),
                                        track.artists.iter().map(|s| s.clone().into()).collect(),
                                        track.duration,
                                    );
                                });
                            }
                            Response::Thumbnail(thumbnail) => {
                                this.now_playing.update(cx, |np, cx| {
                                    np.update_thumbnail(cx, Thumbnail {
                                        img: ImageSource::Render(
                                            RenderImage::new(thumbnail.clone().to_frame()).into(),
                                        ),
                                        width: thumbnail.width,
                                        height: thumbnail.height,
                                    });
                                });
                            }
                            Response::StateChanged(state) => {
                                this.now_playing.update(cx, |np, cx| {
                                    np.update_state(cx, state.clone());
                                });
                            }
                            Response::Tracks(tracks) => this.now_playing.update(cx, |np, cx| {
                                let mut np_tracks = vec![];
                                for track in tracks {
                                    if let Some(thumbnail) = track.thumbnail.clone() {
                                        np_tracks.push(Track {
                                            album: track.album.clone(),
                                            artists: track.artists.clone(),
                                            duration: track.duration,
                                            thumbnail: Some(Thumbnail {
                                                img: ImageSource::Render(
                                                    RenderImage::new(thumbnail.to_frame()).into(),
                                                ),
                                                width: thumbnail.width,
                                                height: thumbnail.height,
                                            }),
                                            title: track.title.clone(),
                                            uri: track.uri.clone(),
                                        });
                                    }
                                }
                                np.update_tracks(cx, np_tracks);
                            }),
                            Response::SavedPlaylists(playlists) => {
                                saved_playlists.update(cx, |this, _| {
                                    *this = playlists.clone();
                                })
                            }
                            _ => {}
                        },
                    )
                    .detach();
                    let layout = cx.new(|_| Layout::new());

                    let titlebar = cx.new(|_| Titlebar::new(np.clone(), layout.clone()));

                    let control_bar = cx.new(|_| ControlBar::new(np.clone(), vol_slider.clone()));
                    let main_view = cx.new(|_| MainView::new(np.clone(), layout.clone()));
                    let queue_list = cx.new(|_| QueueList::new(np.clone(), layout.clone()));
                    let layout_sidebar = layout.clone();
                    let left_sidebar = cx.new(move |cx| {
                        LeftSidebar::new(cx, playlists.clone(), layout_sidebar.clone())
                    });
                    cx.global::<Controller>().load_saved_playlists();

                    Reyvr {
                        layout,
                        now_playing: np,
                        titlebar,
                        res_handler,
                        left_sidebar,
                        control_bar,
                        main_view,
                        queue_list,
                    }
                })
            },
        )
        .unwrap();
    });
    Ok(())
}
