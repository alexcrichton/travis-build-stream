extern crate curl;
extern crate futures;
extern crate getopts;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate time;
extern crate tokio_core;
extern crate tokio_curl;
#[macro_use]
extern crate error_chain;
extern crate chrono;
#[macro_use]
extern crate log;

#[macro_export]
macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {}", stringify!($e), e),
    })
}

pub mod appveyor;
pub mod errors;
pub mod github;
pub mod http;
pub mod travis;

use std::collections::HashMap;
use std::vec;

use tokio_curl::Session;
use futures::{Stream, Future, Async, Poll};

use errors::*;

pub struct TravisBuilds {
    session: Session,
    pending: vec::IntoIter<(travis::Build, Option<travis::Commit>)>,
    last_number: Option<String>,
    fetching: Option<MyFuture<travis::GetBuilds>>,
    token: String,
}

impl TravisBuilds {
    pub fn new(session: Session, token: String) -> TravisBuilds {
        TravisBuilds {
            session: session,
            pending: Vec::new().into_iter(),
            last_number: None,
            fetching: None,
            token: token,
        }
    }

    fn update_with(&mut self, builds: travis::GetBuilds) {
        let commits = builds.commits.into_iter()
            .map(|c| (c.id, c))
            .collect::<HashMap<_, _>>();

        // we're only interested in builds that concern our branch
        let mut builds = builds.builds.into_iter().map(|build| {
            let commit = commits.get(&build.commit_id).cloned();
            (build, commit)
        }).collect::<Vec<_>>();

        builds.sort_by(|a, b| {
            b.0.number.cmp(&a.0.number)
        });

        self.last_number = Some(builds.last().unwrap().0.number.clone());
        self.pending = builds.into_iter();
    }
}

impl Stream for TravisBuilds {
    type Item = (travis::Build, Option<travis::Commit>);
    type Error = MyError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, MyError> {
        loop {
            if let Some(item) = self.pending.next() {
                return Ok(Some(item).into())
            }

            match self.fetching.poll()? {
                Async::Ready(None) => {}
                Async::Ready(Some(builds)) => {
                    self.fetching = None;
                    self.update_with(builds);
                    continue
                }
                Async::NotReady => return Ok(Async::NotReady),
            }

            let mut url = "/repos/rust-lang/rust/builds".to_string();
            if let Some(ref s) = self.last_number {
                url.push_str("?after_number=");
                url.push_str(s);
            }
            self.fetching = Some(http::travis_get(&self.session,
                                                  &url,
                                                  &self.token));
        }
    }
}

pub struct AppVeyorBuilds {
    session: Session,
    pending: vec::IntoIter<appveyor::Build>,
    next_start: Option<u32>,
    fetching: Option<MyFuture<appveyor::History>>,
    token: String,
    branch: Option<String>,
}

impl AppVeyorBuilds {
    pub fn new(session: Session,
               token: String,
               branch: Option<String>) -> AppVeyorBuilds {
        AppVeyorBuilds {
            session: session,
            pending: Vec::new().into_iter(),
            fetching: None,
            next_start: None,
            token: token,
            branch: branch,
        }
    }
}

impl Stream for AppVeyorBuilds {
    type Item = appveyor::Build;
    type Error = MyError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, MyError> {
        const PER_PAGE: u32 = 100;
        loop {
            if let Some(item) = self.pending.next() {
                return Ok(Some(item).into())
            }

            match self.fetching.poll()? {
                Async::Ready(None) => {}
                Async::Ready(Some(builds)) => {
                    self.fetching = None;
                    let min = builds.builds.iter().map(|b| b.build_id).min();
                    self.next_start = min;
                    self.pending = builds.builds.into_iter();
                    continue
                }
                Async::NotReady => return Ok(Async::NotReady),
            }

            let mut url = "/projects/rust-lang/rust/history?recordsNumber=".to_string();
            url.push_str(&PER_PAGE.to_string());
            if let Some(ref branch) = self.branch {
                url.push_str("&branch=");
                url.push_str(branch);
            }
            if let Some(s) = self.next_start {
                url.push_str("&startBuildId=");
                url.push_str(&s.to_string());
            }
            self.fetching = Some(http::appveyor_get(&self.session,
                                                    &url,
                                                    &self.token));
        }
    }
}
