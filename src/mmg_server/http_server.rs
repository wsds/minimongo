use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result, middleware};
use serde::Serialize;
use actix_cors::Cors;
use actix_web::http::header;
use common::helper::get_timestamp;
use crate::minimongo::mmg;

#[derive(Serialize)]
struct HelloMassage {
    message: String,
    timestamp: u128,
}
#[get("/")]
async fn hello_mmg() -> Result<impl Responder> {
    let timestamp = get_timestamp();
    let hello_message = HelloMassage {
        message: "This is a MiniMongo Server!".to_string(),
        timestamp,
    };
    Ok(web::Json(hello_message))
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
pub async fn start_mmg_server() -> std::io::Result<()> {
    let factory = || {
        let cdp_scope = web::scope("/mmg").service(mmg::greet)
            .service(mmg::query_raw)
            .service(mmg::query)
            .service(mmg::update_collection)
            .service(mmg::create_collection);
        App::new()
            .wrap(Cors::permissive())
            .service(cdp_scope)
            .service(hello_mmg).service(echo)
            .route("/hey", web::get().to(manual_hello))
    };
    HttpServer::new(factory).bind(("127.0.0.1", 16655))?.run().await
}


#[cfg(test)]
mod tests {
    use super::*;

    //cargo test start_http_server -- --show-output
    #[test]
    fn test_start_http_server() -> Result<(), Box<i32>> {
        println!("准备测试: test_start_mmg_server");

        Ok(())
    }

    //cargo test test_start_mmg_server -- --nocapture
    #[test]
    fn test_start_mmg_server() -> Result<(), Box<i32>> {
        println!("准备测试: test_start_mmg_server");
        let _ = start_mmg_server();
        println!("测试完成: test_start_mmg_server");

        Ok(())
    }
}