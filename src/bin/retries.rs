#[macro_use]
extern crate futures;
extern crate tokio_core;
extern crate tokio_curl;
extern crate chrono;
#[macro_use]
extern crate explorer;
extern crate env_logger;
extern crate git2;

use std::env;
use std::collections::HashSet;

use futures::{Stream, Future, Poll, Async};
use tokio_core::reactor::Core;
use tokio_curl::Session;

use explorer::{TravisBuilds, AppVeyorBuilds};
use explorer::http;
use explorer::github;
use explorer::appveyor;
use explorer::travis;
use explorer::errors::*;

fn main() {
    drop(env_logger::init());
    let travis_token = env::args().skip(1).next().unwrap();
    let appveyor_token = env::args().skip(2).next().unwrap();
    let gh_password = env::args().skip(3).next().unwrap();
    let mut core = Core::new().unwrap();
    let session = Session::new(core.handle());
    let travis = TravisBuilds::new(session.clone(), travis_token.clone());
    let appveyor = AppVeyorBuilds::new(session.clone(),
                                       appveyor_token.clone(),
                                       Some("auto".to_string()));
    let travis = travis.filter_map(|(build, commit)| {
        if commit.is_none() {
            println!("warning: no commit for build {}", build.number);
        }
        commit.map(|c| (build, c))
    });
    let travis = travis.filter(|&(_, ref commit)| {
        commit.branch == "auto"
    });

    let commits = MyZip {
        a: appveyor,
        b: travis,
        appveyor: Vec::new(),
        travis: Vec::new(),
    };

    let commits = commits.map(|(appveyor, (travis_build, travis))| {
        let url = format!("/repos/rust-lang/rust/commits/{}", travis.sha);
        let c = http::github_get(&session, &url, "alexcrichton", &gh_password);
        c.then(|commit: Result<github::Commit, _>| {
            Ok((commit.ok(), appveyor, travis_build, travis))
        })
    }).buffered(10);

    let commits = commits.filter_map(|(gh, a, b, c)| {
        gh.map(|x| (x, a, b, c))
    });

    let mut successes = HashSet::new();

    let client = commits.for_each(|(gh, appveyor, travis_build, travis)| {
        assert_eq!(appveyor.commit_id, travis.sha);
        let pr = appveyor.message.split(" - ").next().unwrap().to_string();

        if appveyor.status == "success" && travis_build.state == "passed" {
            if !successes.insert(gh.parents[1].sha.clone()) {
                println!("duplicate success?! {}", pr);
            }
        } else {
            if !successes.contains(&gh.parents[1].sha) {
                return Ok(())
            }

            println!("{} spurious failure of {}", travis.committed_at, pr);
            println!("\t{:8} https://travis-ci.org/rust-lang/rust/builds/{}",
                     travis_build.state, travis_build.id);
            println!("\t{:8} https://ci.appveyor.com/project/rust-lang/rust/build/1.0.{}",
                     appveyor.status, appveyor.build_number);
        }
        Ok(())
    });

    core.run(client).unwrap();
}

struct MyZip<A, B> {
    a: A,
    b: B,
    appveyor: Vec<appveyor::Build>,
    travis: Vec<(travis::Build, travis::Commit)>,
}

impl<A, B> Stream for MyZip<A, B>
    where A: Stream<Item = appveyor::Build, Error = MyError>,
          B: Stream<Item = (travis::Build, travis::Commit), Error = MyError>,
{
    type Item = (appveyor::Build, (travis::Build, travis::Commit));
    type Error = MyError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, MyError> {
        if let Async::Ready(Some(a)) = self.a.poll()? {
            self.appveyor.push(a);
        }
        if let Async::Ready(Some(t)) = self.b.poll()? {
            self.travis.push(t);
        }

        for i in 0..self.appveyor.len() {
            for j in 0..self.travis.len() {
                if self.appveyor[i].commit_id == self.travis[j].1.sha {
                    let a = self.appveyor.remove(i);
                    let b = self.travis.remove(j);
                    return Ok(Some((a, b)).into())
                }
            }
        }

        Ok(Async::NotReady)
    }
}
