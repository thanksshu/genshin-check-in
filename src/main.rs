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
use thiserror::Error;
use tracing::{error, info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

const URL_STRING: &str = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:106.0) Gecko/20100101 Firefox/106.0";
const RETRY_INTERVAL: [u64; 5] = [500, 1000, 2000, 4000, 8000]; // ms

static URL: OnceCell<Url> = OnceCell::new();
static LTUID: OnceCell<String> = OnceCell::new();
static LTOKEN: OnceCell<String> = OnceCell::new();

#[derive(Deserialize, Debug)]
struct CheckInResponse {
    message: String,
    retcode: i32,
}

#[derive(Error, Debug)]
enum CheckInError {
    #[error("LTOKEN or LTUID incorrect")]
    NotLogIn,
    #[error("temporarily unable to check in")]
    RetryLater,
    #[error(transparent)]
    RequestFailure(#[from] reqwest::Error),
    #[error("unknow check in failure with {:?}", .0)]
    Unknown(CheckInResponse),
}

fn check_in() -> Result<String, CheckInError> {
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
    let result = client
        .post(URL_STRING)
        .send()?
        // .map_err(|err| CheckInError::SendFailure(err))?
        .json::<CheckInResponse>()
        .unwrap();

    match result.retcode {
        0 | -5003 => Ok(format!(
            "succeed with {} {}",
            result.retcode, result.message
        )),
        -100 => Err(CheckInError::NotLogIn),
        50000 => Err(CheckInError::RetryLater),
        _ => Err(CheckInError::Unknown(result)),
    }
}

fn reply_404(stream: &mut TcpStream) {
    stream
        .write_all("HTTP/1.1 404 Not Found\r\nX-Fc-Status: 404\r\n\r\n".as_bytes())
        .unwrap()
}

fn handle_invoke(stream: &mut TcpStream) {
    for (try_time, wait_time) in RETRY_INTERVAL
        .into_iter()
        .map(Duration::from_millis)
        .enumerate()
    {
        match check_in() {
            Ok(success_message) => {
                stream
                    .write_all(
                        format!("HTTP/1.1 200 OK\r\nX-Fc-Status: 200\r\n\r\n{success_message}")
                            .as_bytes(),
                    )
                    .unwrap();
                info!("check in succeed or already checked in");
                break;
            }
            Err(err @ CheckInError::NotLogIn) => {
                reply_404(stream);
                error!("{}", err);
                break;
            }
            Err(err) => {
                warn!(
                    "check in failed with {:?}, retrying in {:#?}",
                    err, wait_time
                );
                sleep(wait_time);
                if try_time == 5 {
                    reply_404(stream);
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
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("test info");
    error!("test error");
    warn!("test warn");

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
