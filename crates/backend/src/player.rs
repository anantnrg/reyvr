use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use gstreamer::State;
use image::Frame;
use ring_channel::{RingReceiver as Receiver, RingSender as Sender};
use smallvec::SmallVec;

use crate::{
    Backend,
    playback::{Playlist, SavedPlaylist, SavedPlaylists, Track},
};

pub enum Command {
    Play,
    Pause,
    Volume(f64),
    GetMeta,
    GetTracks,
    Next,
    Previous,
    Seek(u64),
    PlayId(usize),
    LoadFromFolder(String),
    LoadFolder,
    LoadSavedPlaylists,
    WriteSavedPlaylists,
    AddSavedPlaylist(SavedPlaylist),
}

#[derive(Clone)]
pub enum Response {
    Error(String),
    Warning(String),
    Info(String),
    Metadata(Track),
    StateChanged(State),
    Eos,
    StreamStart,
    Position(u64),
    Thumbnail(Thumbnail),
    Tracks(Vec<Track>),
    SavedPlaylists(SavedPlaylists),
}

#[derive(Clone)]
pub struct Player {
    pub backend: Arc<dyn Backend>,
    pub playlist: Arc<Mutex<Playlist>>,
    pub volume: f64,
    pub position: u64,
    pub current_index: usize,
    pub loaded: bool,
    pub playing: bool,
    pub saved_playlists: SavedPlaylists,
    pub tx: Sender<Response>,
    pub rx: Receiver<Command>,
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub tx: Sender<Command>,
    pub rx: Receiver<Response>,
}

