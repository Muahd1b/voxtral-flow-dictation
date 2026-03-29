use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};

use crate::paths;
use crate::util::truncate;

#[derive(Debug, Clone)]
pub struct VoxtralConfig {
    pub bin_path: PathBuf,
    pub model_dir: PathBuf,
    pub timeout_sec: u64,
    pub empty_retries: u32,
    pub lock_timeout_ms: u64,
    pub lock_stale_sec: u64,
    pub lock_file: PathBuf,
}

impl VoxtralConfig {
    pub fn from_env(_default_language: &str) -> Self {
        Self {
            bin_path: paths::voxtral_bin_path(),
            model_dir: paths::voxtral_model_dir(),
            timeout_sec: env::var("ASR_VOXTRAL_TIMEOUT_SEC")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(120),
            empty_retries: env::var("ASR_VOXTRAL_EMPTY_RETRIES")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1),
            lock_timeout_ms: env::var("ASR_VOXTRAL_LOCK_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(15_000),
            lock_stale_sec: env::var("ASR_VOXTRAL_LOCK_STALE_SEC")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(900),
            lock_file: paths::voxtral_lock_file(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if !self.bin_path.exists() {
            return Err(anyhow!(
                "Voxtral binary not found: {}",
                self.bin_path.display()
            ));
        }
        let required = [
            self.model_dir.join("consolidated.safetensors"),
            self.model_dir.join("tekken.json"),
            self.model_dir.join("params.json"),
        ];
        for file in required {
            if !file.exists() {
                return Err(anyhow!(
                    "Voxtral model artifact missing: {}",
                    file.display()
                ));
            }
        }
        Ok(())
    }

    pub fn running_instances(&self) -> usize {
        let output = Command::new("pgrep").arg("-x").arg("voxtral").output();
        let Ok(out) = output else {
            return 0;
        };
        if !out.status.success() {
            return 0;
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count()
    }
}

pub fn transcribe_file(cfg: &VoxtralConfig, wav_path: &Path) -> Result<String> {
    cfg.validate()?;
    let _lock = acquire_voxtral_lock(cfg)?;

    let attempts = cfg.empty_retries.saturating_add(1);
    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 1..=attempts {
        match run_voxtral_once(cfg, wav_path) {
            Ok(text) if !text.trim().is_empty() => return Ok(text),
            Ok(_) => {
                last_err = Some(anyhow!(
                    "voxtral returned empty transcript on attempt {attempt}/{attempts}"
                ));
            }
            Err(err) => {
                last_err = Some(anyhow!(
                    "voxtral attempt {attempt}/{attempts} failed: {}",
                    truncate(&err.to_string(), 240)
                ));
            }
        }
        if attempt < attempts {
            thread::sleep(Duration::from_millis(120));
        }
    }

    let wav_size = fs::metadata(wav_path).map(|m| m.len()).unwrap_or(0);
    Err(anyhow!(
        "voxtral produced no usable transcript after {} attempts (wav={} bytes). Last detail: {}",
        attempts,
        wav_size,
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown failure".to_string())
    ))
}

struct VoxtralLockGuard {
    path: PathBuf,
}

impl Drop for VoxtralLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_voxtral_lock(cfg: &VoxtralConfig) -> Result<VoxtralLockGuard> {
    let deadline = Instant::now() + Duration::from_millis(cfg.lock_timeout_ms);
    loop {
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&cfg.lock_file)
        {
            Ok(mut f) => {
                let _ = std::io::Write::write_all(
                    &mut f,
                    format!(
                        "pid={} started_unix={}\n",
                        std::process::id(),
                        chrono::Utc::now().timestamp()
                    )
                    .as_bytes(),
                );
                return Ok(VoxtralLockGuard {
                    path: cfg.lock_file.clone(),
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                if lock_is_stale(&cfg.lock_file, cfg.lock_stale_sec) {
                    let _ = fs::remove_file(&cfg.lock_file);
                    continue;
                }
                if Instant::now() >= deadline {
                    return Err(anyhow!(
                        "Voxtral busy lock timeout after {} ms (lock file: {})",
                        cfg.lock_timeout_ms,
                        cfg.lock_file.display()
                    ));
                }
                thread::sleep(Duration::from_millis(80));
            }
            Err(err) => {
                return Err(anyhow!(
                    "Failed acquiring Voxtral lock {}: {}",
                    cfg.lock_file.display(),
                    err
                ));
            }
        }
    }
}

fn lock_is_stale(path: &Path, stale_sec: u64) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let Ok(elapsed) = modified.elapsed() else {
        return false;
    };
    elapsed > Duration::from_secs(stale_sec)
}

fn run_voxtral_once(cfg: &VoxtralConfig, wav_path: &Path) -> Result<String> {
    let mut cmd = Command::new(&cfg.bin_path);
    cmd.arg("-d")
        .arg(&cfg.model_dir)
        .arg("-i")
        .arg(wav_path)
        .arg("--silent")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Put voxtral into its own process group so we can terminate the full tree
    // on timeout/error and avoid orphaned concurrent instances.
    unsafe {
        cmd.pre_exec(|| {
            let rc = libc::setpgid(0, 0);
            if rc == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed spawning voxtral binary {}", cfg.bin_path.display()))?;
    let child_pid = child.id();

    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(cfg.timeout_sec) {
            terminate_process_group(child_pid);
            let _ = child.wait();
            return Err(anyhow!(
                "voxtral timed out after {}s for {}",
                cfg.timeout_sec,
                wav_path.display()
            ));
        }

        if child
            .try_wait()
            .context("Failed waiting on voxtral")?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .context("Failed collecting voxtral output")?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!(
                    "voxtral exited with {}: {}",
                    output.status,
                    truncate(err.trim(), 240)
                ));
            }
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(text);
        }

        thread::sleep(Duration::from_millis(20));
    }
}

fn terminate_process_group(pid: u32) {
    let pgid = format!("-{}", pid);
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg("--")
        .arg(&pgid)
        .status();
    thread::sleep(Duration::from_millis(60));
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg("--")
        .arg(&pgid)
        .status();
}
