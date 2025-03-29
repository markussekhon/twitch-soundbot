use once_cell::sync::Lazy;
use rodio::{Decoder, OutputStream, Sink};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;

/// Reads the list of available sound names from the "sounds" directory.
fn read_sound_list() -> Vec<String> {
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir("sounds") {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(fname) = entry.file_name().to_str() {
                        if let Some(name) = fname.split('.').next() {
                            list.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    list
}

/// Cache the sound list once.
static SOUND_LIST: Lazy<Mutex<Vec<String>>> =
    Lazy::new(|| Mutex::new(read_sound_list()));

/// Plays a sound for a redemption event if the reward title matches one of the
/// available sound files. The match is done case-insensitively. The decoded
/// sound is appended to a new sink, and the thread will block until that sound
/// finishes playing.
pub fn play_sound_for_redemption(display_name: &str, reward_title: &str) {
    println!("{} redeemed {}", display_name, reward_title);

    let sound_list = SOUND_LIST.lock().unwrap();
    let lower_reward = reward_title.to_lowercase();
    if let Some(matched_name) = sound_list
        .iter()
        .find(|name| name.to_lowercase() == lower_reward)
    {
        let file_path = format!("sounds/{}.mp3", matched_name);
        // Attempt to play the sound file using rodio.
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(file) = File::open(&file_path) {
                if let Ok(source) = Decoder::new(BufReader::new(file)) {
                    let sink = Sink::try_new(&stream_handle)
                        .expect("Failed to create sink");
                    sink.append(source);
                    // Block until the sound finishes playing.
                    sink.sleep_until_end();
                } else {
                    println!("Failed to decode sound file: {}", file_path);
                }
            } else {
                println!("Sound file not found: {}", file_path);
            }
        } else {
            println!("No audio output device available.");
        }
    } else {
        println!("No matching sound for reward: {}", reward_title);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::IndexedRandom;
    use std::thread;
    use std::time::Duration;

    /// This test assumes that a folder named `sounds` exists at the project
    /// root and contains at least one valid MP3 file. It locks the sound list
    /// before using methods like `is_empty()` and `choose()`. Then it spawns
    /// 10 threads that each call `play_sound_for_redemption` with a short
    /// delay between spawns to force overlapping playback.
    #[test]
    fn test_overlapping_playback() {
        {
            let sound_list = SOUND_LIST.lock().unwrap();
            assert!(
                !sound_list.is_empty(),
                "Sound list is empty. Please ensure that the sounds/
                 folder contains at least one .mp3 file."
            );
        }

        let mut handles = Vec::new();
        for i in 0..10 {
            let display_name = format!("TestUser{}", i);
            // Choose a random sound from the sound list.
            let chosen_sound = {
                let sound_list = SOUND_LIST.lock().unwrap();
                let mut rng = rand::rng();
                sound_list.choose(&mut rng).unwrap().clone()
            };
            // For testing, we assume the reward title exactly equals the name.
            let reward_title = chosen_sound.clone();

            let handle = thread::spawn(move || {
                play_sound_for_redemption(&display_name, &reward_title);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
        // Allow extra time for the sounds to play.
        thread::sleep(Duration::from_secs(3));
    }
}
