use std::collections::BTreeMap;
use actix_web::{get, post, web};
use serde_json::Value;
use common::helper::get_timestamp;
use crate::minimongo::minimongo::{Collection, get_mgdb, Schema};
use crate::minimongo::query::UpdateType;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Info {
    mmg: String,
    timestamp: u128,
}

#[get("/hello/{name}")]
pub async fn greet(name: web::Path<String>) -> web::Json<Info> {
    // format!("Hello {}!", name)
    let timestamp = get_timestamp();
    println!("最简请求: greet @{timestamp}");

    web::Json(Info {
        mmg: name.clone(),
        timestamp,
    })
}


#[derive(Deserialize, Serialize, Debug)]
pub struct CreateCollectionRequest {
    workspace_id: String,
    collection_name: String,
    schema: Schema,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateCollectionResponse {
    timestamp: u128,
    state: u64,
    message: String,
    collections: Vec<Collection>,
}

#[post("/create_collection")]
pub async fn create_collection(data: web::Json<CreateCollectionRequest>) -> web::Json<CreateCollectionResponse> {
    // println!("准备创建collection: {data:#?}");
    let timestamp = get_timestamp();

    let mg_db = get_mgdb(data.0.workspace_id);
    mg_db.create_collection(data.0.collection_name, data.0.schema);
    let collections = mg_db.list_all_collections();

    let response = CreateCollectionResponse {
        timestamp,
        state: 200,
        message: "collection创建成功".to_string(),
        collections,
    };
    web::Json(response)
}


#[derive(Deserialize, Serialize, Debug)]
pub struct UpdateCollectionRequest {
    workspace_id: String,
    collection_name: String,
    collections: Vec<Value>,
    update_type: UpdateType,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UpdateCollectionResponse {
    timestamp: u128,
    state: u64,
    message: String,
    num_created: u32,
    num_updated: u32,
}

#[post("/update_collection")]
pub async fn update_collection(data: web::Json<UpdateCollectionRequest>) -> web::Json<UpdateCollectionResponse> {
    // println!("准备 update_collection : {data:#?}");
    let timestamp = get_timestamp();

    let mg_db = get_mgdb(data.0.workspace_id);
    mg_db.update_records(&data.0.collection_name, data.0.collections, data.0.update_type);

    let response = UpdateCollectionResponse {
        timestamp,
        state: 200,
        message: "update collection 成功".to_string(),
        num_created: 55,
        num_updated: 51,
    };
    web::Json(response)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct QueryRequest {
    workspace_id: String,
    query: String,
    params: BTreeMap<String, Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct QueryResponse {
    timestamp: u128,
    state: u64,
    message: String,
    final_result: BTreeMap<String, Value>,
}

#[post("/query")]
pub async fn query(data: web::Json<QueryRequest>) -> web::Json<QueryResponse> {
    // println!("准备 query : {data:#?}");
    let timestamp = get_timestamp();

    let mg_db = get_mgdb(data.0.workspace_id);
    let final_result = mg_db.query_records(&data.0.query, data.0.params);

    let response = QueryResponse {
        timestamp,
        state: 200,
        message: "query 执行成功".to_string(),
        final_result,
    };
    web::Json(response)
}

#[post("/query_raw")]
pub async fn query_raw(data: web::Json<QueryRequest>) -> web::Json<BTreeMap<String, Value>> {
    // println!("准备 query : {data:#?}");
    let _timestamp = get_timestamp();

    let mg_db = get_mgdb(data.0.workspace_id);
    let final_result = mg_db.query_records(&data.0.query, data.0.params);

    web::Json(final_result)
}
