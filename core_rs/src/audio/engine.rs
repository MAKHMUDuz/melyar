use anyhow::{anyhow, Result};
use libmpv_sys as mpv_sys;
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// The band centres of a ten band graphic equaliser, in hertz.
///
/// These are the usual octave centres, so the bands line up with what a
/// hardware equaliser would show.
pub const EQUALIZER_BANDS: [f64; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// MPV Handle Wrapper with proper cleanup
pub struct MpvHandleWrapper {
    raw: *mut mpv_sys::mpv_handle,
}

unsafe impl Send for MpvHandleWrapper {}
unsafe impl Sync for MpvHandleWrapper {}

impl Drop for MpvHandleWrapper {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                mpv_sys::mpv_terminate_destroy(self.raw);
            }
        }
    }
}

/// Audio Event Handler trait for callbacks
pub trait AudioEventHandler: Send + Sync + 'static {
    fn on_progress(&self, current_secs: f64, total_secs: f64);
    fn on_state_changed(&self, is_playing: bool);
    fn on_track_ended(&self);
    fn on_gapless_prefetch_needed(&self);
}

/// MPV Audio Engine - Low-level MPV wrapper
/// Build the mpv command that sets the equaliser chain.
///
/// Returns `None` when there is nothing to set, which is how the equaliser is
/// turned off.
pub fn build_equalizer_command(gains: &[f64]) -> Option<String> {
    if gains.is_empty() {
        return None;
    }

    let mut filters = Vec::with_capacity(gains.len());
    for (index, gain) in gains.iter().enumerate() {
        let frequency = EQUALIZER_BANDS.get(index).copied().unwrap_or(1000.0);
        // mpv is tolerant of a wide range, but a gain outside it would be
        // amplified past what the filters can express.
        let gain = gain.clamp(-24.0, 24.0);
        filters.push(format!("equalizer=f={}:t=q:w=1:g={:.1}", frequency, gain));
    }

    Some(format!("af add @melyar_eq:lavfi=[{}]\n", filters.join(",")))
}

pub struct MelyarAudioEngine {
    mpv: Arc<MpvHandleWrapper>,
    is_playing: Arc<AtomicBool>,
    prefetched_next: Arc<AtomicBool>,
}

