use cookie_store::CookieStore;
use log::{error, info};
use serde::Deserialize;
use std::{
    env::var,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};
use ureq::{builder, Cookie};
use url::Url;

const URL: &str = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/100.0";

fn check_in() -> Result<String, Box<dyn std::error::Error>> {
    #[derive(Deserialize, Debug)]
    struct CheckInResponse {
        message: String,
        retcode: i32,
    }

    let url = URL.parse::<Url>()?; // parse check in url

    /* bake cookies */
    let mut jar = CookieStore::default();

    let cookies = [
        Cookie::new("ltoken", var("LTOKEN")?),
        Cookie::new("ltuid", var("LTUID")?),
    ];

    cookies.iter().for_each(|cookie| {
        jar.insert_raw(cookie, &url).unwrap();
    });

    /* build client */
    let client = builder().user_agent(UA).cookie_store(jar).build();

    /* post request */
    let result = client.post(URL).call()?.into_json()?;

    /* verify response */
    match result {
        CheckInResponse {
            message,
            retcode: i,
        } if i == 0 || i == -5003 => Ok(message),
        CheckInResponse { message, retcode } => Err(format!("{} {}", retcode, message).into()),
    }
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buf = [0_u8; 2048];
    let n = stream.read(&mut buf[..]).unwrap();
    let receive = String::from_utf8_lossy(&buf[..n]);
    if receive.starts_with("POST /invoke") {
        match check_in() {
            Ok(message) => {
                info!("{}", message);
                stream.write_all(
                    format!("HTTP/1.1 200 OK\r\nX-Fc-Status: 200\r\n\r\n{}", message).as_bytes(),
                )?;
            }
            Err(message) => {
                error!("{}", message);
                stream.write_all(
                    format!(
                        "HTTP/1.1 404 Not Found\r\nX-Fc-Status: 404\r\n\r\n{}",
                        message
                    )
                    .as_bytes(),
                )?;
            }
        }
        stream.flush()?;
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    /* init logger */
    simple_logger::SimpleLogger::new().env().init().unwrap();

    /* start server */
    let listener = TcpListener::bind("0.0.0.0:9000").unwrap();

    /* accept connections and process them serially */
    for stream in listener.incoming() {
        handle_client(stream?)?;
    }
    Ok(())
}
