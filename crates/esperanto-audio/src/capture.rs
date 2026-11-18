//! Microphone capture: a stream thread owns the `cpal` stream (it isn't
//! `Send` on macOS) and a DSP thread converts the audio to 16 kHz mono and
//! passes it to a [`CaptureHandler`].

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::AudioError;
use crate::devices::find_input_device;
use crate::resample::ToMono16k;

/// Ring buffer capacity, in seconds of captured audio.
const BUFFER_SECS: u32 = 2;
const DSP_IDLE: Duration = Duration::from_millis(5);

/// Throughput of the DSP thread, logged every few seconds at debug level,
/// with a warning when processing blocks (audio would be dropped).
#[derive(Default)]
struct DspStats {
    since: Option<std::time::Instant>,
    input: usize,
    calls: usize,
    slowest: Duration,
}

impl DspStats {
    const EVERY: Duration = Duration::from_secs(5);
    const SLOW: Duration = Duration::from_millis(200);

    fn record(&mut self, took: Duration) {
        self.calls += 1;
        self.slowest = self.slowest.max(took);
        if took > Self::SLOW {
            log::warn!("audio processing blocked for {took:?}");
        }
    }

    fn maybe_log(&mut self, overruns: &AtomicUsize) {
        let since = *self.since.get_or_insert_with(std::time::Instant::now);
        if since.elapsed() < Self::EVERY {
            return;
        }
        log::debug!(
            "audio: {} input samples in {:?}, {} handler calls (slowest {:?}), {} samples dropped so far",
            self.input,
            since.elapsed(),
            self.calls,
            self.slowest,
            overruns.load(Ordering::Relaxed)
        );
        *self = Self {
            since: Some(std::time::Instant::now()),
            ..Self::default()
        };
    }
}

/// Receives 16 kHz mono audio on the DSP thread.
pub trait CaptureHandler: Send + 'static {
    fn samples(&mut self, samples: &[f32]);
    /// The stream failed (e.g. the device was unplugged). Capture stops.
    fn error(&mut self, error: AudioError);
    /// Capture was stopped normally; flush any pending work.
    fn finished(&mut self) {}
}

/// A running capture. Dropping it stops the stream and joins its threads.
pub struct Capture {
    stop: Arc<AtomicBool>,
    stream_thread: Option<JoinHandle<()>>,
    dsp_thread: Option<JoinHandle<()>>,
    overruns: Arc<AtomicUsize>,
    device_name: String,
}

impl Capture {
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Samples dropped because the DSP thread fell behind.
    pub fn overruns(&self) -> usize {
        self.overruns.load(Ordering::Relaxed)
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.stream_thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
        if let Some(thread) = self.dsp_thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Opened {
    consumer: Consumer<f32>,
    sample_rate: u32,
    channels: usize,
    device_name: String,
}

/// Device id that captures the system audio (what the Mac is playing on its
/// default output) instead of an input device.
pub const SYSTEM_AUDIO: &str = "system:output";

/// Device ids with this prefix replay a WAV file in real time instead of
/// capturing a device (`file:/path/to/clip.wav`). For tests and debugging.
pub const REPLAY_PREFIX: &str = "file:";

/// Starts capturing `device_id` (or the default input device).
pub fn start_capture(
    device_id: Option<&str>,
    handler: impl CaptureHandler,
) -> Result<Capture, AudioError> {
    if let Some(path) = device_id.and_then(|id| id.strip_prefix(REPLAY_PREFIX)) {
        return start_replay(std::path::Path::new(path), handler);
    }
    start_device(device_id, handler)
}

/// Feeds a WAV file to `handler` at real-time speed, then silence until stopped.
fn start_replay(
    path: &std::path::Path,
    mut handler: impl CaptureHandler,
) -> Result<Capture, AudioError> {
    let samples = crate::wav::load_wav(path)?.samples;

    let stop = Arc::new(AtomicBool::new(false));
    let thread = {
        let stop = Arc::clone(&stop);
        thread::Builder::new()
            .name("esperanto-audio-replay".into())
            .spawn(move || {
                const CHUNK: usize = 160; // 10 ms
                let silence = [0.0_f32; CHUNK];
                let started = std::time::Instant::now();
                let mut sent = 0_usize;
                let mut chunks = samples.chunks(CHUNK);
                while !stop.load(Ordering::Relaxed) {
                    let chunk = chunks.next().unwrap_or(&silence);
                    handler.samples(chunk);
                    sent += CHUNK;
                    let due = Duration::from_secs_f64(sent as f64 / 16_000.0);
                    if let Some(wait) = due.checked_sub(started.elapsed()) {
                        thread::sleep(wait);
                    }
                }
                handler.finished();
            })
            .map_err(|e| AudioError::Stream(e.to_string()))?
    };
    Ok(Capture {
        stop,
        stream_thread: None,
        dsp_thread: Some(thread),
        overruns: Arc::default(),
        device_name: path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into(),
        ),
    })
}

fn start_device(
    device_id: Option<&str>,
    mut handler: impl CaptureHandler,
) -> Result<Capture, AudioError> {
    let stop = Arc::new(AtomicBool::new(false));
    let overruns = Arc::new(AtomicUsize::new(0));
    let stream_error: Arc<Mutex<Option<String>>> = Arc::default();
    let (opened_tx, opened_rx) = mpsc::channel::<Result<Opened, AudioError>>();

    let stream_thread = {
        let device_id = device_id.map(str::to_owned);
        let stop = Arc::clone(&stop);
        let overruns = Arc::clone(&overruns);
        let stream_error = Arc::clone(&stream_error);
        thread::Builder::new()
            .name("esperanto-audio-stream".into())
            .spawn(move || {
                let stream = match open_stream(device_id.as_deref(), overruns, stream_error) {
                    Ok((stream, opened)) => {
                        let _ = opened_tx.send(Ok(opened));
                        stream
                    }
                    Err(error) => {
                        let _ = opened_tx.send(Err(error));
                        return;
                    }
                };
                while !stop.load(Ordering::Relaxed) {
                    thread::park_timeout(Duration::from_millis(100));
                }
                drop(stream);
            })
            .map_err(|e| AudioError::Stream(e.to_string()))?
    };

    let opened = match opened_rx.recv() {
        Ok(Ok(opened)) => opened,
        Ok(Err(error)) => {
            let _ = stream_thread.join();
            return Err(error);
        }
        Err(_) => return Err(AudioError::Stream("capture thread exited".into())),
    };

    log::info!(
        "capturing {:?}: {} Hz, {} channel(s)",
        opened.device_name,
        opened.sample_rate,
        opened.channels
    );
    let mut converter = ToMono16k::new(opened.sample_rate, opened.channels)?;
    let mut consumer = opened.consumer;
    let overruns_dsp = Arc::clone(&overruns);
    let dsp_thread = {
        let stop = Arc::clone(&stop);
        thread::Builder::new()
            .name("esperanto-audio-dsp".into())
            .spawn(move || {
                let mut interleaved = Vec::new();
                let mut stats = DspStats::default();
                while !stop.load(Ordering::Relaxed) {
                    stats.maybe_log(&overruns_dsp);
                    if let Some(message) = stream_error.lock().ok().and_then(|mut e| e.take()) {
                        handler.error(AudioError::Stream(message));
                        return;
                    }
                    let available = consumer.slots();
                    // Keep whole frames so channels stay aligned.
                    let available = available - available % opened.channels;
                    if available == 0 {
                        thread::sleep(DSP_IDLE);
                        continue;
                    }
                    let Ok(chunk) = consumer.read_chunk(available) else {
                        continue;
                    };
                    interleaved.clear();
                    let (first, second) = chunk.as_slices();
                    interleaved.extend_from_slice(first);
                    interleaved.extend_from_slice(second);
                    chunk.commit_all();
                    stats.input += interleaved.len();
                    let started = std::time::Instant::now();
                    let converted = converter.process(&interleaved);
                    match converted {
                        Ok(samples) if !samples.is_empty() => {
                            // A panic here would otherwise end this thread
                            // silently: capture would freeze while the
                            // session looks alive. Report it instead.
                            let processed =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    handler.samples(samples)
                                }));
                            if processed.is_err() {
                                handler
                                    .error(AudioError::Stream("audio processing panicked".into()));
                                return;
                            }
                            stats.record(started.elapsed());
                        }
                        Ok(_) => {}
                        Err(error) => {
                            handler.error(error);
                            return;
                        }
                    }
                }
                handler.finished();
            })
            .map_err(|e| AudioError::Stream(e.to_string()))?
    };

    Ok(Capture {
        stop,
        stream_thread: Some(stream_thread),
        dsp_thread: Some(dsp_thread),
        overruns,
        device_name: opened.device_name,
    })
}

