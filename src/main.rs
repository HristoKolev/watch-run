#![forbid(unsafe_code)]

#[macro_use]
mod global;

use std::sync::mpsc::channel;
use std::time::Duration;
use std::sync::{Mutex, Arc};
use std::thread::JoinHandle;
use std::path::{PathBuf};

use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};

use crate::global::prelude::*;

static WATCH_DELAY: u64 = 1000;

fn main() {
    global::initialize();
    main_result().crash_on_error();
}

fn main_result() -> Result {

    let args = ::std::env::args_os()
        .skip(1)
        .map_result(|x| x.get_as_string())?
        .collect_vec();

    if args.len() < 2 {
        log!("Error: not enough parameters.");
        ::std::process::exit(1);
    }

    let watch_path = args[0].clone();
    let command = args.into_iter().skip(1).collect_vec().join(" ");

    log!("Watching `{}` ...", command);

    let (sender, receiver) = channel();

    let mut watcher = watcher(sender, Duration::from_millis(WATCH_DELAY))?;
    watcher.watch(&watch_path, RecursiveMode::Recursive)?;

    let flag = Arc::new(Mutex::new(false));

    let watch_flag = flag.clone();

    let watch_thread: JoinHandle<Result> = ::std::thread::spawn(move || {

        loop {
            match receiver.recv() {
                Ok(event) => {

                    if let Some(path) = event.get_path() {

                        log!("Change: {}", path.get_as_string()?);
                    }

                    let mut value = watch_flag.lock()?;

                    *value = true;
                },
                Err(error) => elog!("{:#?}", error),
            }
        }
    });

    let run_flag = flag.clone();

    let run_thread: JoinHandle<Result> = ::std::thread::spawn(move || {

        loop {
            ::std::thread::sleep(Duration::from_millis(WATCH_DELAY));

            let mut value = run_flag.lock()?;

            if *value {

                run_command(&command)?;

                *value = false;
            }
        }
    });

    watch_thread.join().replace_error(||
        CustomError::from_message("The receiver thread failed for some reason."))??;

    run_thread.join().replace_error(||
        CustomError::from_message("The run thread failed for some reason."))??;

    Ok(())
}

fn run_command(command: &str) -> Result {

    let result = crate::global::bash_shell::exec(&command);

    match result {
        Ok(_) => (),
        Err(err) => elog!("{:#?}", err)
    }

    Ok(())
}

trait DebounceEventExtensions {
    fn get_path(&self) -> Option<PathBuf>;
}

impl DebounceEventExtensions for DebouncedEvent {

    fn get_path(&self) -> Option<PathBuf> {
        match self {
            DebouncedEvent::NoticeWrite(x) => Some(x.clone()),
            DebouncedEvent::NoticeRemove(x) => Some(x.clone()),
            DebouncedEvent::Create(x) => Some(x.clone()),
            DebouncedEvent::Write(x) => Some(x.clone()),
            DebouncedEvent::Chmod(x) => Some(x.clone()),
            DebouncedEvent::Remove(x) => Some(x.clone()),
            DebouncedEvent::Rename(_, x) => Some(x.clone()),
            DebouncedEvent::Rescan => None,
            DebouncedEvent::Error(_, x) => x.clone(),
        }
    }
}
