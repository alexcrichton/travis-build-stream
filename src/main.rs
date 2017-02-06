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

use std::env;
use std::collections::HashMap;
use std::time::Duration;
use std::vec;

use chrono::{DateTime, UTC};
use futures::{Future, Poll, Async, Stream};
use getopts::Options;
use tokio_core::reactor::{Core, Handle, Timeout};
use tokio_curl::Session;

use errors::*;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {}", stringify!($e), e),
    })
}

type MyFuture<T> = Box<Future<Item=T, Error=MyError>>;

mod http;
mod errors;
mod travis;

fn main() {
    let token = env::args().skip(1).next().unwrap();
    let mut core = Core::new().unwrap();
    let builds = Builds {
        session: Session::new(core.handle()),
        pending: Vec::new().into_iter(),
        last_number: None,
        fetching: None,
        token: token.clone(),
    };
    let session = builds.session.clone();

    let builds = builds.filter_map(|(build, commit)| {
        if commit.is_none() {
            println!("warning: no commit for build {}", build.number);
        }
        commit.map(|c| (build, c))
    });

    let builds = builds.filter(|&(_, ref commit)| {
        commit.branch == "auto"
    });

    let builds = builds.filter(|&(ref build, _)| {
        build.state == "passed"
    });

    let builds = builds.map(move |(build, _commit)| {
        let url = format!("/builds/{}", build.id);
        http::travis_get::<travis::GetBuild>(&session, &url, &token)
    }).buffered(10);

    println!("{:40} {:5} {:10} {:10} = {:10}",
             "sha",
             "num",
             "build-id",
             "job-id",
             "dur (s)");
    let client = builds.for_each(|travis::GetBuild { build, commit, jobs }| {
        let job = jobs.iter().find(|job| {
            let env = job.config["env"].as_str().unwrap();
            env.contains("android")
        }).unwrap();
        let started = job.started_at.parse::<DateTime<UTC>>().unwrap();
        let finished = job.finished_at.parse::<DateTime<UTC>>().unwrap();
        let duration = (finished - started).num_seconds();

        println!("{} {:5} {:10} {:10} = {:10}",
                 commit.sha,
                 build.number,
                 build.id,
                 job.id,
                 duration);
        Ok(())
    });

    core.run(client).unwrap();
}

struct Builds {
    session: Session,
    pending: vec::IntoIter<(travis::Build, Option<travis::Commit>)>,
    last_number: Option<String>,
    fetching: Option<MyFuture<travis::GetBuilds>>,
    token: String,
}

impl Builds {
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

impl Stream for Builds {
    type Item = (travis::Build, Option<travis::Commit>);
    type Error = MyError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, MyError> {
        loop {
            if let Some(item) = self.pending.next() {
                return Ok(Some(item).into())
            }

            match self.fetching.poll()? {
                Async::Ready(None) => {}
                Async::Ready(Some(builds)) => self.update_with(builds),
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
