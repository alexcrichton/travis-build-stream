use std::str;
use std::io;

use futures::{Stream, Future};
use tokio_curl;
use curl;
use serde_json;

error_chain! {
    types {
        MyError, MyErrorKind, MyChainErr, MyResult;
    }

    foreign_links {
        curl::Error, Curl;
        tokio_curl::PerformError, TokioCurl;
        serde_json::Error, Json;
        str::Utf8Error, NotUtf8;
        io::Error, Io;
    }
}

pub type MyFuture<T> = Box<Future<Item=T, Error=MyError>>;

pub fn b<'a, F>(f: F) -> Box<Future<Item = F::Item, Error = MyError> + 'a>
    where F: Future<Error = MyError> + 'a,
{
    Box::new(f)
}

pub fn bs<'a, F>(f: F) -> Box<Stream<Item = F::Item, Error = MyError> + 'a>
    where F: Stream<Error = MyError> + 'a,
{
    Box::new(f)
}
