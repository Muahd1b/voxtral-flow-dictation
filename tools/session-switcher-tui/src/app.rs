use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use anyhow::{anyhow, Result};
use chrono::Local;

use crate::asr::{self, VoxtralConfig};
use crate::audio::{self, ActiveRecording};
use crate::config::{self, Profile};
use crate::inject;
use crate::transform;
use crate::util::truncate;

const MAX_TALK_LOGS: usize = 500;
const MAX_RUNTIME_LOGS: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Top,
    Middle,
    Bottom,
}

impl FocusPane {
    pub fn next(self) -> Self {
        match self {
            Self::Top => Self::Middle,
            Self::Middle => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

#[derive(Debug)]
pub enum WorkerEvent {
    Runtime(String),
    Partial(String),
    ClearPartial,
    Injected {
        raw_transcript: String,
        final_text: String,
        front_app: String,
        chunks: usize,
        duration_ms: u128,
    },
    Error(String),
    JobFinished,
}

pub struct App {
    pub focus: FocusPane,
    pub profile: Profile,
    pub profile_path: PathBuf,
    pub voxtral: VoxtralConfig,
    pub ffmpeg_bin: String,

    pub talk_logs: VecDeque<String>,
    pub runtime_logs: VecDeque<String>,
    pub live_partial: Option<String>,
    pub last_injected_app: Option<String>,

    pub recording: Option<ActiveRecording>,
    pub record_started_at: Option<Instant>,
    pub jobs_inflight: usize,
    pub global_ptt_running: bool,

    global_ptt_child: Option<Child>,
    tx: Sender<WorkerEvent>,
    rx: Receiver<WorkerEvent>,
}

impl App {
    pub fn new() -> Result<Self> {
        let (profile, profile_path) = config::load_or_create_profile()?;
        let voxtral = VoxtralConfig::from_env(&profile.asr_language);
        let ffmpeg_bin = std::env::var("ASR_FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".to_string());

        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            focus: FocusPane::Top,
            profile,
            profile_path,
            voxtral,
            ffmpeg_bin,
            talk_logs: VecDeque::new(),
            runtime_logs: VecDeque::new(),
            live_partial: None,
            last_injected_app: None,
            recording: None,
            record_started_at: None,
            jobs_inflight: 0,
            global_ptt_running: false,
            global_ptt_child: None,
            tx,
            rx,
        };

        app.push_runtime("ASR Switch started (local monolith, voxtral backend)");
        match app.voxtral.validate() {
            Ok(_) => app.push_runtime("Voxtral backend ready"),
            Err(err) => app.push_runtime(format!("Voxtral readiness warning: {}", err)),
        }
        if let Err(err) = app.start_global_ptt() {
            app.push_runtime(format!("Global PTT startup failed: {}", err));
        }
        Ok(app)
    }

    pub fn drain_worker_events(&mut self) {
        self.check_global_ptt_health();
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                WorkerEvent::Runtime(v) => self.push_runtime(v),
                WorkerEvent::Partial(v) => self.live_partial = Some(v),
                WorkerEvent::ClearPartial => self.live_partial = None,
                WorkerEvent::Injected {
                    raw_transcript,
                    final_text,
                    front_app,
                    chunks,
                    duration_ms,
                } => {
                    self.last_injected_app = Some(front_app.clone());
                    self.push_talk(format!(
                        "Transcript: \"{}\" -> \"{}\"",
                        truncate(&raw_transcript, 120),
                        truncate(&final_text, 120)
                    ));
                    self.push_talk(format!(
                        "Injected into focused app '{}' ({} chunks, {} ms)",
                        front_app, chunks, duration_ms
                    ));
                }
                WorkerEvent::Error(v) => self.push_talk(format!("ERROR: {}", v)),
                WorkerEvent::JobFinished => {
                    self.jobs_inflight = self.jobs_inflight.saturating_sub(1);
                    self.live_partial = None;
                }
            }
        }
    }

