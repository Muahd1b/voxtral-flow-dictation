use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use tempfile::Builder;

pub struct ActiveRecording {
    child: Child,
    wav_path: PathBuf,
}

pub fn start_push_to_talk_recording(
    ffmpeg_bin: &str,
    mic_device_index: &str,
) -> Result<ActiveRecording> {
    let tmp = Builder::new()
        .prefix("asr-ptt-")
        .suffix(".wav")
        .tempfile()
        .context("Failed creating temp wav file")?;
    let (_file, wav_path) = tmp.keep().context("Failed keeping temp wav path")?;

    let mut cmd = Command::new(ffmpeg_bin);
    cmd.arg("-y")
        .arg("-f")
        .arg("avfoundation")
        .arg("-i")
        .arg(format!(":{mic_device_index}"))
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg(&wav_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn ffmpeg from {}", ffmpeg_bin))?;

    Ok(ActiveRecording { child, wav_path })
}

pub fn stop_push_to_talk_recording(mut session: ActiveRecording) -> Result<PathBuf> {
    if let Some(stdin) = session.child.stdin.as_mut() {
        let _ = stdin.write_all(b"q\n");
        let _ = stdin.flush();
    }

    let started = Instant::now();
    loop {
        if let Some(status) = session
            .child
            .try_wait()
            .context("Failed waiting for ffmpeg")?
        {
            if !status.success() {
                return Err(anyhow!("ffmpeg exited with status {}", status));
            }
            break;
        }

        if started.elapsed() > Duration::from_secs(2) {
            let _ = session.child.kill();
            let _ = session.child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(40));
    }

    if !session.wav_path.exists() {
        return Err(anyhow!(
            "Recorded wav file missing: {}",
            session.wav_path.display()
        ));
    }

    Ok(session.wav_path)
}