impl MelyarAudioEngine {
    pub fn new<H: AudioEventHandler>(handler: Arc<H>) -> Result<Self> {
        let handle = unsafe { mpv_sys::mpv_create() };
        if handle.is_null() {
            return Err(anyhow!("Failed to create MPV handle"));
        }

        // Configure MPV for zero video and high performance audio streaming
        unsafe {
            let set_opt = |name: &str, val: &str| {
                let c_name = CString::new(name).unwrap();
                let c_val = CString::new(val).unwrap();
                mpv_sys::mpv_set_option_string(handle, c_name.as_ptr(), c_val.as_ptr());
            };

            // Audio-only configuration
            set_opt("vo", "null");
            set_opt("audio-buffer", "4");
            set_opt("gapless-audio", "yes");
            set_opt("replaygain", "track");
            set_opt("volume", "100");
            set_opt("idle", "yes");
            set_opt("keep-open", "always");

            let res = mpv_sys::mpv_initialize(handle);
            if res < 0 {
                mpv_sys::mpv_terminate_destroy(handle);
                return Err(anyhow!("Failed to initialize MPV, code {}", res));
            }

            // Observe properties
            let time_pos = CString::new("time-pos").unwrap();
            let duration = CString::new("duration").unwrap();
            let pause = CString::new("pause").unwrap();
            let eof_reached = CString::new("eof-reached").unwrap();

            mpv_sys::mpv_observe_property(
                handle,
                1,
                time_pos.as_ptr(),
                mpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
            );
            mpv_sys::mpv_observe_property(
                handle,
                2,
                duration.as_ptr(),
                mpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
            );
            mpv_sys::mpv_observe_property(
                handle,
                3,
                pause.as_ptr(),
                mpv_sys::mpv_format_MPV_FORMAT_FLAG,
            );
            mpv_sys::mpv_observe_property(
                handle,
                4,
                eof_reached.as_ptr(),
                mpv_sys::mpv_format_MPV_FORMAT_FLAG,
            );
        }

        let mpv_wrapper = Arc::new(MpvHandleWrapper { raw: handle });
        let is_playing = Arc::new(AtomicBool::new(false));
        let prefetched_next = Arc::new(AtomicBool::new(false));

        // Start event processing thread
        let mpv_clone = mpv_wrapper.clone();
        let is_playing_clone = is_playing.clone();
        let prefetched_clone = prefetched_next.clone();

        thread::Builder::new()
            .name("melyar-mpv-events".into())
            .spawn(move || {
                let mut current_pos = 0.0f64;
                let mut current_dur = 0.0f64;

                loop {
                    let event_ptr = unsafe { mpv_sys::mpv_wait_event(mpv_clone.raw, 0.05) };
                    if event_ptr.is_null() {
                        continue;
                    }
                    let event = unsafe { &*event_ptr };
                    if event.event_id == mpv_sys::mpv_event_id_MPV_EVENT_SHUTDOWN {
                        break;
                    }

                    if event.event_id == mpv_sys::mpv_event_id_MPV_EVENT_PROPERTY_CHANGE {
                        if !event.data.is_null() {
                            let prop =
                                unsafe { &*(event.data as *const mpv_sys::mpv_event_property) };
                            if !prop.name.is_null() {
                                let name = unsafe { CStr::from_ptr(prop.name) }.to_string_lossy();
                                match name.as_ref() {
                                    "time-pos" => {
                                        if prop.format == mpv_sys::mpv_format_MPV_FORMAT_DOUBLE
                                            && !prop.data.is_null()
                                        {
                                            current_pos = unsafe { *(prop.data as *const f64) };
                                            handler.on_progress(current_pos, current_dur);

                                            // Gapless prefetch trigger: 5 seconds before end.
                                            // The flag is raised by queue_next_url() only once
                                            // mpv has actually accepted the appended file, so a
                                            // failed resolve cannot make the end of the track
                                            // look like the next one is already loaded.
                                            if current_dur > 10.0
                                                && (current_dur - current_pos) <= 5.0
                                                && !prefetched_clone.load(Ordering::Relaxed)
                                            {
                                                handler.on_gapless_prefetch_needed();
                                            }
                                        }
                                    }
                                    "duration" => {
                                        if prop.format == mpv_sys::mpv_format_MPV_FORMAT_DOUBLE
                                            && !prop.data.is_null()
                                        {
                                            current_dur = unsafe { *(prop.data as *const f64) };
                                            handler.on_progress(current_pos, current_dur);
                                        }
                                    }
                                    "pause"
                                        if prop.format == mpv_sys::mpv_format_MPV_FORMAT_FLAG
                                            && !prop.data.is_null() =>
                                    {
                                        let paused = unsafe { *(prop.data as *const i32) } != 0;
                                        is_playing_clone.store(!paused, Ordering::Relaxed);
                                        handler.on_state_changed(!paused);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    } else if event.event_id == mpv_sys::mpv_event_id_MPV_EVENT_END_FILE {
                        // mpv reports END_FILE for every reason a file stops, not only for a
                        // file that ran out. `loadfile … replace` and `stop` both arrive as
                        // REASON_STOP, so advancing on all of them made Stop start the next
                        // track and made clicking a row mid-playback walk the queue forward
                        // one hop per click until it ran dry.
                        let reason = if event.data.is_null() {
                            mpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_EOF as i32
                        } else {
                            unsafe { &*(event.data as *const mpv_sys::mpv_event_end_file) }.reason
                        };
                        match reason {
                            r if r
                                == mpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_EOF as i32 =>
                            {
                                handler.on_track_ended()
                            }
                            // The file was replaced or playback stopped on purpose.
                            r if r
                                == mpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_STOP as i32 => {
                            }
                            // It could not be played. Do not silently walk past it, but do
                            // stop claiming to be playing a track that is not audible.
                            r if r
                                == mpv_sys::mpv_end_file_reason_MPV_END_FILE_REASON_ERROR
                                    as i32 =>
                            {
                                is_playing_clone.store(false, Ordering::Relaxed);
                                handler.on_state_changed(false);
                            }
                            _ => {}
                        }
                    }
                }
            })
            .expect("Failed to spawn mpv event thread");

        Ok(Self {
            mpv: mpv_wrapper,
            is_playing,
            prefetched_next,
        })
    }

    pub fn play_url(&self, url: &str) -> Result<()> {
        self.prefetched_next.store(false, Ordering::Relaxed);
        let cmd = format!("loadfile \"{}\" replace\n", url);
        self.command_string(&cmd)?;
        self.is_playing.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn queue_next_url(&self, url: &str) -> Result<()> {
        let cmd = format!("loadfile \"{}\" append-play\n", url);
        self.command_string(&cmd)?;
        // mpv moves to this file by itself once the current one runs out, so the
        // end of the track must not resolve and load it a second time.
        self.prefetched_next.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Whether the next track was already appended to mpv's playlist. The claim
    /// is spent, so a flag raised for one track boundary cannot cover the next.
    pub fn take_prefetched(&self) -> bool {
        self.prefetched_next.swap(false, Ordering::Relaxed)
    }

    pub fn toggle_play(&self) -> Result<bool> {
        let cmd = "cycle pause\n";
        self.command_string(cmd)?;
        let playing = !self.is_playing.load(Ordering::Relaxed);
        self.is_playing.store(playing, Ordering::Relaxed);
        Ok(playing)
    }

    pub fn seek(&self, seconds: f64) -> Result<()> {
        let cmd = format!("seek {} absolute\n", seconds);
        self.command_string(&cmd)
    }

    pub fn set_volume(&self, volume: f64) -> Result<()> {
        let clamped = volume.clamp(0.0, 100.0);
        let cmd = format!("set volume {}\n", clamped);
        self.command_string(&cmd)
    }

    /// Set the gains of a ten band graphic equaliser, in decibels.
    ///
    /// The bands are built as a single lavfi chain so that the whole
    /// equaliser is replaced in one go, rather than one command per band.
    pub fn set_equalizer_bands(&self, gains: &[f64]) -> Result<()> {
        // The chain may not be there yet, so a failure here is expected.
        let _ = self.command_string("af remove @melyar_eq\n");

        let Some(cmd) = build_equalizer_command(gains) else {
            return Ok(());
        };
        self.command_string(&cmd)
    }

    pub fn stop(&self) -> Result<()> {
        self.command_string("stop\n")?;
        self.is_playing.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    /// Correct the flag when playback ended by itself, with no command to send.
    ///
    /// mpv runs the last file out, goes idle, and never touches `pause` again -
    /// so nothing observes the change and this flag stays `true` while nothing
    /// is audible. `toggle_play` computes its answer from it, which makes the
    /// first press after an empty queue report the opposite of what it did.
    pub fn mark_stopped(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
    }

    fn command_string(&self, cmd: &str) -> Result<()> {
        let c_cmd = CString::new(cmd)?;
        let res = unsafe { mpv_sys::mpv_command_string(self.mpv.raw, c_cmd.as_ptr()) };
        if res < 0 {
            Err(anyhow!("MPV command failed: {} (code {})", cmd.trim(), res))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equalizer_command_is_empty_for_no_bands() {
        assert!(build_equalizer_command(&[]).is_none());
    }

    #[test]
    fn equalizer_command_uses_the_standard_bands() {
        let gains = [3.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let cmd = build_equalizer_command(&gains).unwrap();
        assert!(cmd.starts_with("af add @melyar_eq:lavfi=["));
        assert!(cmd.contains("equalizer=f=31:t=q:w=1:g=3.0"));
        assert!(cmd.contains("equalizer=f=62:t=q:w=1:g=-2.0"));
        assert!(cmd.contains("equalizer=f=16000:t=q:w=1:g=0.0"));
        assert!(cmd.ends_with("\n"));
    }

    #[test]
    fn equalizer_command_clamps_the_gain() {
        let cmd = build_equalizer_command(&[999.0]).unwrap();
        assert!(cmd.contains("g=24.0"));

        let cmd = build_equalizer_command(&[-999.0]).unwrap();
        assert!(cmd.contains("g=-24.0"));
    }

    #[test]
    fn equalizer_command_survives_a_short_band_list() {
        // Fewer bands than the preset still works, the rest stay flat.
        let cmd = build_equalizer_command(&[1.0, 1.0]).unwrap();
        assert!(cmd.contains("f=31"));
        assert!(cmd.contains("f=62"));
        assert_eq!(cmd.matches("equalizer=").count(), 2);
    }

    /// The engine talks to a real mpv, so this checks that the command the
    /// core builds is one that mpv actually accepts, and that a broken one
    /// is reported rather than swallowed.
    #[test]
    fn mpv_accepts_the_equaliser_chain_and_rejects_a_broken_one() {
        struct Silent;
        impl AudioEventHandler for Silent {
            fn on_progress(&self, _current_secs: f64, _total_secs: f64) {}
            fn on_state_changed(&self, _is_playing: bool) {}
            fn on_track_ended(&self) {}
            fn on_gapless_prefetch_needed(&self) {}
        }

        let Ok(engine) = MelyarAudioEngine::new(Arc::new(Silent)) else {
            // No audio device in this environment, nothing to check.
            return;
        };

        let gains = [6.0, 3.0, 0.0, -2.0, -4.0, 0.0, 3.0, 5.0, 2.0, 0.0];
        assert!(
            engine.set_equalizer_bands(&gains).is_ok(),
            "mpv refused the equaliser chain"
        );

        // Turning it off removes the chain, which must not fail even though
        // the chain may not be there.
        assert!(engine.set_equalizer_bands(&[]).is_ok());

        // A filter that does not exist has to be reported.
        assert!(
            engine
                .command_string("af add @broken:not_a_real_filter\n")
                .is_err(),
            "mpv accepted a filter that does not exist"
        );
    }
}