    pub fn push_talk(&mut self, msg: impl Into<String>) {
        self.talk_logs
            .push_back(format!("[{}] {}", now_hms(), msg.into()));
        while self.talk_logs.len() > MAX_TALK_LOGS {
            self.talk_logs.pop_front();
        }
    }

    pub fn push_runtime(&mut self, msg: impl Into<String>) {
        self.runtime_logs
            .push_back(format!("[{}] {}", now_hms(), msg.into()));
        while self.runtime_logs.len() > MAX_RUNTIME_LOGS {
            self.runtime_logs.pop_front();
        }
    }

    pub fn reload_profile(&mut self) -> Result<()> {
        let (profile, path) = config::load_or_create_profile()?;
        self.profile = profile;
        self.profile_path = path;
        self.push_runtime("Profile reloaded");
        Ok(())
    }

    pub fn save_profile(&mut self) -> Result<()> {
        self.profile.ptt_hotkey = config::normalize_ptt_hotkey(&self.profile.ptt_hotkey);
        config::save_profile(&self.profile_path, &self.profile)?;
        Ok(())
    }

    pub fn cycle_ptt_hotkey(&mut self) -> Result<()> {
        let current = config::normalize_ptt_hotkey(&self.profile.ptt_hotkey);
        self.profile.ptt_hotkey = match current.as_str() {
            "F8" => "F9".to_string(),
            "F9" => "F10".to_string(),
            "F10" => "F11".to_string(),
            "F11" => "F12".to_string(),
            "F12" => "RIGHT_SHIFT".to_string(),
            _ => "F8".to_string(),
        };
        self.save_profile()?;
        self.push_runtime(format!("PTT hotkey set to {}", self.profile.ptt_hotkey));

        if self.global_ptt_running {
            self.stop_global_ptt();
            self.start_global_ptt()?;
            self.push_runtime("Global PTT daemon restarted with new hotkey");
        }
        Ok(())
    }

    pub fn start_push_to_talk(&mut self) -> Result<()> {
        if self.recording.is_some() {
            return Ok(());
        }

        let session =
            audio::start_push_to_talk_recording(&self.ffmpeg_bin, &self.profile.mic_device_index)?;
        self.recording = Some(session);
        self.record_started_at = Some(Instant::now());
        self.push_talk("Recording... release Space to transcribe+inject");
        self.live_partial = Some("Listening...".to_string());
        Ok(())
    }

    pub fn stop_push_to_talk_and_process(&mut self) -> Result<()> {
        let Some(recording) = self.recording.take() else {
            return Ok(());
        };

        let elapsed_ms = self
            .record_started_at
            .take()
            .map(|i| i.elapsed().as_millis())
            .unwrap_or_default();
        self.push_talk(format!(
            "Recording stopped ({} ms), transcribing...",
            elapsed_ms
        ));

        let wav_path = audio::stop_push_to_talk_recording(recording)?;
        self.dispatch_transcription_job(wav_path);
        Ok(())
    }

    pub fn trigger_single_shot(&mut self, seconds: u64) -> Result<()> {
        if self.recording.is_some() {
            return Err(anyhow!("Already recording"));
        }
        self.push_talk(format!("Single-shot: recording {}s", seconds));

        let rec =
            audio::start_push_to_talk_recording(&self.ffmpeg_bin, &self.profile.mic_device_index)?;
        thread::sleep(std::time::Duration::from_secs(seconds));
        let wav_path = audio::stop_push_to_talk_recording(rec)?;
        self.dispatch_transcription_job(wav_path);
        Ok(())
    }

