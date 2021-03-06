extern crate futures;
extern crate tokio_core;
extern crate tokio_curl;
extern crate chrono;
#[macro_use]
extern crate explorer;
extern crate time;

use std::env;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, UTC};
use futures::{Future, Stream};
use futures::future;
use futures::stream;
use tokio_core::reactor::{Core, Interval};
use tokio_curl::Session;

use explorer::TravisBuilds;
use explorer::http;
use explorer::travis;
use explorer::errors::*;

struct Data {
    queued: Vec<(travis::Build, travis::Job)>,
    running: Vec<(travis::Build, travis::Job)>,
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut interval = Interval::new(Duration::new(5 * 60, 0), &handle).unwrap();

    println!("time\trunning\tqueued");
    // for _ in 0..12 * 48 {
    loop {
        let now = time::now();
        let (running, queued) = get();

        println!("{}\t{}\t{}", now.rfc822(), running, queued);

        let (_a, b) = core.run(interval.into_future()).ok().unwrap();
        interval = b;

    }
}

fn get() -> (usize, usize) {
    let token = env::args().skip(1).next().unwrap();
    let mut core = Core::new().unwrap();
    let session = Session::new(core.handle());

    let user = http::travis_get::<travis::GetUser>(&session, "/users", &token);
    let user = t!(core.run(user));

    let ref data = RefCell::new(Data {
        queued: Vec::new(),
        running: Vec::new(),
    });

    let ref all_repos = RefCell::new(HashMap::new());
    let repos = user.user.channels.iter()
                                  .filter(|c| c.starts_with("repo-"))
                                  .map(|c| c[5..].parse::<u32>().unwrap());
    let repos = repos.map(|id| {
        let url = format!("/repos/{}", id);
        let repo = http::travis_get::<travis::GetRepo>(&session, &url, &token);

        let session2 = session.clone();
        let token2 = token.clone();
        let builds = repo.map(move |repo| {
            let repo = repo.repo;
            all_repos.borrow_mut().insert(repo.id, repo.clone());
            if repo.slug.starts_with("rust-lang/") {
                bs(TravisBuilds::new(session2, &repo.slug, token2))
            } else {
                bs(stream::iter(None))
            }
        }).flatten_stream();

        let builds = builds.take(50);
        let builds = bs(builds.map(|pair| pair.0).filter(|build| {
            build.state != "passed" &&
                build.state != "failed" &&
                build.state != "canceled" &&
                build.state != "errored"
        }));
        // let builds = bs(builds.filter_map(move |build| {
        //     if build.state == "created" {
        //         data.borrow_mut().queued.push(build);
        //         None
        //     } else {
        //         Some(build)
        //     }
        // }));
        let session2 = session.clone();
        let token2 = token.clone();
        let builds = bs(builds.map(move |build| {
            let url = format!("/builds/{}", build.id);
            http::travis_get::<travis::GetBuild>(&session2, &url, &token2)
        }).buffer_unordered(20));
        let jobs = builds.map(|travis::GetBuild { build, commit, jobs }| {
            drop(commit);
            stream::iter(jobs.into_iter().map(move |j| Ok((build.clone(), j))))
        }).flatten();
        let jobs = jobs.filter(|pair| {
            pair.1.state != "passed" &&
                pair.1.state != "failed" &&
                pair.1.state != "canceled" &&
                pair.1.state != "errored"
        });
        b(jobs.for_each(move |(build, job)| {
            if job.state == "created" {
                data.borrow_mut().queued.push((build, job));
            } else {
                data.borrow_mut().running.push((build, job));
            }
            Ok(())
        }))
    }).collect::<Vec<_>>();
    t!(core.run(future::join_all(repos)));

    let mut data = data.borrow_mut();
    let all_repos = all_repos.borrow();

    (data.running.len(), data.queued.len())
}