fn open_stream(
    device_id: Option<&str>,
    overruns: Arc<AtomicUsize>,
    stream_error: Arc<Mutex<Option<String>>>,
) -> Result<(cpal::Stream, Opened), AudioError> {
    // System audio: an input stream on the default *output* device makes
    // cpal create a Core Audio process tap (macOS 14.2+) that records what
    // every app plays on it, without muting it.
    let system = device_id == Some(SYSTEM_AUDIO);
    let device = if system {
        cpal::default_host()
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?
    } else {
        find_input_device(device_id)?
    };
    let device_name = device.to_string();
    let config = if system {
        device.default_output_config()
    } else {
        device.default_input_config()
    }
    .map_err(|e| AudioError::Stream(e.to_string()))?;
    let sample_rate = config.sample_rate();
    let channels = usize::from(config.channels());
    let (producer, consumer) =
        RingBuffer::<f32>::new((sample_rate * BUFFER_SECS) as usize * channels);

    let on_error = move |error: cpal::Error| {
        // Glitches and notifications don't stop the stream; only errors that
        // end it do. (Treating every error as fatal silently stalled system
        // audio capture, whose aggregate device reports changes.)
        use cpal::ErrorKind;
        match error.kind() {
            ErrorKind::Xrun | ErrorKind::RealtimeDenied | ErrorKind::DeviceChanged => {
                log::warn!("audio stream: {error}");
            }
            _ => {
                log::error!("audio stream failed: {error}");
                if let Ok(mut slot) = stream_error.lock() {
                    slot.get_or_insert_with(|| error.to_string());
                }
            }
        }
    };
    let stream_config = config.config();
    let stream = match config.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, &stream_config, producer, overruns, on_error),
        SampleFormat::I16 => build::<i16>(&device, &stream_config, producer, overruns, on_error),
        SampleFormat::I32 => build::<i32>(&device, &stream_config, producer, overruns, on_error),
        SampleFormat::U16 => build::<u16>(&device, &stream_config, producer, overruns, on_error),
        other => Err(AudioError::Stream(format!(
            "unsupported sample format {other}"
        ))),
    }?;
    stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;

    Ok((
        stream,
        Opened {
            consumer,
            sample_rate,
            channels,
            device_name,
        },
    ))
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    overruns: Arc<AtomicUsize>,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            *config,
            // Real-time thread: no allocation, no locks.
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let writable = producer.slots().min(data.len());
                if let Ok(chunk) = producer.write_chunk_uninit(writable) {
                    chunk.fill_from_iter(data.iter().map(|s| f32::from_sample_(*s)));
                }
                if writable < data.len() {
                    overruns.fetch_add(data.len() - writable, Ordering::Relaxed);
                }
            },
            on_error,
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}
