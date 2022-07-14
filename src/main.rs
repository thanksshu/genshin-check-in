use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use log::{error, info};
use reqwest::{cookie::Jar, ClientBuilder, Url};
use serde::{Deserialize};
use std::{env, sync::Arc};

const URL: &str = "https://hk4e-api-os.mihoyo.com/event/sol/sign?act_id=e202102251931481";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/100.0";

#[derive(Deserialize, Debug)]
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
        .post(url)
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

/*  app for POST /invoke */
#[post("/invoke")]
async fn invoke() -> impl Responder {
    match check_in().await {
        Ok(m) => {
            let message = m.to_string();
            info!("{}", message);
            HttpResponse::Ok()
                .insert_header(("x-fc-status", "200"))
                .body(m)
        }

        Err(e) => {
            let message = format!("{}", e);
            error!("{}", message);
            HttpResponse::NotFound()
                .insert_header(("x-fc-status", "404"))
                .body(message)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    /* init logger */
    simple_logger::SimpleLogger::new().env().init().unwrap();

    /* start server */
    HttpServer::new(|| App::new().service(invoke))
        .bind(("0.0.0.0", 9000))?
        .run()
        .await
}
