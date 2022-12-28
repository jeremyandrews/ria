use std::cmp;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::Result;
use glib::{FlagsClass, MainContext, ObjectExt};
use gstreamer::{event::Seek, ClockTime, Element, SeekFlags, SeekType};
use gstreamer_pbutils::prelude::{ElementExt, ElementExtManual};
use serde::{Deserialize, Serialize};

pub trait PlayerTrait {
    fn add_and_play(&mut self, current_track: &str);
    fn volume(&self) -> i32;
    fn volume_up(&mut self);
    fn volume_down(&mut self);
    fn set_volume(&mut self, volume: i32);
    fn pause(&mut self);
    fn resume(&mut self);
    fn is_paused(&self) -> bool;
    fn seek(&mut self, secs: i64) -> Result<()>;
    fn seek_to(&mut self, last_pos: Duration);
    fn set_speed(&mut self, speed: i32);
    fn speed_up(&mut self);
    fn speed_down(&mut self);
    fn speed(&self) -> i32;
    fn stop(&mut self);
}

#[derive(Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub volume: i32,
    pub speed: i32,
    pub gapless: bool,
}

#[derive(Clone)]
pub enum PlayerMsg {
    Eos,
    AboutToFinish,
    CurrentTrackUpdated,
    Progress(i64, i64),
}

pub trait PathToURI {
    fn to_uri(&self) -> String;
}

impl PathToURI for Path {
    /// Returns `self` as a URI. Panics in case of an error.
    fn to_uri(&self) -> String {
        glib::filename_to_uri(self, None)
            .expect("Error converting path to URI")
            .to_string()
    }
}

#[derive(Clone)]
pub struct GStreamer {
    playbin: Element,
    paused: bool,
    volume: i32,
    speed: i32,
    pub gapless: bool,
    pub message_tx: Sender<PlayerMsg>,
}

impl GStreamer {
    pub fn new(config: &Settings, message_tx: Sender<PlayerMsg>) -> Self {
        gstreamer::init().expect("Couldn't initialize Gstreamer");

        let ctx = glib::MainContext::default();
        let _guard = ctx.acquire();
        let mainloop = glib::MainLoop::new(Some(&ctx), false);

        // playbin3 provides a stand-alone everything-in-one abstraction for an audio and/or video player.
        // https://gstreamer.freedesktop.org/documentation/playback/playbin3.html
        let playbin = gstreamer::ElementFactory::make("playbin3")
            .build()
            .expect("playbin3 make error");

        // autoaudiosink is an audio sink that automatically detects an appropriate audio sink to use.
        // https://gstreamer.freedesktop.org/documentation/autodetect/autoaudiosink.html
        let sink = gstreamer::ElementFactory::make("autoaudiosink")
            .build()
            .expect("audio sink make error");

        playbin.set_property("audio-sink", &sink);

        // Set flags to show Audio and Video but ignore Subtitles
        let flags = playbin.property_value("flags");
        let flags_class = FlagsClass::new(flags.type_()).unwrap();

        let flags = flags_class
            .builder_with_value(flags)
            .unwrap()
            .set_by_nick("audio")
            .unset_by_nick("video")
            .unset_by_nick("text")
            .build()
            .unwrap();
        playbin.set_property_from_value("flags", &flags);

        // Asynchronous channel to communicate with main() with
        let (main_tx, main_rx) = MainContext::channel(glib::Priority::default());
        // Handle messages from GSTreamer bus
        playbin
        .bus()
        .expect("Failed to get GStreamer message bus")
        .add_watch(glib::clone!(@strong main_tx => move |_bus, msg| {
            match msg.view() {
                gstreamer::MessageView::Eos(_) =>
                    main_tx.send(PlayerMsg::Eos)
                    .expect("Unable to send message to main()"),
                gstreamer::MessageView::StreamStart(_) =>
                    main_tx.send(PlayerMsg::CurrentTrackUpdated).expect("Unable to send current track message"),
                gstreamer::MessageView::Error(e) =>
                    glib::g_debug!("song", "{}", e.error()),
                _ => (),
            }
                glib::Continue(true)
                // gstreamer::prelude::Continue(true)
        }))
        .expect("Failed to connect to GStreamer message bus");

        let tx = message_tx.clone();
        std::thread::spawn(move || {
            main_rx.attach(
                None,
                glib::clone!(@strong mainloop => move |msg| {
                    tx.send(msg).ok();
                    glib::Continue(true)
                }),
            );
            mainloop.run();
        });

        let volume = config.volume;
        let speed = config.speed;
        let gapless = config.gapless;

        let mut this = Self {
            playbin,
            paused: false,
            volume,
            speed,
            gapless,
            message_tx,
        };

        this.set_volume(volume);
        this.set_speed(speed);

        // Switch to next song when reaching end of current track
        let tx = main_tx;
        // this.playbin.connect(
        //     "about-to-finish",
        //     false,
        //     glib::clone!(@strong this => move |_args| {
        //        tx.send(PlayerMsg::AboutToFinish).unwrap();
        //        None
        //     }),
        // );

        this.playbin.connect("about-to-finish", false, move |_| {
            tx.send(PlayerMsg::AboutToFinish).ok();
            None
        });

        glib::source::timeout_add(
            std::time::Duration::from_millis(1000),
            glib::clone!(@strong this => move || {
                this.get_progress().ok();
            glib::Continue(true)
            }),
        );

        this
    }
    pub fn skip_one(&mut self) {
        self.message_tx.send(PlayerMsg::Eos).unwrap();
    }
    pub fn enqueue_next(&mut self, next_track: &str) {
        self.playbin
            .set_state(gstreamer::State::Ready)
            .expect("set gst state ready error.");

        let path = Path::new(next_track);
        self.playbin.set_property("uri", path.to_uri());

        self.playbin
            .set_state(gstreamer::State::Playing)
            .expect("set gst state playing error");
    }
    fn set_volume_inside(&mut self, volume: f64) {
        self.playbin.set_property("volume", volume);
    }

