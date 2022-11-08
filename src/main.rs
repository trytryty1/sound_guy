#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate core;

mod graphics;
mod texture;

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use image::{DynamicImage, ImageResult, RgbImage};
use image::io::Reader;


#[derive(Parser, Debug)]
#[command(version, about = "CPAL feedback example", long_about = None)]
struct Opt {
    /// The input audio device to use
    #[arg(short, long, value_name = "IN", default_value_t = String::from("default"))]
    input_device: String,

    /// Specify the delay between input and output
    #[arg(short, long, value_name = "DELAY_MS", default_value_t = 150.0)]
    latency: f32,

    /// Use the JACK host
    #[cfg(all(
    any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
    ),
    feature = "jack"
    ))]
    #[arg(short, long)]
    #[allow(dead_code)]
    jack: bool,
}

// Float that stores the loudest audio input detected over the las few milliseconds
pub static mut AUDIO_IN: f32 = 0.0;
const AUDIO_DIFFUSE: f32 = 0.00005;

fn main() {
    // Setup the audio stream
    let stream = setup_feedback();

    // Setup the window and graphics
    pollster::block_on(graphics::run());

    // Destroy the audio steam
    drop(stream);
}

// Consumes the thread until done with feedback
fn setup_feedback() -> Stream {
    let opt = Opt::parse();

    // Conditionally compile with jack if the feature is specified.
    #[cfg(all(
    any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
    ),
    feature = "jack"
    ))]
        // Manually check for flags. Can be passed through cargo with -- e.g.
        // cargo run --release --example beep --features jack -- --jack
        let host = if opt.jack {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    #[cfg(any(
    not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
    )),
    not(feature = "jack")
    ))]
        let host = cpal::default_host();

    // Find devices.
    let input_device = host.default_input_device()
        .expect("failed to find input device");

    println!("Using input device: \"{}\"", match input_device.name() {
        Ok(t) => t,
        Err(_) => panic!("ERROR")
    });

    // We'll try and use the same configuration between streams to keep it simple.
    let config: cpal::StreamConfig = match input_device.default_input_config() {
        Ok(t) => t.into(),
        Err(_) => panic!("Config is brok")
    };

    // Call back for when the audio input device get audio
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| unsafe {
        for &sample in data {

            // Increases AUDIO_IN if the input is louder and decrease it gradually
            //let var = if sample < 0.1 {0.0} else {}
            AUDIO_IN = f32::max(AUDIO_IN, if sample < 0.03 {0.0} else {f32::sqrt(sample*2.0)}) - AUDIO_IN * AUDIO_DIFFUSE;
        }
    };

    // Build streams.
    println!(
        "Attempting to build both streams with f32 samples and `{:?}`.",
        config
    );
    let input_stream = match input_device.build_input_stream(&config, input_data_fn, err_fn) {
        Ok(t) => t,
        Err(_) => panic!("NOOOOOO!")
    };
    println!("Successfully built streams.");

    // Play the streams.
    println!(
        "Starting the input and output streams with `{}` milliseconds of latency.",
        opt.latency
    );


    input_stream.play().expect("TODO: panic message");

    //thread::sleep(time::Duration::from_millis(10000));

    input_stream
}



fn err_fn(_: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", "Audio input stream");
}