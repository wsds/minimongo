use std::thread;
use actix_web::{get, web, App, HttpServer, Responder, Result};
use serde::Serialize;
use actix_cors::Cors;
use crate::common::helper::get_timestamp;
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
            .service(hello_mmg)
    };
    HttpServer::new(factory).bind(("127.0.0.1", 16655))?.run().await
}

pub fn start_mmg_server_sub_thread() {
    thread::spawn(|| {
        println!("start mini_mongo_server in sub_thread. 1002");
        let _ = start_mmg_server();
        println!("start mini_mongo_server finished. 1002");
    });
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

    //cargo test test_start_mmg_server_sub_thread -- --nocapture
    #[test]
    fn test_start_mmg_server_sub_thread() -> Result<(), Box<i32>> {
        println!("准备测试: test_start_mmg_server_sub_thread");
        let _ = start_mmg_server_sub_thread();
        println!("测试完成: test_start_mmg_server_sub_thread");
        Ok(())
    }
}