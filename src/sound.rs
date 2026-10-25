use std::{
    fs::File,
    io::BufReader,
};

use anyhow::Context;

const SOUND_PATH: &str = "sound.ogg";

/// Starts playing the sound on a background thread.
/// Join the returned handle before exiting so the sound isn't cut off.
pub fn play() -> jod_thread::JoinHandle<()> {
    jod_thread::spawn(|| {
        if let Err(error) = try_play() {
            eprintln!("failed to play {SOUND_PATH}: {error:#}");
        }
    })
}

fn try_play() -> anyhow::Result<()> {
    let file = File::open(SOUND_PATH)
        .with_context(|| format!("could not open sound file `{SOUND_PATH}`"))?;

    let mut sink_handle = rodio::DeviceSinkBuilder::open_default_sink()
        .context("could not open default audio output")?;
    sink_handle.log_on_drop(false);

    let player = rodio::play(sink_handle.mixer(), BufReader::new(file))
        .context("could not decode sound file")?;
    player.sleep_until_end();
    Ok(())
}
