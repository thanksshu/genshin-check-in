use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use reqwest::{blocking::Client, cookie::Jar, Url};
use serde::Deserialize;
use std::{
    env::var,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use tracing::{error, info, warn};

const URL_STRING: &str = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:106.0) Gecko/20100101 Firefox/106.0";
const RETRY_TIME: u64 = 5;
const RETRY_INTERVAL: u64 = 1000; // ms

static URL: OnceCell<Url> = OnceCell::new();
static LTUID: OnceCell<String> = OnceCell::new();
static LTOKEN: OnceCell<String> = OnceCell::new();

#[derive(Deserialize, Debug)]
struct CheckInResponse {
    message: String,
    retcode: i32,
}

fn check_in() -> Result<CheckInResponse, reqwest::Error> {
    /* bake cookies */
    let cookies = [
        format!("ltoken={}; Domain=.mihoyo.com;", LTOKEN.get().unwrap()),
        format!("ltuid={}; Domain=.mihoyo.com;", LTUID.get().unwrap()),
    ];

    /* add cookies to jar */
    let jar = Arc::new(Jar::default());
    cookies
        .iter()
        .for_each(|cookie| jar.add_cookie_str(cookie, URL.get().unwrap()));

    /* build client */
    let client = Client::builder()
        .user_agent(UA)
        .cookie_provider(jar)
        .build()
        .unwrap();

    /* post request */
    Ok(client
        .post(URL_STRING)
        .send()?
        .json::<CheckInResponse>()
        .unwrap())
}

fn handle_invoke(stream: &mut TcpStream) {
    let mut rely_404 = |error_message: &String| {
        stream
            .write_all(
                format!(
                    "HTTP/1.1 404 Not Found\r\nX-Fc-Status: 404\r\n\r\n{}",
                    error_message
                )
                .as_bytes(),
            )
            .unwrap();
    };

    for try_time in 1..=RETRY_TIME {
        let wait_time = Duration::from_millis(try_time * RETRY_INTERVAL);
        match check_in() {
            Ok(CheckInResponse { message, retcode }) => {
                if retcode == 0 || retcode == -5003 {
                    stream
                        .write_all(
                            format!(
                                "HTTP/1.1 200 OK\r\nX-Fc-Status: 200\r\n\r\n{retcode} {message}"
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                    info!("check in succeed or already checked in");
                    break;
                } else {
                    warn!("check in failed, retrying in {:#?}", wait_time);
                    sleep(wait_time);
                    if try_time == 5 {
                        rely_404(&format!("{retcode} {message}"));
                        error!("all check in retry failed");
                    }
                    continue;
                }
            }
            Err(error) => {
                warn!("check in failed, retrying in {:#?}", wait_time);
                sleep(wait_time);
                if try_time == 5 {
                    rely_404(&format!("{error}"));
                    error!("all check in retry failed");
                }
                continue;
            }
        }
    }
    stream.flush().unwrap();
}

fn main() {
    /* init statics */
    LTUID
        .set(
            var("LTUID")
                .context("environment variable \"LTUID\" not present")
                .unwrap(),
        )
        .unwrap();
    LTOKEN
        .set(
            var("LTOKEN")
                .context("environment variable \"LTOKEN\" not present")
                .unwrap(),
        )
        .unwrap();
    URL.set(URL_STRING.parse::<Url>().unwrap()).unwrap();

    /* init logger */
    tracing_subscriber::fmt().init();

    /* start server */
    let listener = TcpListener::bind("0.0.0.0:9000").unwrap();

    /* accept connections and process them serially */
    for income in listener.incoming() {
        let mut stream = income.unwrap();
        let mut buf = [0_u8; 2048];
        let n = stream.read(&mut buf[..]).unwrap();
        let receive = String::from_utf8_lossy(&buf[..n]);
        if receive.starts_with("POST /invoke") {
            handle_invoke(&mut stream);
        }
    }
}
