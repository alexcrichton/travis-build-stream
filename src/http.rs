use std::sync::{Arc, Mutex};
use std::str;

use futures::Future;
use tokio_curl::Session;
use curl::easy::{Easy, List};
use serde_json;
use serde::Deserialize;

use errors::*;

#[allow(dead_code)]
pub struct Response {
    easy: Easy,
    headers: Arc<Mutex<Vec<Vec<u8>>>>,
    body: Arc<Mutex<Vec<u8>>>,
}

pub fn travis_get<T>(sess: &Session,
                     url: &str,
                     token: &str) -> MyFuture<T>
    where T: Deserialize + 'static
{
    let url = format!("https://api.travis-ci.org{}", url);
    let headers = vec![
        format!("Authorization: token {}", token),
        format!("Accept: application/vnd.travis-ci.2+json"),
    ];
    get_json(sess, &url, None, None, &headers)
}

pub fn travis_post(sess: &Session,
                   url: &str,
                   token: &str) -> MyFuture<()> {
    let headers = vec![
        format!("Authorization: token {}", token),
        format!("Accept: application/vnd.travis-ci.2+json"),
    ];

    let response = post(sess,
                        &format!("https://api.travis-ci.org{}", url),
                        &headers);
    Box::new(response.map(|_| ()))
}

pub fn appveyor_get<T>(sess: &Session,
                       url: &str,
                       token: &str) -> MyFuture<T>
    where T: Deserialize + 'static,
{
    let headers = vec![
        format!("Authorization: Bearer {}", token),
        format!("Accept: application/json"),
    ];

    get_json(sess,
             &format!("https://ci.appveyor.com/api{}", url),
             None,
             None,
             &headers)
}

pub fn github_get<T>(sess: &Session,
                     url: &str,
                     user: &str,
                     password: &str) -> MyFuture<T>
    where T: Deserialize + 'static,
{
    let headers = vec![
        format!("Accept: application/vnd.github.v3+json"),
    ];

    get_json(sess,
             &format!("https://api.github.com{}", url),
             Some(user),
             Some(password),
             &headers)
}

pub fn get_json<T>(sess: &Session,
                   url: &str,
                   user: Option<&str>,
                   pass: Option<&str>,
                   headers: &[String]) -> MyFuture<T>
    where T: Deserialize + 'static
{
    let response = get(sess, url, user, pass, headers);
    let ret = response.and_then(|response| {
        let body = response.body.lock().unwrap();
        let json = try!(str::from_utf8(&body));
        let ret = try!(serde_json::from_str(json).chain_err(|| {
            format!("failed to decode: {:#?}", json)
        }));
        Ok(ret)
    });
    Box::new(ret)
}

pub fn get(sess: &Session,
           url: &str,
           user: Option<&str>,
           pass: Option<&str>,
           headers: &[String]) -> MyFuture<Response> {
    let mut handle = Easy::new();
    let mut list = List::new();
    t!(list.append("User-Agent: hello!"));
    for header in headers {
        t!(list.append(header));
    }

    if let Some(user) = user {
        t!(handle.username(user));
    }
    if let Some(pass) = pass {
        t!(handle.password(pass));
    }

    t!(handle.http_headers(list));
    t!(handle.get(true));
    t!(handle.url(url));

    perform(sess, handle, url)
}

pub fn delete(sess: &Session, url: &str, headers: &[String]) -> MyFuture<Response> {
    let mut handle = Easy::new();
    let mut list = List::new();
    t!(list.append("User-Agent: hello!"));
    for header in headers {
        t!(list.append(header));
    }

    t!(handle.http_headers(list));
    t!(handle.custom_request("DELETE"));
    t!(handle.url(url));

    perform(sess, handle, url)
}

pub fn post(sess: &Session, url: &str, headers: &[String]) -> MyFuture<Response> {
    let mut handle = Easy::new();
    let mut list = List::new();
    t!(list.append("User-Agent: hello!"));
    for header in headers {
        t!(list.append(header));
    }

    t!(handle.http_headers(list));
    t!(handle.post(true));
    t!(handle.url(url));

    perform(sess, handle, url)
}

pub fn perform(sess: &Session, mut easy: Easy, url: &str) -> MyFuture<Response> {
    debug!("fetching: {}", url);
    let headers = Arc::new(Mutex::new(Vec::new()));
    let data = Arc::new(Mutex::new(Vec::new()));

    let (data2, headers2) = (data.clone(), headers.clone());
    t!(easy.header_function(move |data| {
        headers2.lock().unwrap().push(data.to_owned());
        true
    }));
    t!(easy.write_function(move |buf| {
        data2.lock().unwrap().extend_from_slice(&buf);
        Ok(buf.len())
    }));

    let response = sess.perform(easy);
    let url = url.to_string();
    let checked_response = response.map_err(|e| e.into()).and_then(move |mut easy| {
        debug!("finished: {}", url);
        match t!(easy.response_code()) {
            200 | 204 => {
                Ok(Response {
                    easy: easy,
                    headers: headers,
                    body: data,
                })
            }
            code => {
                Err(format!("not a 200 code: {}\n\n{}\n",
                            code,
                            String::from_utf8_lossy(&data.lock().unwrap())).into())
            }
        }
    });

    Box::new(checked_response)
}