#[derive(Clone)]
pub struct Thumbnail {
    pub img: SmallVec<[Frame; 1]>,
    pub width: u32,
    pub height: u32,
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
                current_index: 0,
                loaded: false,
                playing: false,
                saved_playlists: SavedPlaylists::default(),
                tx: res_tx,
                rx: cmd_rx,
            },
            Controller {
                tx: cmd_tx,
                rx: res_rx,
            },
        )
    }

    pub fn set_playing(&mut self) {
        self.playing = !self.playing;
    }

    pub async fn play_next(&mut self, backend: &Arc<dyn Backend>) -> anyhow::Result<()> {
        let tracks_len = {
            let guard = self.playlist.lock().expect("Could not lock playlist");
            guard.tracks.len()
        };

        if self.current_index + 1 < tracks_len {
            self.current_index += 1;
            {
                let mut cloned_playlist = {
                    let guard = self.playlist.lock().expect("Could not lock playlist");
                    guard.clone()
                };
                cloned_playlist.load(backend, self.current_index).await?;
                self.playlist = Arc::new(Mutex::new(cloned_playlist));
            }
        }
        Ok(())
    }

    pub async fn play_previous(&mut self, backend: &Arc<dyn Backend>) -> anyhow::Result<()> {
        if self.current_index > 0 {
            self.current_index -= 1;
            {
                let mut cloned_playlist = {
                    let guard = self.playlist.lock().expect("Could not lock playlist");
                    guard.clone()
                };
                cloned_playlist.load(backend, self.current_index).await?;
                self.playlist = Arc::new(Mutex::new(cloned_playlist));
            }
        }
        Ok(())
    }

    pub async fn play_id(&mut self, backend: &Arc<dyn Backend>, id: usize) -> anyhow::Result<()> {
        self.current_index = id;
        let uri = {
            let guard = self.playlist.lock().expect("Could not lock playlist");
            guard.tracks[id].uri.clone()
        };
        backend.load(&uri).await?;
        Ok(())
    }

    pub async fn run(&mut self) {
        loop {
            while let Ok(command) = self.rx.try_recv() {
                match command {
                    Command::Play => {
                        let cloned_playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        let backend = self.backend.clone();
                        if !cloned_playlist.tracks.is_empty() {
                            if !self.playing {
                                if self.loaded {
                                    let tx = self.tx.clone();
                                    self.tx
                                        .send(Response::StateChanged(State::Playing))
                                        .expect("Could not send message");
                                    let _ = backend
                                        .play()
                                        .await
                                        .map_err(|e| tx.send(Response::Error(e.to_string())));
                                    self.playing = true;
                                } else {
                                    println!("Playlist is not loaded.");
                                    self.tx
                                        .send(Response::Error(
                                            "Playlist is not loaded.".to_string(),
                                        ))
                                        .expect("Could not send message");
                                }
                            }
                        }
                        self.playlist = Arc::new(Mutex::new(cloned_playlist));
                    }
                    Command::Pause => {
                        let backend = self.backend.clone();
                        if self.playing {
                            self.tx
                                .send(Response::StateChanged(State::Paused))
                                .expect("Could not send message");
                            let _ = backend
                                .pause()
                                .await
                                .map_err(|e| self.tx.send(Response::Error(e.to_string())));
                            self.playing = false;
                        }
                    }
                    Command::GetMeta => {
                        let cloned_playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        if self.loaded {
                            let track = cloned_playlist.tracks[self.current_index].clone();
                            self.tx
                                .send(Response::Metadata(track))
                                .expect("Could not send message");
                        }
                    }
                    Command::GetTracks => {
                        let cloned_playlist = {
                            let guard = self.playlist.lock().expect("Could not lock playlist");
                            guard.clone()
                        };
                        if self.loaded {
                            let tracks = cloned_playlist.tracks.clone();
                            self.tx
                                .send(Response::Tracks(tracks))
                                .expect("Could not send message");
                        }
                    }
                    Command::Volume(vol) => {
                        let backend = self.backend.clone();
                        if self.loaded {
                            self.tx
                                .send(Response::Info(format!("Volume set to {vol}")))
                                .expect("Could not send message");
                            backend.set_volume(vol).await.expect("Could not set volume");
                            println!("Volume set to {vol}");
                            self.volume = vol;
                        }
                    }
                    Command::Next => {
                        let backend = self.backend.clone();
                        if self.loaded {
                            backend.stop().await.expect("Could not stop");
                            self.play_next(&backend)
                                .await
                                .expect("Could not play next.");
                            self.tx
                                .send(Response::StateChanged(State::Playing))
                                .expect("Could not send message");
                            backend.play().await.expect("Could not play");
                            self.playing = true;
                            backend
                                .set_volume(self.volume)
                                .await
                                .expect("Could not set volume");
                        }
                    }
                    Command::Previous => {
                        let backend = self.backend.clone();
                        if self.loaded {
                            backend.stop().await.expect("Could not stop");
                            self.play_previous(&backend)
                                .await
                                .expect("Could not play previous.");
                            self.tx
                                .send(Response::StateChanged(State::Playing))
                                .expect("Could not send message");
                            backend.play().await.expect("Could not play");
                            self.playing = true;
                            backend
                                .set_volume(self.volume)
                                .await
                                .expect("Could not set volume");
                        }
                    }
                    Command::PlayId(id) => {
                        let backend = self.backend.clone();
                        if self.loaded {
                            backend.stop().await.expect("Could not stop");
                            self.play_id(&backend, id)
                                .await
                                .expect("Could not play track");
                            self.tx
                                .send(Response::StateChanged(State::Playing))
                                .expect("Could not send message");
                            backend.play().await.expect("Could not play");
                            self.playing = true;
                            backend
                                .set_volume(self.volume)
                                .await
                                .expect("Could not set volume");
                        }
                    }
                    Command::LoadFromFolder(path) => {
                        let backend = self.backend.clone();
                        let mut playlist = Playlist::from_dir(&backend, PathBuf::from(path)).await;
                        playlist
                            .load(&backend, 0)
                            .await
                            .expect("Could not load first item");
                        self.loaded = true;
                        self.playlist = Arc::new(Mutex::new(playlist));
                    }
                    Command::LoadFolder => {
                        let backend = self.backend.clone();
                        if let Some(path) = rfd::AsyncFileDialog::new().pick_folder().await {
                            let mut playlist =
                                Playlist::from_dir(&backend, PathBuf::from(path.path().to_owned()))
                                    .await;
                            playlist
                                .load(&backend, 0)
                                .await
                                .expect("Could not load first item");
                            self.loaded = true;
                            self.playlist = Arc::new(Mutex::new(playlist));
                        }
                    }
                    Command::LoadSavedPlaylists => {
                        self.saved_playlists = SavedPlaylists::load();
                        self.tx
                            .send(Response::SavedPlaylists(self.saved_playlists.clone()))
                            .expect("Could not send message");
                    }
                    Command::WriteSavedPlaylists => {
                        SavedPlaylists::save_playlists(&self.saved_playlists)
                            .expect("Could not save to file");
                    }
                    Command::AddSavedPlaylist(playlist) => {
                        self.saved_playlists.playlists.push(playlist);
                    }
                    Command::Seek(time) => {
                        let backend = self.backend.clone();
                        if self.playing {
                            backend.seek(time).await.expect("Could not seek");
                        }
                    }
                }
            }

            if let Some(res) = self.backend.monitor().await {
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
    pub fn load(&self, path: String) {
        self.tx
            .send(Command::LoadFromFolder(path))
            .expect("Could not send command");
    }

    pub fn open_folder(&self) {
        self.tx
            .send(Command::LoadFolder)
            .expect("Could not send command");
    }

    pub fn play(&self) {
        self.tx.send(Command::Play).expect("Could not send command");
    }

    pub fn play_id(&self, id: usize) {
        self.tx
            .send(Command::PlayId(id))
            .expect("Could not send command");
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

    pub fn get_meta(&self) {
        self.tx
            .send(Command::GetMeta)
            .expect("Could not send command");
    }

    pub fn get_queue(&self) {
        self.tx
            .send(Command::GetTracks)
            .expect("Could not send command");
    }

    pub fn volume(&self, vol: f64) {
        self.tx
            .send(Command::Volume(vol))
            .expect("Could not send command");
    }

    pub fn load_saved_playlists(&self) {
        self.tx
            .send(Command::LoadSavedPlaylists)
            .expect("Could not send command");
    }

    pub fn save_playlist(&self) {}
}
