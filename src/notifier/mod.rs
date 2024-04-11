use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, Sink};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};

pub mod webhook;

pub struct Simple {}

impl Simple {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Notifiable for Simple {
    fn notify(&self, message: &str) -> Result<bool, Box<dyn Error>> {
        log::info!("Simple notify: {}", message);
        Ok(true)
    }
}

struct Player {
    sink: Sink,
    _stream: OutputStream,
}

pub struct Ringtone {
    path: String,
    player: Option<Player>,
}

impl Player {
    fn stop(&self) {
        self.sink.stop();
    }
    fn play<R>(&self, source: Decoder<R>)
    where
        R: Read + Seek + Send + Sync + 'static,
    {
        self.sink.append(source);
    }
    fn wait_end(&self) {
        self.sink.sleep_until_end();
    }
}

impl Ringtone {
    pub fn new(path: String, device_name: String) -> Result<Self, Box<dyn Error>> {
        let device = Ringtone::find_device(&device_name)?;
        let mut player = None;
        if let Some(d) = device {
            let (_stream, handle) = OutputStream::try_from_device(&d)?;
            let sink = Sink::try_new(&handle)?;
            player = Some(Player { sink, _stream });
        }
        Ok(Self { path, player })
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        if let Some(player) = &self.player {
            player.stop();
        }
    }

    fn find_device(name: &String) -> Result<Option<cpal::Device>, Box<dyn Error>> {
        let host = cpal::default_host();
        if name.is_empty() {
            return Ok(host.default_output_device());
        }
        let devices = host.output_devices()?;
        let mut device = None;
        for d in devices {
            let name = d.name()?;
            // println!("Device: {}", name);
            if name.contains(name.as_str()) {
                device = Some(d);
            }
        }
        Ok(device)
    }
}

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

impl super::Notifiable for Ringtone {
    fn notify(&self, message: &str) -> Result<bool, Box<dyn Error>> {
        log::info!("Ringtone notify: {}", message);
        if let Some(player) = &self.player {
            player.stop();
            let path = &self.path;
            if path.is_empty() {
                let data = include_bytes!("demo.mp3");
                player.play(Decoder::new(Cursor::new(data.as_ref()))?);
            } else {
                player.play(Decoder::new(BufReader::new(File::open(path)?))?);
            };
            player.wait_end();
            Ok(true)
        } else {
            Err("Device not found".into())
        }
    }
}

pub struct Invoke {
    path: String,
    args: Vec<String>,
    workdir: String,
}

impl Invoke {
    pub fn new(path: String, args: Vec<String>, workdir: String) -> Self {
        Self {
            path,
            args,
            workdir,
        }
    }
}

impl super::Notifiable for Invoke {
    fn notify(&self, message: &str) -> Result<bool, Box<dyn Error>> {
        let mut command = std::process::Command::new(&self.path);
        let dir = if self.workdir.is_empty() {
            std::env::current_dir()?
        } else {
            std::path::PathBuf::from(&self.workdir)
        };
        command.current_dir(dir);
        for arg in &self.args {
            command.arg(arg.replace("{message}", message));
        }
        let output = command.output()?;
        log::info!("Invoke result: {output:?}");
        Ok(output.status.success())
    }
}

#[cfg(test)]
mod tests {

    use std::thread::sleep;

    use super::super::Notifiable;
    use super::*;

    #[test]
    fn test_music_mp3() {
        let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&handle).unwrap();

        let file = std::fs::File::open("assets/demo.mp3").unwrap();
        sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap());

        // sink.sleep_until_end();
        std::thread::sleep(std::time::Duration::from_millis(5000));
        sink.stop();
        // drop(_stream);
        let file = std::fs::File::open("assets/demo.mp3").unwrap();
        sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap());
        // sink.play();
        std::thread::sleep(std::time::Duration::from_millis(5000));
    }

    #[test]
    fn test_default_ringtone() {
        let ringtone = Ringtone::new(String::new(), String::new()).unwrap();
        let ret = ringtone.notify("Hello, World!").unwrap();
        assert!(ret);
        sleep(std::time::Duration::from_secs(10));
    }

    #[test]
    fn test_ringtone() {
        let ringtone = Ringtone::new("assets/y1717.mp3".to_owned(), String::new()).unwrap();
        let ret = ringtone.notify("Hello, World!").unwrap();
        assert!(ret);
        sleep(std::time::Duration::from_secs(10));
    }

    #[test]
    fn test_invoke() {
        let invoke = Invoke::new(
            "cmd".to_owned(),
            vec!["/C".to_owned(), "echo {message}".to_owned()],
            String::new(),
        );
        let ret = invoke.notify("Hello, World!").unwrap();
        assert!(ret);
    }

    #[test]
    fn test_invoke_shutdown() {
        let invoke = Invoke::new(
            "cmd".to_owned(),
            vec!["/C".to_owned(), "shutdown /s /f /t 60".to_owned()],
            String::new(),
        );
        let ret = invoke.notify("Hello, World!").unwrap();
        assert!(ret);
    }
}