    pub fn command_mode_rewrite_selected(&mut self) -> Result<()> {
        let mode = self.profile.rewrite_mode;
        let result = inject::rewrite_selected_text(self.profile.inject_app, mode)?;
        self.push_talk(format!(
            "Command mode replaced selected text in '{}' ({} -> {} chars, mode={})",
            result.front_app,
            result.before_chars,
            result.after_chars,
            mode.label()
        ));
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.stop_global_ptt();
        if let Some(recording) = self.recording.take() {
            if let Ok(path) = audio::stop_push_to_talk_recording(recording) {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn dispatch_transcription_job(&mut self, wav_path: PathBuf) {
        if self.jobs_inflight > 0 {
            let _ = fs::remove_file(&wav_path);
            self.push_talk("Skipped transcription: previous job still running");
            self.live_partial = None;
            return;
        }

        self.jobs_inflight += 1;

        let tx = self.tx.clone();
        let profile = self.profile.clone();
        let voxtral = self.voxtral.clone();

        thread::spawn(move || {
            let _ = tx.send(WorkerEvent::Partial(
                "Transcribing with voxtral...".to_string(),
            ));
            let started = Instant::now();

            let result = (|| -> Result<WorkerEvent> {
                let raw = asr::transcribe_file(&voxtral, &wav_path)?;
                let final_text = transform::apply_pipeline(&raw, &profile);
                if final_text.trim().is_empty() {
                    return Err(anyhow!("Transcript became empty after transforms"));
                }
                let injected = inject::inject_focused_text(
                    &final_text,
                    profile.inject_app,
                    profile.chunk_chars,
                )?;
                Ok(WorkerEvent::Injected {
                    raw_transcript: raw,
                    final_text,
                    front_app: injected.front_app,
                    chunks: injected.chunks,
                    duration_ms: started.elapsed().as_millis(),
                })
            })();

            let _ = fs::remove_file(&wav_path);

            match result {
                Ok(ev) => {
                    let _ = tx.send(ev);
                }
                Err(err) => {
                    let _ = tx.send(WorkerEvent::Error(err.to_string()));
                }
            }

            let _ = tx.send(WorkerEvent::ClearPartial);
            let _ = tx.send(WorkerEvent::JobFinished);
        });
    }

    pub fn voxtral_instances(&self) -> usize {
        self.voxtral.running_instances()
    }

    pub fn recording_active(&self) -> bool {
        self.recording.is_some()
    }

    pub fn recording_elapsed_ms(&self) -> u128 {
        self.record_started_at
            .map(|s| s.elapsed().as_millis())
            .unwrap_or(0)
    }

    pub fn toggle_global_ptt(&mut self) -> Result<()> {
        if self.global_ptt_running {
            self.stop_global_ptt();
            Ok(())
        } else {
            self.start_global_ptt()
        }
    }

    pub fn start_global_ptt(&mut self) -> Result<()> {
        if self.global_ptt_running {
            return Ok(());
        }
        let exe = std::env::current_exe()?;
        let mut child = Command::new(exe)
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed starting global PTT daemon: {}", e))?;

        thread::sleep(std::time::Duration::from_millis(120));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(anyhow!("Global PTT daemon exited immediately: {}", status));
        }

        let stderr = child.stderr.take();
        let tx = self.tx.clone();
        if let Some(stderr) = stderr {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx.send(WorkerEvent::Runtime(format!("[global] {}", line)));
                }
            });
        }

        self.global_ptt_running = true;
        self.global_ptt_child = Some(child);
        self.push_runtime("Global PTT enabled (hotkey daemon)");
        Ok(())
    }

    pub fn stop_global_ptt(&mut self) {
        if let Some(mut child) = self.global_ptt_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if self.global_ptt_running {
            self.push_runtime("Global PTT disabled");
        }
        self.global_ptt_running = false;
    }

    fn check_global_ptt_health(&mut self) {
        if let Some(child) = self.global_ptt_child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.global_ptt_running = false;
                    self.global_ptt_child = None;
                    self.push_runtime(format!("Global PTT daemon exited: {}", status));
                }
                Ok(None) => {}
                Err(err) => {
                    self.global_ptt_running = false;
                    self.global_ptt_child = None;
                    self.push_runtime(format!("Global PTT daemon health check failed: {}", err));
                }
            }
        }
    }
}

fn now_hms() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
