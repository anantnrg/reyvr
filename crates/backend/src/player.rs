use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use gstreamer::State;
use ring_channel::{RingReceiver as Receiver, RingSender as Sender};

use crate::{
    Backend, PlaybackState,
    playback::{Playlist, Track},
};

pub enum Command {
    Play,
    Pause,
    Volume(f64),
    GetMeta,
    Next,
    Previous,
    LoadFromFolder(String),
}

#[derive(Debug, Clone)]
pub enum Response {
    Error(String),
    Warning(String),
    Info(String),
    Metadata(Track),
    StateChanged(State),
    Eos,
    StreamStart,
    Position(u64),
}

#[derive(Debug, Clone)]
pub struct Player {
    pub backend: Arc<dyn Backend>,
    pub playlist: Arc<Mutex<Playlist>>,
    pub volume: f64,
    pub position: u64,
    pub tx: Sender<Response>,
    pub rx: Receiver<Command>,
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub tx: Sender<Command>,
    pub rx: Receiver<Response>,
}

impl gpui::Global for Controller {}

impl Player {
    pub fn new(backend: Arc<dyn Backend>, playlist: Arc<Mutex<Playlist>>) -> (Player, Controller) {
        let (cmd_tx, cmd_rx) = ring_channel::ring_channel(NonZeroUsize::new(128).unwrap());
        let (res_tx, res_rx) = ring_channel::ring_channel(NonZeroUsize::new(128).unwrap());
        (
            Player {
                backend,
                playlist,
                volume: 0.5,
                position: 0,
                tx: res_tx,
                rx: cmd_rx,
            },
            Controller {
                tx: cmd_tx,
                rx: res_rx,
            },
        )
    }

    pub async fn run(&mut self) {
        self.tx.send(Response::Eos).unwrap();
        loop {
            while let Ok(command) = self.rx.try_recv() {
                match command {
                    Command::Play => {
                        let mut playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        let backend = self.backend.clone();

                        if !playlist.tracks.is_empty() {
                            if !playlist.playing {
                                if playlist.loaded {
                                    let tx = self.tx.clone();

                                    backend
                                        .play()
                                        .await
                                        .map_err(|e| tx.send(Response::Error(e.to_string())))
                                        .expect("Could not play");

                                    playlist.playing = true;
                                } else {
                                    println!("Playlist is not loaded.");
                                    self.tx
                                        .send(Response::Error(
                                            "Playlist is not loaded.".to_string(),
                                        ))
                                        .expect("Could not send message");
                                }
                                self.tx
                                    .send(Response::Info("Playback started.".to_string()))
                                    .expect("Could not send message");
                            }
                        }
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                    Command::Pause => {
                        let mut playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        let backend = self.backend.clone();

                        if playlist.playing {
                            backend
                                .pause()
                                .await
                                .map_err(|e| self.tx.send(Response::Error(e.to_string())))
                                .expect("Could not pause playback");
                            playlist.playing = false;
                        }
                        self.tx
                            .send(Response::Info("Playback paused.".to_string()))
                            .expect("Could not send message");
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                    Command::GetMeta => {
                        let playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };

                        if playlist.loaded {
                            let track = playlist.tracks[playlist.current_index].clone();
                            self.tx
                                .send(Response::Metadata(track))
                                .expect("Could not send message");
                        }
                    }
                    Command::Volume(vol) => {
                        let playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        let backend = self.backend.clone();

                        if playlist.loaded {
                            backend.set_volume(vol).await.expect("Could not set volume");
                            println!("Volume set to {vol}");
                            self.volume = vol;
                            self.tx
                                .send(Response::Info(format!("Volume set to {vol}").to_string()))
                                .expect("Could not send message");
                        }
                    }
                    Command::Next => {
                        let mut playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        let backend = self.backend.clone();

                        if playlist.loaded {
                            if playlist.playing {
                                backend.stop().await.expect("Could not pause");
                                playlist
                                    .play_next(&backend)
                                    .await
                                    .expect("Could not play next.");
                                backend.play().await.expect("Could not pause");
                                backend
                                    .set_volume(self.volume)
                                    .await
                                    .expect("Could not set volume");
                            } else {
                                playlist
                                    .play_next(&backend)
                                    .await
                                    .expect("Could not play next.");
                                backend.play().await.expect("Could not stop");
                                backend
                                    .set_volume(self.volume)
                                    .await
                                    .expect("Could not set volume");
                            }
                        }
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                    Command::Previous => {
                        let mut playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        let backend = self.backend.clone();
                        if playlist.loaded {
                            if playlist.playing {
                                backend.stop().await.expect("Could not stop");
                                playlist
                                    .play_previous(&backend)
                                    .await
                                    .expect("Could not play next.");
                                backend.play().await.expect("Could not pause");
                                backend
                                    .set_volume(self.volume)
                                    .await
                                    .expect("Could not set volume");
                            } else {
                                playlist
                                    .play_previous(&backend)
                                    .await
                                    .expect("Could not play next.");
                                backend.play().await.expect("Could not pause");
                                backend
                                    .set_volume(self.volume)
                                    .await
                                    .expect("Could not set volume");
                            }
                        }
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                    Command::LoadFromFolder(path) => {
                        let backend = self.backend.clone();
                        let mut playlist = Playlist::from_dir(&backend, PathBuf::from(path)).await;

                        playlist
                            .load(&backend)
                            .await
                            .expect("Could not load first item");
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                }
            }
            if let Some(res) = self.backend.monitor().await {
                println!("response from gstreamer: {:#?}", res);
                self.tx.send(res).unwrap();
            }
            let curr_pos = self.backend.get_position().await;
            if self.position != curr_pos {
                self.tx
                    .send(Response::Position(curr_pos))
                    .expect("Could not send message.");

                self.position = curr_pos;
            }
        }
    }
}

impl Controller {
    pub fn load(&self, path: &'static str) {
        self.tx
            .send(Command::LoadFromFolder(path.to_string()))
            .expect("Could not send command");
    }

    pub fn play(&self) {
        self.tx.send(Command::Play).expect("Could not send command");
    }

    pub fn pause(&self) {
        self.tx
            .send(Command::Pause)
            .expect("Could not send command");
    }

    pub fn next(&self) {
        self.tx.send(Command::Next).expect("Could not send command");
    }

    pub fn prev(&self) {
        self.tx
            .send(Command::Previous)
            .expect("Could not send command");
    }

    pub fn volume(&self, vol: f64) {
        self.tx
            .send(Command::Volume(vol))
            .expect("Could not send command");
        // println!("Volume set to {vol}");
    }
}
