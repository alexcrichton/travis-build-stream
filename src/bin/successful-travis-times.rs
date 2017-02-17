extern crate futures;
extern crate tokio_core;
extern crate tokio_curl;
extern crate chrono;
#[macro_use]
extern crate explorer;

use std::env;

use chrono::{DateTime, UTC};
use futures::Stream;
use tokio_core::reactor::Core;
use tokio_curl::Session;

use explorer::TravisBuilds;
use explorer::http;
use explorer::travis;

fn main() {
    let token = env::args().skip(1).next().unwrap();
    let mut core = Core::new().unwrap();
    let session = Session::new(core.handle());
    let builds = TravisBuilds::new(session.clone(), token.clone());

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
