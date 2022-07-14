use log::{info, error};
use reqwest::{cookie::Jar, ClientBuilder, Url};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use warp::{http::Response, Filter};

const URL: &str = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481";
const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/100.0";

#[derive(Serialize, Deserialize, Debug)]
struct CheckInResponse {
    message: String,
    retcode: i32,
}

async fn check_in() -> Result<String, Box<dyn std::error::Error>> {
    let url = URL.parse::<Url>()?; // parse check in url

    /* bake cookies */
    let ltuid = env::var("LTUID")?;
    let ltoken = env::var("LTOKEN")?;

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
        .build()?;

    /* post request */
    let result = client
        .post(url.clone())
        .send()
        .await?
        .json::<CheckInResponse>()
        .await?;

    /* verify response */
    match result {
        CheckInResponse {
            message,
            retcode: i,
        } if i == 0 || i == -5003 => Ok(message),
        CheckInResponse { message, retcode } => Err(format!("{} {}", retcode, message).into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /* init logger */
    simple_logger::SimpleLogger::new().env().init().unwrap();

    /*  app for POST /invoke */
    let app = warp::path!("invoke").and(warp::post()).then(|| async {
        match check_in().await {
            Ok(m) => {
                let message = m.to_string();
                info!("{}", message);
                Response::builder()
                    .header("x-fc-status", "200")
                    .status(200)
                    .body(m)
            }

            Err(m) => {
                let message = format!("{}", m);
                error!("{}", message);
                Response::builder()
                    .header("x-fc-status", "404")
                    .status(404)
                    .body(message)
            }
        }
    });

    warp::serve(app).run(([0, 0, 0, 0], 9000)).await;
    Ok(())
}