    fn get_progress(&self) -> Result<()> {
        let time_pos = self.get_position().seconds() as i64;
        let duration = self.get_duration().seconds() as i64;
        self.message_tx
            .send(PlayerMsg::Progress(time_pos, duration))?;
        Ok(())
    }

    fn get_position(&self) -> ClockTime {
        match self.playbin.query_position::<ClockTime>() {
            Some(pos) => pos,
            None => ClockTime::from_seconds(0),
        }
    }

    fn get_duration(&self) -> ClockTime {
        match self.playbin.query_duration::<ClockTime>() {
            Some(pos) => pos,
            None => ClockTime::from_seconds(0),
        }
    }

    fn send_seek_event(&mut self, rate: i32) -> bool {
        self.speed = rate;
        let rate = rate as f64 / 10.0;
        // Obtain the current position, needed for the seek event
        let position = self.get_position();

        // Create the seek event
        let seek_event = if rate > 0. {
            Seek::new(
                rate,
                SeekFlags::FLUSH | SeekFlags::ACCURATE,
                SeekType::Set,
                position,
                SeekType::None,
                position,
            )
        } else {
            Seek::new(
                rate,
                SeekFlags::FLUSH | SeekFlags::ACCURATE,
                SeekType::Set,
                position,
                SeekType::Set,
                position,
            )
        };

        // If we have not done so, obtain the sink through which we will send the seek events
        if let Some(sink) = self.playbin.property::<Option<Element>>("audio-sink") {
            // try_property::<Option<Element>>("audio-sink") {
            // Send the event
            sink.send_event(seek_event)
        } else {
            false
        }
    }
}

impl PlayerTrait for GStreamer {
    fn add_and_play(&mut self, song_str: &str) {
        self.playbin
            .set_state(gstreamer::State::Ready)
            .expect("set gst state ready error.");
        let path = Path::new(song_str);
        self.playbin.set_property("uri", path.to_uri());
        self.playbin
            .set_state(gstreamer::State::Playing)
            .expect("set gst state playing error");
    }

    fn volume_up(&mut self) {
        self.volume = cmp::min(self.volume + 5, 100);
        self.set_volume_inside(f64::from(self.volume) / 100.0);
    }

    fn volume_down(&mut self) {
        self.volume = cmp::max(self.volume - 5, 0);
        self.set_volume_inside(f64::from(self.volume) / 100.0);
    }

    fn volume(&self) -> i32 {
        self.volume
    }

    fn set_volume(&mut self, mut volume: i32) {
        volume = volume.clamp(0, 100);
        self.volume = volume;
        self.set_volume_inside(f64::from(volume) / 100.0);
    }

    fn pause(&mut self) {
        self.paused = true;
        // self.player.pause();
        self.playbin
            .set_state(gstreamer::State::Paused)
            .expect("set gst state paused error");
    }

    fn resume(&mut self) {
        self.paused = false;
        // self.player.play();
        self.playbin
            .set_state(gstreamer::State::Playing)
            .expect("set gst state playing error in resume");
    }

    fn is_paused(&self) -> bool {
        self.playbin.current_state() == gstreamer::State::Paused
    }

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_wrap)]
    fn seek(&mut self, secs: i64) -> Result<()> {
        let time_pos = self.get_position().seconds() as i64;
        let duration = self.get_duration().seconds() as i64;
        let mut seek_pos = time_pos + secs;
        if seek_pos < 0 {
            seek_pos = 0;
        }
        if seek_pos > duration - 6 {
            seek_pos = duration - 6;
        }

        let seek_pos_clock = ClockTime::from_seconds(seek_pos as u64);
        self.set_volume_inside(0.0);
        self.playbin
            .seek_simple(gstreamer::SeekFlags::FLUSH, seek_pos_clock)?; // ignore any errors
        self.set_volume_inside(f64::from(self.volume) / 100.0);
        self.message_tx
            .send(PlayerMsg::Progress(seek_pos, duration))?;
        Ok(())
    }

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_wrap)]
    fn seek_to(&mut self, last_pos: Duration) {
        let seek_pos = last_pos.as_secs() as i64;
        let duration = self.get_duration().seconds() as i64;

        let seek_pos_clock = ClockTime::from_seconds(seek_pos as u64);
        self.set_volume_inside(0.0);
        while self
            .playbin
            .seek_simple(gstreamer::SeekFlags::FLUSH, seek_pos_clock)
            .is_err()
        {
            std::thread::sleep(Duration::from_secs(100));
        }
        self.set_volume_inside(f64::from(self.volume) / 100.0);
        self.message_tx
            .send(PlayerMsg::Progress(seek_pos, duration))
            .ok();
    }
    fn speed(&self) -> i32 {
        self.speed
    }

    fn set_speed(&mut self, speed: i32) {
        self.send_seek_event(speed);
    }

    fn speed_up(&mut self) {
        let mut speed = self.speed + 1;
        if speed > 30 {
            speed = 30;
        }
        if !self.send_seek_event(speed) {
            eprintln!("error set speed");
        }
    }

    fn speed_down(&mut self) {
        let mut speed = self.speed - 1;
        if speed < 1 {
            speed = 1;
        }
        self.set_speed(speed);
    }
    fn stop(&mut self) {
        self.playbin.set_state(gstreamer::State::Null).ok();
    }
}

impl Drop for GStreamer {
    /// Cleans up `GStreamer` pipeline when `Backend` is dropped.
    fn drop(&mut self) {
        self.playbin
            .set_state(gstreamer::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");
    }
}
