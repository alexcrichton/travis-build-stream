use std::str;
use std::io;

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
