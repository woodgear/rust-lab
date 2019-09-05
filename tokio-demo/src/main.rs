use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ConfigBuilder};
use futures::{Async, Poll, Stream, try_ready};
use failure::Error;
use std::{fs::{self, File}, path::*, io::SeekFrom, io::prelude::*, thread};
use std::ffi::OsStr;
use std::collections::VecDeque;
use std::thread::JoinHandle;
use tokio::sync::mpsc::{channel, Receiver};
use failure::_core::time::Duration;
use tokio::prelude::{Sink, Future};
use std::sync::mpsc::SendError;
use log::*;

fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    n.as_micros().to_string()
}

struct Test {
    thread_handle: JoinHandle<()>,
    rx: Receiver<String>,
}


impl Stream for Test {
    type Item = String;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        info!("poll");
        loop {
            match self.rx.poll() {
                Ok(Async::Ready(Some(msg))) => {
                    info!("ready -->{}<--", msg);
                    return Ok(Async::Ready(Some(msg)));
                }
                Err(e) => {
                    info!("err {:?}", e);
                    return Err(failure::err_msg(format!("{:?}", e)));
                }
                Ok(Async::NotReady) => {
                    info!("notready");
                    return Ok(Async::NotReady);
                }
                Ok(Async::Ready(None)) => {
                    info!("none");
                    return Ok(Async::Ready(None));
                }
            }
        }
    }
}


impl Test {
    fn new() -> Self {
        let (tx, rx) = channel(1);
        let thread_handle = thread::spawn(move || {
            info!("current thread start");
            let mut count = 0;
            loop {
                let tx_clone = tx.clone();
                info!("should send {}",count);
                tokio::run(futures::lazy(move|| {
                    tokio::spawn(futures::lazy(move || {
                        let msg = format!("test {} {}",count,now());
                        info!("start send {}",count);
                        tx_clone.clone().send(msg).then(|res| {
                            match res {
                                Ok(_) => {
                                    info!("send over");
                                    Ok((()))
                                }
                                Err(e) => {
                                    Err(())
                                }
                            }
                        })
                    }));
                    Ok(())
                }));
                thread::sleep(Duration::from_secs(3));
                count = count+1;
            }
        });
        return Test { thread_handle, rx };
    }
}


fn main() {
        let log_config = ConfigBuilder::new().set_thread_level(LevelFilter::Info).build();
        TermLogger::init(LevelFilter::Debug, log_config, TerminalMode::Mixed);
        info!("main start");
        let mut t = Test::new();
        let f = t.for_each(|line|{
            info!("rece {}",line);
            thread::sleep(Duration::from_secs(5));
            Ok(())
        }).map_err(|e|());
        tokio::run(futures::lazy(||{
            tokio::spawn(f);
            tokio::spawn(tokio::timer::Interval::new_interval(Duration::from_secs(3)).for_each(|_|{
                info!("interval");
                Ok(())
            }).map_err(|e|()))
        }));
        info!("over");
}