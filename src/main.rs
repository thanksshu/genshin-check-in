use log::{error, info, warn};
use reqwest::{cookie::Jar, ClientBuilder, Url};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use tokio;
use warp::{http::Response, Filter};

const URL: &'static str = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481";
const UA: &'static str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/100.0";

async fn check_in() -> Result<String, Box<dyn std::error::Error>> {
    #[derive(Serialize, Deserialize, Debug)]
    struct Response {
        message: String,
        retcode: i16,
    }

    let url = URL.parse::<Url>().unwrap_or_else(|err| {
        error!("Check in url parsing error: {:?}", err);
        panic!("Exiting...");
    });

    /* bake cookies */
    let ltuid = env::var("LTUID").unwrap_or_else(|err| {
        error!("LTUID setting error: {:?}", err);
        "0".to_string()
    });
    let ltoken = env::var("LTOKEN").unwrap_or_else(|err| {
        error!("LTOKEN setting error: {:?}", err);
        "0".to_string()
    });

    let cookies = [
        format!("ltoken={}; Domain=.mihoyo.com;", ltoken),
        format!("ltuid={}; Domain=.mihoyo.com;", ltuid),
    ];

    /* add cookies to jar */
    let jar = Arc::new(Jar::default());
    cookies
        .iter()
        .for_each(|cookie| jar.add_cookie_str(cookie, &url));

    /* build client */
    let client = ClientBuilder::new()
        .user_agent(UA)
        .cookie_provider(jar.clone())
        .build()
        .unwrap_or_else(|err| {
            error!("Client build error: {:?}", err);
            panic!("Exiting...")
        });

    /* post request */
    let result = client
        .post(url.clone())
        .send()
        .await?
        .json::<Response>()
        .await
        .unwrap_or_else(|err| {
            error!("Json paring error: {:?}", err);
            panic!("Exiting...");
        });

    /* verify response */
    match result {
        Response {
            message,
            retcode: i,
        } if i == 0 => Ok(message),
        Response { message, retcode } => Err(format!("{} {}", retcode, message).into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    /*  POST /invoke */
    let app = warp::path!("invoke").and(warp::post()).then(|| async {
        match check_in().await {
            Ok(m) => {
                let message = format!("{}", m);
                info!("{}", message);
                Response::builder()
                    .header("x-fc-status", "200")
                    .status(200)
                    .body(m)
                    .unwrap_or_else(|err| {
                        error!("Ok response building error: {:?}", err);
                        panic!("Exiting...");
                    })
            }

            Err(m) => {
                let message = format!("{:?}", m);
                warn!("{}", message);
                Response::builder()
                    .header("x-fc-status", "404")
                    .status(404)
                    .body(message)
                    .unwrap_or_else(|err| {
                        error!("Err response building error: {:?}", err);
                        panic!("Exiting...");
                    })
            }
        }
    });

    warp::serve(app).run(([0, 0, 0, 0], 9000)).await;

    Ok(())
}
