use std::{fs, path::Path};
use notify::{event::CreateKind, poll::ScanEvent, Config, EventKind, PollWatcher, RecursiveMode, Watcher};
use slog::{error, info};

use crate::{run_workflow_until_stop, Options};

pub fn watch(options: &mut Options, path: &str) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    enum Message {
        Event(notify::Result<notify::Event>),
        Scan(ScanEvent),
    }

    let tx_c = tx.clone();
    let mut watcher = PollWatcher::with_initial_scan(
        move |watch_event| {
            tx_c.send(Message::Event(watch_event)).unwrap();
        },
        Config::default(),
        move |scan_event| {
            tx.send(Message::Scan(scan_event)).unwrap();
        },
    )?;

    watcher.watch(Path::new(path), RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Message::Event(e) => {
                info!(options.global_logger(), "Watch event {e:?}");
                if let Ok(event) = e {
                    if event.kind == EventKind::Create(CreateKind::Any) {
                        let latest_path = event.paths.first().unwrap();
                        info!(options.global_logger(), "Running new file {:?}", latest_path);
                        let contents = fs::read_to_string(latest_path).expect("file read error");

                        match run_workflow_until_stop(options, contents) {
                            Ok(_) => info!(options.global_logger(), "successfully ran {:?}", latest_path),
                            Err(e) => error!(options.global_logger(), "failed to run: {:?}\n {}", latest_path, e)
                        }
                    }
                }
            }
            Message::Scan(e) => {
                info!(options.global_logger(), "Scan event {e:?}")
            }
        }
    }

    Ok(())
}
