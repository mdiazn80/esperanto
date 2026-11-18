//! Lists input devices and the microphone permission status.
//!
//! cargo run -p esperanto-audio --example devices

fn main() {
    println!("permission: {:?}", esperanto_audio::microphone_permission());
    for device in esperanto_audio::list_input_devices().expect("cannot list devices") {
        let default = if device.is_default { " (default)" } else { "" };
        println!("{}{default}\n    id: {}", device.name, device.id);
    }
}
