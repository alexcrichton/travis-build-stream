extern crate futures;
extern crate tokio_core;
extern crate tokio_curl;
extern crate chrono;
extern crate explorer;

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

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
    let builds = TravisBuilds::new(session.clone(),
                                   "rust-lang/rust",
                                   token.clone());

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

    println!("{:40} {:5} {:10}",
             "sha",
             "num",
             "build-id");

    let data = Path::new("data");
    drop(fs::remove_dir_all(&data));
    fs::create_dir_all(&data).unwrap();
    let mut map = HashMap::new();
    {
        let client = builds.take(50).for_each(|build| {
            let travis::GetBuild { build, commit, jobs } = build;
            println!("{} {:5} {:10}", commit.sha, build.number, build.id);
            for job in jobs.iter() {
                let started = job.started_at.parse::<DateTime<UTC>>().unwrap();
                let finished = job.finished_at.parse::<DateTime<UTC>>().unwrap();
                let duration = (finished - started).num_seconds();

                let env = job.config["env"].as_str().unwrap();

                let filename = sanitize(env);
                let entry = map.entry(sanitize(env).trim().to_string())
                               .or_insert((filename.clone(), 0));
                entry.1 += 1;

                let mut file = fs::OpenOptions::new()
                                    .read(true)
                                    .write(true)
                                    .create(true)
                                    .open(&data.join(filename)).unwrap();
                file.seek(SeekFrom::End(0)).unwrap();
                writeln!(file, "{} {}", build.number, duration).unwrap();
            }
            Ok(())
        });

        core.run(client).unwrap();
    }

    let mut f = File::create(data.join("script.gnuplot")).unwrap();

    write!(f, r#"
set term png
set output "foo.png"
set terminal png size 2000,2000
set xlabel "build number"
set ylabel "build time (s)"
plot \
"#).unwrap();
    for (k, v) in map {
        if v.1 > 10 {
            writeln!(f, "\"{}\" title \'{}\' with lines smooth bezier, \\", v.0, k).unwrap();
        }
    }
}

fn sanitize(s: &str) -> String {
    s.replace("RUST_CHECK_TARGET", "")
     .replace("RUSTC_RETRY_LINKER_ON_SEGFAULT=1", "")
     .replace("DEPLOY=1", "")
     .replace("SCCACHE_ERROR_LOG=/tmp/sccache.log", "")
     .replace("RUST_LOG=sccache=debug", "")
     .replace("RUST_CONFIGURE_ARGS", "")
     .replace(" ", "")
     .trim()
     .chars().map(|c| {
        match c {
            'a' ... 'z' |
            'A' ... 'Z' |
            '0' ... '9' => c,
            _ => '-',
        }
    }).collect()
}
