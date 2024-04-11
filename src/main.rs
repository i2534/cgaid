use encoding_rs_io::DecodeReaderBytesBuilder;
use notify::{Config as NC, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use simplelog::{Config as SLC, SimpleLogger};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, Seek};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::{cmp, env};

mod chat;
mod config;
mod notifier;
use chat::record::{Channel, Record};
use config::Config as CC;

pub trait Notifiable {
    fn notify(&self, message: &str) -> Result<bool, Box<dyn Error>>;
}

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(log::LevelFilter::Info, SLC::default())?;

    let work_dir = env::current_dir()?;
    log::info!("Work dir: {work_dir:?}");

    let cfg = CC::load(work_dir.join("config.toml"))?;
    log::debug!("Config: {cfg:?}");

    let game_dir: &String = &cfg.game.path;
    log::info!("Game root: {game_dir}");

    let game_path = Path::new(game_dir);
    let log_dir = game_path.join("Log");
    if !log_dir.exists() {
        log::info!("Log dir not exists: {log_dir:?}");
        fs::create_dir_all(&log_dir)?;
    }

    let (tx, rx) = channel();
    let mut watcher =
        RecommendedWatcher::new(tx, NC::default().with_poll_interval(Duration::from_secs(1)))?;

    watcher.watch(&log_dir, RecursiveMode::NonRecursive)?;

    let chat_file_re = Regex::new(r"^chat_\d{6}\.txt$")?;
    let chat_file_filter =
        |p: &PathBuf| chat_file_re.is_match(p.file_name().and_then(|v| v.to_str()).unwrap_or(""));
    let mut chat_file = find_file(&log_dir, chat_file_filter)?;

    let mut offset = 0_u64;
    if let Some(f) = &chat_file {
        log::info!("Chat file found: {f}");
        let (_, p) = read(Path::new(f), 0)?;
        offset = p;
    }

    let empty = PathBuf::new();
    let ac = Arc::new(cfg);
    for r in rx {
        match r {
            Ok(event) => {
                // println!("{:?} {:?}", event, &chat_file);
                match event.kind {
                    EventKind::Modify(_) => {
                        let path = event.paths.iter().next().unwrap_or(&empty);
                        if let Some(f) = &chat_file {
                            if path != Path::new(f) {
                                chat_file = find_file(&log_dir, chat_file_filter)?;
                                log::info!("Chat file changed: {chat_file:?}");
                            }
                        } else {
                            chat_file = find_file(&log_dir, chat_file_filter)?;
                            log::info!("Chat file changed: {chat_file:?}");
                        }

                        if let Some(f) = &chat_file {
                            let (lines, p) = read(Path::new(f), offset)?;
                            log::debug!("{} -> {}", offset, p);
                            offset = p;
                            try_notify(Arc::clone(&ac), lines);
                        } else {
                            log::info!("Chat file not found");
                        }
                    }
                    _ => {
                        // log::info!("Other event: {other:?}");
                    }
                }
            }
            Err(error) => log::error!("Error: {error:?}"),
        }
    }

    Ok(())
}

fn find_file<P, F>(root: P, filter: F) -> io::Result<Option<String>>
where
    P: AsRef<Path>,
    F: Fn(&PathBuf) -> bool,
{
    let mut entries: Vec<PathBuf> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .filter(filter)
        .collect::<Vec<_>>();
    entries.sort_by_key(|f| cmp::Reverse(f.file_name().unwrap().to_owned()));
    if entries.is_empty() {
        Ok(None)
    } else {
        Ok(entries[0].to_str().map(|v| v.to_owned()))
    }
}

fn read(path: &Path, offset: u64) -> io::Result<(Vec<String>, u64)> {
    // log::info!("Reading file: {path:?}");
    let mut f = File::open(path)?;
    f.seek(io::SeekFrom::Start(offset))?;
    let reader = io::BufReader::new(
        DecodeReaderBytesBuilder::new()
            .encoding(Some(encoding_rs::GB18030))
            .build(&f),
    );

    let mut lines = Vec::new();
    for line in reader.lines().map(|v| v.unwrap()) {
        lines.push(line);
    }
    let p = f.stream_position()?;
    // println!("Read: {offset} -> {p}");
    Ok((lines, p))
}

fn try_notify(cfg: Arc<CC>, lines: Vec<String>) {
    let records: BTreeSet<_> = lines.iter().filter_map(|v| Record::from(v)).collect();
    // log::info!("{:?}", records.len());
    let triggers = cfg.as_ref().clone().trigger;
    for record in records {
        // println!("{:?}", r);
        if record.is_channel(Channel::Common) {
            let msg = record.msg();
            for trigger in &triggers {
                let nc = trigger.clone();
                if let Some(matched) = nc.try_match(msg) {
                    let message = nc.format(&matched).replace("{time}", &record.fmt_time());
                    log::info!("Matched: {message}");
                    for name in nc.notifier {
                        let cc = Arc::clone(&cfg);
                        let mc = message.clone();
                        thread::spawn(move || {
                            match config::Notifier::find(cc.as_ref(), &name)
                                .and_then(|o| o.notify(&mc))
                            {
                                Ok(b) => {
                                    log::info!("{name} notified: {b}");
                                }
                                Err(e) => {
                                    log::error!("Notify error: {e}");
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}
