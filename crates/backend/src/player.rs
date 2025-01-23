use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ring_channel::{RingReceiver as Receiver, RingSender as Sender};

use crate::{
    Backend,
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
    Success(String),
    Metadata(Track),
}

#[derive(Debug, Clone)]
pub struct Player {
    pub backend: Arc<dyn Backend>,
    pub playlist: Arc<Mutex<Playlist>>,
    pub volume: f64,
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
                                if !playlist.loaded {
                                    playlist
                                        .load(&backend)
                                        .await
                                        .map_err(|e| self.tx.send(Response::Error(e.to_string())))
                                        .expect("Could not load track.");
                                    let tx = self.tx.clone();

                                    backend
                                        .play()
                                        .await
                                        .map_err(|e| tx.send(Response::Error(e.to_string())))
                                        .expect("Could not play");

                                    playlist.playing = true;
                                } else {
                                    let tx = self.tx.clone();

                                    backend
                                        .play()
                                        .await
                                        .map_err(|e| tx.send(Response::Error(e.to_string())))
                                        .expect("Could not play");

                                    playlist.playing = true;
                                }
                                self.tx
                                    .send(Response::Success("Playback started.".to_string()))
                                    .expect("Could not send message");
                            }
                        } else {
                            println!("Playlist is not loaded.");
                            self.tx
                                .send(Response::Error("Playlist is not loaded.".to_string()))
                                .expect("Could not send message");
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
                            .send(Response::Success("Playback paused.".to_string()))
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
                            self.tx
                                .send(Response::Success(
                                    format!("Volume set to {vol}").to_string(),
                                ))
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
                                println!("index was: {}", playlist.current_index);
                                backend.pause().await.expect("Could not pause");
                                playlist
                                    .play_next(&backend)
                                    .await
                                    .expect("Could not play next.");
                                println!("index is: {}", playlist.current_index);
                                backend.play().await.expect("Could not pause");
                            } else {
                                playlist
                                    .play_next(&backend)
                                    .await
                                    .expect("Could not play next.");
                                backend.play().await.expect("Could not pause");
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
                            playlist
                                .play_previous(&backend)
                                .await
                                .expect("Could not play previous.");
                        }
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                    Command::LoadFromFolder(path) => {
                        let backend = self.backend.clone();
                        self.playlist = Arc::new(Mutex::new(
                            Playlist::from_dir(&backend, PathBuf::from(path)).await,
                        ));
                    }
                }
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
