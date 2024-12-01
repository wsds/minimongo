use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug};
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};

use redb::{Database, ReadableTable, ReadTransaction, TableDefinition, MultimapTableDefinition, ReadableMultimapTable};
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use mini_common::helper::hash_to_u32;
use crate::minimongo::query::{UpdateType};
use crate::minimongo::query_helper::{MyF64, open_table_read, open_table_write};

static MGDB_MAP: LazyLock<RwLock<HashMap<String, Arc<MgDb>>>> = LazyLock::new(|| {
    let _map = HashMap::new();
    let _map_rw = RwLock::new(_map);
    _map_rw
});

pub struct MgDb {
    pub db: Database,
    pub _workspace_nanoid: String,
    pub collection_map: RwLock<BTreeMap<String, Collection>>,
    pub counter_map: RwLock<BTreeMap<String, u32>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Schema {
    primary_key: String,
    indexes_f64: Vec<String>,
    indexes_string: Vec<String>,
    indexes_string_unique: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub collection_name: String,
    pub field_list: Vec<String>,
    // schema: Value,
    pub primary_key: String,
    pub indexes_f64_list: Vec<String>,
    pub indexes_string_list: Vec<String>,
    pub indexes_string_unique_list: Vec<String>,
    //index
    //field_map
}

// struct MyF64(f64);
//
// impl Key for MyF64 {
// fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
//     Self::from_bytes(data1).cmp(&Self::from_bytes(data2))
// }
// }
pub fn get_mgdb(workspace_nanoid: String) -> Arc<MgDb> {
    let  need_create;

    {
        let db_map_lock = MGDB_MAP.read().unwrap();
        let db_option = db_map_lock.get(&workspace_nanoid);
        if let Some(db) = db_option {
            return db.clone();
        }
    }

    let mut db_map_lock = MGDB_MAP.write().unwrap();
    let db_option = db_map_lock.get(&workspace_nanoid);
    if let Some(db) = db_option {
        return db.clone();
    } else {
        need_create = true;
    }

    let db = create_db(&workspace_nanoid);
    let mut need_init = false;
    let mut collection_map = BTreeMap::new();
    let mut counter_map = BTreeMap::new();
    {
        let read_txn = db.begin_read().unwrap();
        let collection_define_table_result = read_txn.open_table(COLLECTION_DEFINE_TABLE);
        if let Ok(table) = collection_define_table_result {
            let mut iter = table.iter().unwrap();
            while let Some(kv) = iter.next() {
                if let Ok((key, value)) = kv {
                    // println!("读取collection value: {:?} @ {:?}", key.value(), value.value());
                    let collection_name = key.value();
                    let collections_str = value.value();
                    let collection = serde_json::from_str(collections_str.as_str()).unwrap();
                    collection_map.insert(collection_name, collection);
                }
            }
        } else {
            need_init = true;
        }

        let counter_table_result = read_txn.open_table(COUNTER_TABLE);
        if let Ok(table) = counter_table_result {
            let mut iter = table.iter().unwrap();
            '_label: while let Some(kv) = iter.next() {
                if let Ok((key, value)) = kv {
                    let counter_name = key.value();
                    let number = value.value();
                    counter_map.insert(counter_name, number);
                }
            }
        } else {
            need_init = true;
        }
    }
    if need_init {
        println!("需要初始化: COLLECTION_TABLE");
        let write_txn = db.begin_write().unwrap();
        {
            let _table = write_txn.open_table(COLLECTION_DEFINE_TABLE).unwrap();
            let _table = write_txn.open_table(COUNTER_TABLE).unwrap();
        }
        write_txn.commit().unwrap();
    }

    let mg_db = MgDb {
        db,
        _workspace_nanoid: workspace_nanoid.clone(),
        collection_map: RwLock::new(collection_map),
        counter_map: RwLock::new(counter_map),
    };
    let db_arc = Arc::new(mg_db);

    if need_create {
        db_map_lock.insert(workspace_nanoid, db_arc.clone());
        drop(db_map_lock);
    }
    return db_arc;
}

fn create_db(workspace_nanoid: &String) -> Database {
    fs::create_dir_all("MMG").unwrap();
    let db_filename = format!("MMG/W_{}.db", workspace_nanoid);
    let db_file = Path::new(&db_filename);
    let db = Database::create(db_file).unwrap();
    return db;
}


const COLLECTION_DEFINE_TABLE: TableDefinition<String, String> = TableDefinition::new("collection_define");
const COUNTER_TABLE: TableDefinition<String, u32> = TableDefinition::new("counter");

impl MgDb {
    pub fn create_collection(&self, collection_name: String, schema: Schema) {
        let primary_key = schema.primary_key;
        let indexes_f64_list: Vec<String> = schema.indexes_f64;
        let indexes_string_list: Vec<String> = schema.indexes_string;
        let indexes_string_unique_list: Vec<String> = schema.indexes_string_unique;

        let collection = Collection {
            collection_name: collection_name.clone(),
            field_list: vec![],
            // schema,
            primary_key,
            indexes_f64_list,
            indexes_string_list,
            indexes_string_unique_list,
        };

        let mut count_number: u32 = 10_0000_0000;

        {
            let write_txn = self.db.begin_write().unwrap();
            {
                let mut collection_define_table = write_txn.open_table(COLLECTION_DEFINE_TABLE).unwrap();
                let collections_str = serde_json::to_string(&collection).unwrap_or("{}".to_string());
                collection_define_table.insert(collection_name.clone(), collections_str).unwrap();

                {
                    let mut counter_map_lock = self.counter_map.write().unwrap();
                    let number_option = counter_map_lock.get(&collection_name);
                    if let Some(number) = number_option {
                        count_number = *number;
                    } else {
                        let mut counter_table = write_txn.open_table(COUNTER_TABLE).unwrap();
                        counter_table.insert(collection_name.clone(), count_number).unwrap();
                    }
                    counter_map_lock.insert(collection_name.clone(), count_number);

                    drop(counter_map_lock);
                }

                let collection_table_define: TableDefinition<u32, String> = TableDefinition::new(collection_name.as_str());
                let _collection_table = write_txn.open_table(collection_table_define).unwrap();

                println!("新建_collection_table");

                let collection_name_primary = format!("{}@primary", collection_name);
                let _primary_key_table = open_table_write::<&str, u32>(&collection_name_primary, &write_txn);

                {
                    println!("新建_动态值_table");
                    let collection_name_f64 = format!("{collection_name}#f64#");
                    let _f64_table = open_table_write::<(u32, u32), f64>(&collection_name_f64, &write_txn);
                    println!("新建_动态值 for: {}", collection_name_f64);
                }

                println!("新建_indexes_tables");
                for index_f64 in &collection.indexes_f64_list {
                    let collection_name_index = format!("{}@f64@{}", collection_name, index_f64);
                    let _index_table = open_table_write::<(MyF64, u32), ()>(&collection_name_index, &write_txn);
                    println!("新建_index_f64 for: {}", collection_name_index);
                }

                for index_string in &collection.indexes_string_list {
                    let collection_name_index = format!("{}@string@{}", collection_name, index_string);
                    let index_table_define: MultimapTableDefinition<&str, u32> = MultimapTableDefinition::new(collection_name_index.as_str());
                    let _index_table = write_txn.open_multimap_table(index_table_define).unwrap();
                    println!("新建index_string for: {}", collection_name_index);
                }

                for index_string in &collection.indexes_string_unique_list {
                    let collection_name_index = format!("{}@stringU@{}", collection_name, index_string);
                    let _index_table = open_table_write::<&str, u32>(&collection_name_index, &write_txn);
                    println!("新建index_string_unique for: {}", collection_name_index);
                }
            }
            write_txn.commit().unwrap();
        }
        {
            let mut collection_map_lock = self.collection_map.write().unwrap();
            collection_map_lock.insert(collection_name, collection);
            drop(collection_map_lock);
        }
    }
    pub fn list_all_collections(&self) -> Vec<Collection> {
        {
            let collection_map_lock = self.collection_map.read().unwrap();
            let collections: Vec<Collection> = collection_map_lock.values().map(|c| c.clone()).collect();
            let collections_clone = collections.clone();
            return collections_clone;
        }
    }

    fn _add_index(&self, _collection_name: &String, _field_name: &String) {}
    pub fn update_records(&self, collection_name: &String, records: Vec<Value>, update_type: UpdateType) {
        let primary_key;
        let indexes_f64_list: Vec<String>;
        let indexes_string_list: Vec<String>;
        let indexes_string_unique_list: Vec<String>;

        {
            let collection_map_lock = self.collection_map.read().unwrap();
            let collection = collection_map_lock.get(collection_name).unwrap();
            primary_key = collection.primary_key.clone();
            indexes_f64_list = collection.indexes_f64_list.clone();
            indexes_string_list = collection.indexes_string_list.clone();
            indexes_string_unique_list = collection.indexes_string_unique_list.clone();
            //获取主键
            drop(collection_map_lock);
        }
        let records_len = records.len() as u32;
        let mut count_number = 100;

        match update_type {
            UpdateType::UpdateOnly => {}
            _ => {
                let mut counter_map_lock = self.counter_map.write().unwrap();

                {
                    let collection = counter_map_lock.get(collection_name).unwrap();
                    count_number = *collection;
                    let target_count = count_number + records_len;
                    counter_map_lock.insert(collection_name.clone(), target_count);
                }
                drop(counter_map_lock);
            }
        }


        let mut created_number: u32 = 0;

        let write_txn = self.db.begin_write().unwrap();
        {
            let mut collection_table = open_table_write::<u32, String>(&collection_name, &write_txn);

            let collection_name_primary = format!("{}@primary", collection_name);
            let mut primary_key_table = open_table_write::<&str, u32>(&collection_name_primary, &write_txn);

            let collection_name_f64 = format!("{collection_name}#f64#");
            let mut f64_table = open_table_write::<(u32, u32), f64>(&collection_name_f64, &write_txn);

            for record in records {
                let mut need_write = false;
                let mut need_skip = false;
                let mut is_new = false;
                let mut record_id = count_number + 1;
                let record_key_option = record[primary_key.as_str()].as_str();
                let mut old_record_option = None;
                if let Some(record_key) = record_key_option {
                    {
                        let old_record_id_option = primary_key_table.get(&record_key).unwrap();
                        match old_record_id_option {
                            None => {
                                is_new = true;
                            }
                            Some(old_record_id) => {
                                record_id = old_record_id.value();
                                let old_record_str_option = collection_table.get(record_id).unwrap();
                                if let Some(old_record_str_lock) = old_record_str_option {
                                    let old_record_str = old_record_str_lock.value();
                                    let old_record: Value = serde_json::from_str(old_record_str.as_str()).unwrap();
                                    old_record_option = Some(old_record);
                                }
                            }
                        }
                    }

                    match update_type {
                        UpdateType::CreateOnlY => {
                            if is_new {
                                count_number += 1;
                                created_number += 1;
                                need_write = true;
                            }
                        }
                        UpdateType::UpdateOnly => {
                            if !is_new {
                                need_write = true;
                            }
                        }
                        UpdateType::Merge => {
                            if is_new {
                                count_number += 1;
                                created_number += 1;
                            }
                            need_write = true;
                        }
                    }
                    if need_write {
                        for index_string in &indexes_string_unique_list {
                            let str_option = &record[index_string].as_str();
                            if let Some(str) = str_option {
                                let collection_name_index = format!("{}@stringU@{}", collection_name, index_string);
                                let mut index_table = open_table_write::<&str, u32>(&collection_name_index, &write_txn);
                                let is_conflict;
                                {
                                    let conflict_record_id_option = index_table.get(*str).unwrap();
                                    is_conflict = conflict_record_id_option.is_some();
                                }

                                if is_conflict && is_new {
                                    need_skip = true;
                                    println!("唯一索引冲突: {index_string}@{str}, record插入失败");
                                } else {
                                    let mut need_update_index = true;
                                    if is_new {
                                        println!("新建索引 for: {} with {}", collection_name_index, str);
                                    } else if let Some(ref old_record) = old_record_option {
                                        let old_str_option = &old_record[index_string].as_str();
                                        if let Some(old_str) = old_str_option {
                                            if *str == *old_str {
                                                println!("索引相同，无需更新, with {str}");
                                                need_update_index = false;
                                            } else {
                                                // println!("更新索引 for: {} with {}", collection_name_index, str);
                                                index_table.remove(*old_str).unwrap();
                                            }
                                        }
                                    }
                                    if need_update_index {
                                        println!("插入索引 for: {} with {}", collection_name_index, str);
                                        index_table.insert(*str, record_id).unwrap();
                                    }
                                }
                                //A,B  --> B,A  去重和old_str冲突
                            }
                        }

                        if need_skip {
                            continue;
                        }
                        for index_f64 in &indexes_f64_list {
                            let number_option = &record[index_f64].as_f64();
                            if let Some(number) = number_option {
                                let collection_name_index = format!("{}@f64@{}", collection_name, index_f64);
                                let mut index_table = open_table_write::<(MyF64, u32), ()>(&collection_name_index, &write_txn);
                                let mut need_update_index = true;
                                let field_id = hash_to_u32(index_f64);

                                if !is_new {
                                    let old_number_option = f64_table.get((record_id, field_id)).unwrap();

                                    if let Some(old_number_lock) = old_number_option {
                                        let old_number = old_number_lock.value();
                                        if old_number == *number {
                                            println!("索引相同，无需更新, with {number}");
                                            need_update_index = false;
                                        } else {
                                            // println!("更新索引 for: {} with {}", collection_name_index, number);
                                            index_table.remove((MyF64(old_number), record_id)).unwrap();
                                        }
                                    }
                                }

                                if need_update_index {
                                    println!("插入索引 for: {} with {}", collection_name_index, number);
                                    index_table.insert((MyF64(*number), record_id), ()).unwrap();
                                    f64_table.insert((record_id, field_id), *number).unwrap();
                                }
                            }
                        }

                        for index_string in &indexes_string_list {
                            let str_option = &record[index_string].as_str();
                            if let Some(str) = str_option {
                                let collection_name_index = format!("{}@string@{}", collection_name, index_string);
                                let index_table_define: MultimapTableDefinition<&str, u32> = MultimapTableDefinition::new(collection_name_index.as_str());
                                let mut index_table = write_txn.open_multimap_table(index_table_define).unwrap();
                                let mut need_update_index = true;
                                if !is_new {
                                    if let Some(ref old_record) = old_record_option {
                                        let old_str_option = &old_record[index_string].as_str();
                                        if let Some(old_str) = old_str_option {
                                            if *old_str == *str {
                                                println!("索引相同，无需更新, with {str}");
                                                need_update_index = false;
                                            } else {
                                                index_table.remove(old_str, record_id).unwrap();
                                            }
                                        }
                                    }
                                }

                                if need_update_index {
                                    index_table.insert(*str, record_id).unwrap();
                                    println!("插入索引 for: {} with {}", collection_name_index, str);
                                }
                            }
                        }

                        if !need_skip {
                            let record_str = serde_json::to_string(&record).unwrap_or("{}".to_string());
                            collection_table.insert(record_id, record_str).unwrap();
                            if is_new {
                                primary_key_table.insert(record_key, record_id).unwrap();
                            }
                        }
                    }
                }
            }

            if created_number > 0 {
                let mut counter_table = write_txn.open_table(COUNTER_TABLE).unwrap();
                let mut need_write = false;
                {
                    let old_counter_option = counter_table.get(collection_name).unwrap();
                    match old_counter_option {
                        None => {
                            need_write = true;
                        }
                        Some(old_counter) => {
                            if count_number > old_counter.value() {
                                need_write = true;
                            }
                        }
                    }
                }

                if need_write {
                    counter_table.insert(collection_name.clone(), count_number).unwrap();
                }
            }
        }
        write_txn.commit().unwrap();
    }


    fn _list_all_records(&self, collection_name: &String) -> Vec<Value> {
        let mut records = Vec::new();
        {
            let read_txn = self.db.begin_read().unwrap();
            let collection_table = open_table_read::<u32, String>(&collection_name, &read_txn);
            let mut iter = collection_table.iter().unwrap();
            while let Some(kv) = iter.next() {
                if let Ok((_key, value)) = kv {
                    // println!("读取collection value: {:?} @ {:?}", key.value(), value.value());
                    let record_str = value.value();
                    let record: Value = serde_json::from_str(record_str.as_str()).unwrap();
                    records.push(record);
                }
            }
        }
        return records;
    }

    fn _show_collection_inner(&self, collection_name: &String) -> (HashMap<String, u32>, HashMap<String, u32>, Vec<String>) {
        let mut primary_key_map = HashMap::new();
        let mut counter_map = HashMap::new();
        let mut collection_define_map = HashMap::new();

        let mut index_list: Vec<String> = Vec::new();

        {
            let read_txn = self.db.begin_read().unwrap();

            {
                let collection_name_primary = format!("{}@primary", collection_name);
                let primary_key_table = open_table_read::<&str, u32>(&collection_name_primary, &read_txn);
                let mut primary_key_table_iter = primary_key_table.iter().unwrap();
                while let Some(kv_primary_key) = primary_key_table_iter.next() {
                    if let Ok((key_lock, value_lock)) = kv_primary_key {
                        let key = key_lock.value();
                        let value = value_lock.value();
                        primary_key_map.insert(key.to_string(), value);
                    }
                }
            }

            {
                let collection_name_f64 = format!("{collection_name}#f64#");
                let f64_table = open_table_read::<(u32, u32), f64>(&collection_name_f64, &read_txn);
                let mut table_iter = f64_table.iter().unwrap();
                while let Some(kv) = table_iter.next() {
                    if let Ok((key_lock, value_lock)) = kv {
                        let key = key_lock.value();
                        let value = value_lock.value();
                        let index_str = format!("#{}#{:08x}->{value}", key.0, key.1);
                        index_list.push(index_str);
                    }
                }
            }

            {
                let counter_table = read_txn.open_table(COUNTER_TABLE).unwrap();
                let mut counter_table_iter = counter_table.iter().unwrap();
                '_label1: while let Some(kv_counter) = counter_table_iter.next() {
                    if let Ok((key_lock, value_lock)) = kv_counter {
                        let counter_name = key_lock.value();
                        let number = value_lock.value();
                        counter_map.insert(counter_name, number);
                    }
                }
            }

            {
                let collection_define_table = read_txn.open_table(COLLECTION_DEFINE_TABLE).unwrap();
                let mut collection_define_table_iter = collection_define_table.iter().unwrap();
                while let Some(kv) = collection_define_table_iter.next() {
                    if let Ok((key_lock, value_lock)) = kv {
                        let key = key_lock.value();
                        let value = value_lock.value();
                        let collection: Collection = serde_json::from_str(value.as_str()).unwrap();
                        collection_define_map.insert(key, collection);
                    }
                }
            }

            {
                let collection = collection_define_map.get(collection_name).unwrap();

                for index_string in &collection.indexes_string_unique_list {
                    let collection_name_index = format!("{}@stringU@{}", collection_name, index_string);
                    let index_table = open_table_read::<&str, u32>(&collection_name_index, &read_txn);
                    let mut index_table_iter = index_table.iter().unwrap();
                    while let Some(kv) = index_table_iter.next() {
                        if let Ok((key_lock, value_lock)) = kv {
                            let key = key_lock.value();
                            let value = value_lock.value();
                            let index_str = format!("$${key}->{value}");
                            index_list.push(index_str);
                        }
                    }
                }

                for index_f64 in &collection.indexes_f64_list {
                    let collection_name_index = format!("{}@f64@{}", collection_name, index_f64);
                    let index_table = open_table_read::<(MyF64, u32), ()>(&collection_name_index, &read_txn);
                    let mut index_table_iter = index_table.iter().unwrap();
                    while let Some(kv_counter) = index_table_iter.next() {
                        if let Ok((key_lock, value_lock)) = kv_counter {
                            let key = key_lock.value();
                            let _value = value_lock.value();

                            let index_str = format!("#{}@{}->{}", index_f64, key.0.0, key.1);
                            index_list.push(index_str);
                        }
                    }
                }

                for index_string in &collection.indexes_string_list {
                    let collection_name_index = format!("{}@string@{}", collection_name, index_string);
                    let index_table_define: MultimapTableDefinition<&str, u32> = MultimapTableDefinition::new(collection_name_index.as_str());
                    let index_table = read_txn.open_multimap_table(index_table_define).unwrap();
                    let mut index_table_iter = index_table.iter().unwrap();
                    while let Some(kv) = index_table_iter.next() {
                        if let Ok((key_lock, value_lock)) = kv {
                            let key = key_lock.value();
                            for id_lock in value_lock {
                                let record_id = id_lock.unwrap().value();
                                let index_str = format!("${}->{}", key, record_id);
                                index_list.push(index_str);
                            }
                        }
                    }
                }
            }
        }
        return (primary_key_map, counter_map,
                // collection_define_map,
                index_list);
    }

    fn _read_db(&self) -> ReadTransaction {
        let read_txn = self.db.begin_read().unwrap();
        read_txn
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{env, fs};
    use std::collections::{HashSet};
    use std::ops::Deref;
    use std::path::Path;

    use redb::{Database, ReadableTableMetadata, TableDefinition};
    use serde_json::json;

    use super::*;

    const STR_TABLE: TableDefinition<&str, &str> = TableDefinition::new("x");
    pub(crate) const DB_NAME: &str = "AABBCC_10015";
    const COLLECTION_NAME: &str = "Books";


    #[test]
    fn do_some_test_01() -> Result<(), Box<i32>> {
        println!("do_some_test_01");
        println!("do_some_test_01 done 1231321231321");
        println!("do_some_test_01");
        Ok(())
    }

    #[test]
    fn redb_len() {
        if let Ok(current_dir) = env::current_dir() {
            println!("当前路径是: {:?}", current_dir);
        } else {
            println!("无法获取当前路径");
        }
        fs::create_dir_all("MMG").unwrap();
        let test_01_db_file = Path::new("MMG/test_01.db");
        // let tmpfile = create_tempfile();
        let db = Database::create(test_01_db_file).unwrap();
        println!("redb_len @ {:?}", test_01_db_file);

        let read_txn = db.begin_read().unwrap();
        let table = read_txn.open_table(STR_TABLE).unwrap();
        assert_eq!(table.len().unwrap(), 3);
        let untyped_table = read_txn.open_untyped_table(STR_TABLE).unwrap();
        assert_eq!(untyped_table.len().unwrap(), 3);
    }

    //cargo test test_get_mgdb -- --show-output
    #[test]
    fn test_get_mgdb() -> Result<(), Box<i32>> {
        let mg_db = get_mgdb(DB_NAME.to_string());
        let write_txn = mg_db.deref().db.begin_write().unwrap();

        // let write_txn = mg_db.db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(STR_TABLE).unwrap();
            table.insert("hello", "world").unwrap();
            table.insert("hello2", "world2").unwrap();
            table.insert("hi", "world").unwrap();
        }
        write_txn.commit().unwrap();

        let read_txn = mg_db.db.begin_read().unwrap();
        let table = read_txn.open_table(STR_TABLE).unwrap();
        assert_eq!(table.len().unwrap(), 3);

        Ok(())
    }

    //cargo test test_create_collection -- --show-output
    #[test]
    fn test_create_collection() -> Result<(), Box<i32>> {
        println!("准备测试: test_create_collection");
        let mg_db = get_mgdb(DB_NAME.to_string());
        let schema: Schema = Schema {
            primary_key: "name".to_string(),
            indexes_f64: vec!["price".to_string()],
            indexes_string: vec!["book_type".to_string()],
            indexes_string_unique: vec!["book_uid".to_string()],
        };
        mg_db.create_collection(COLLECTION_NAME.to_string(), schema);
        let collections = mg_db.list_all_collections();
        println!("collections_str {}: {:#?}", COLLECTION_NAME, collections);
        println!("测试完毕: test_create_collection");
        Ok(())
    }

    //cargo test test_update_records -- --show-output
    #[test]
    fn test_update_records_0() -> Result<(), Box<i32>> {
        println!("准备测试: test_update_records");

        let mg_db = get_mgdb(DB_NAME.to_string());

        let value_1 = json!(
            {
                "name":"A1",
                "pa":1233,
                "pb":"其字符或计数器生成一系列字符串"
            }
        );

        let value_2 = json!(
            {
                "name":"A2",
                "pa":9999999,
                "pb":"语言教程中通常被"
            }
        );

        let records = vec![value_1, value_2];

        mg_db.update_records(&COLLECTION_NAME.to_string(), records, UpdateType::Merge);
        let records = mg_db._list_all_records(&COLLECTION_NAME.to_string());
        for record in records {
            println!("{}", serde_json::to_string(&record).unwrap());
        }

        println!("测试完毕: test_update_records");
        Ok(())
    }

    #[test]
    fn test_update_records_1() -> Result<(), Box<i32>> {
        println!("准备测试: test_update_records");

        let mg_db = get_mgdb(DB_NAME.to_string());

        let value_1 = json!(
            {
                "name":"A1",
                "pa":55001,
                "pb":"***************"
            }
        );

        let value_2 = json!(
            {
                "name":"A4",
                "pa":55003,
                "pb":"它用于验证非测试代码的功能是否和预期一致"
            }
        );

        let records = vec![value_1, value_2];
        let collection_name = COLLECTION_NAME.to_string();

        // mg_db.update_records(&collection_name, records, UpdateType::UpdateOnly);
        mg_db.update_records(&collection_name, records, UpdateType::CreateOnlY);
        let records = mg_db._list_all_records(&collection_name);
        for record in records {
            println!("{}", serde_json::to_string(&record).unwrap());
        }

        let primary_key_map = mg_db._show_collection_inner(&collection_name);
        println!("primary_key_map:{:#?}", primary_key_map);

        println!("测试完毕: test_update_records");
        Ok(())
    }

    //cargo test test_update_records_2 -- --show-output
    #[test]
    fn test_update_records_2() -> Result<(), Box<i32>> {
        println!("准备测试: test_update_records");

        let mg_db = get_mgdb(DB_NAME.to_string());
        let mut records = Vec::new();

        let types = vec!["Math", "Physics", "History"];
        for i in 0..10 {
            let book_type = types[(i + 2) % 3];
            let value = json!(
                {
                    "name":format!("BooK_a_{}", i),
                    "pa":999,
                    "pb":"***************",
                    "price":50000.5-(i as f64),
                    "book_type":book_type,
                    "book_uid":format!("uid_10_{i}")
                }
            );
            records.push(value);
        }

        let collection_name = COLLECTION_NAME.to_string();

        // mg_db.update_records(&collection_name, records, UpdateType::CreateOnlY);
        mg_db.update_records(&collection_name, records, UpdateType::CreateOnlY);
        let records = mg_db._list_all_records(&collection_name);
        for record in records {
            println!("{}", serde_json::to_string(&record).unwrap());
        }

        let info = mg_db._show_collection_inner(&collection_name);
        println!("primary_key_map:{:#?}", info);

        println!("测试完毕: test_update_records");
        Ok(())
    }
    //cargo test test_index_range_f64 -- --show-output

    #[test]
    fn test_index_range_f64() -> Result<(), Box<i32>> {
        println!("准备测试: test_index_range_f64");
        let collection_name = COLLECTION_NAME.to_string();
        let index_f64 = "price".to_string();

        let mg_db = get_mgdb(DB_NAME.to_string());
        let read_txn = mg_db._read_db();
        let collection_name_index = format!("{}@f64@{}", collection_name, index_f64);
        let index_table_define = TableDefinition::<(MyF64, u32), ()>::new(collection_name_index.as_str());
        let index_table = read_txn.open_table(index_table_define).unwrap();
        let range_cursor = index_table.range((MyF64(100.0), 0)..=(MyF64(106.0), u32::MAX)).unwrap();
        // while let Some(kv_counter) = range_cursor.next() {
        //     if let Ok((key_lock, value_lock)) = kv_counter {
        //         let key = key_lock.value();
        //         let _value = value_lock.value();
        //         println!("#{}@{}->{}", index_f64, key.0.0, key.1);
        //     }
        // }
        let ids_map = range_cursor.map(|v| v.unwrap().0.value().1);
        // let ids: BTreeSet<_> = ids_map.collect();//重建顺序
        let ids: Vec<_> = ids_map.collect(); //保顺序
        println!("ids;{ids:#?}");

        // while let Some(kv) = range_cursor.next() {
        //     if let Ok((key_lock, value_lock)) = kv {
        //         let key = key_lock.value();
        //         println!("Price: {}, record_id: {}", key.0.0, key.1);
        //     }
        // }

        println!("测试完毕: test_index_range_f64");
        Ok(())
    }

    //cargo test test_index_string -- --show-output
    #[test]
    fn test_index_string() -> Result<(), Box<i32>> {
        println!("准备测试: test_index_string");
        let collection_name = COLLECTION_NAME.to_string();
        let index_string = "book_type".to_string();
        let key = "Math";

        let mg_db = get_mgdb(DB_NAME.to_string());
        let read_txn = mg_db._read_db();

        let collection_name_index = format!("{}@string@{}", collection_name, index_string);
        let index_table_define: MultimapTableDefinition<&str, u32> = MultimapTableDefinition::new(collection_name_index.as_str());
        let index_table = read_txn.open_multimap_table(index_table_define).unwrap();

        let values = index_table.get(key).unwrap();

        let ids_map = values.map(|v| v.unwrap().value());
        let ids: HashSet<u32> = ids_map.collect();
        println!("ids:{ids:#?}");

        // for id_lock in values {
        //     let record_id = id_lock.unwrap().value();
        //     println!("{index_string}: {}, record_id: {}", key, record_id);
        // }
        println!("测试完毕: test_index_string");
        Ok(())
    }

    //cargo test test_show_collection_inner -- --show-output
    #[test]
    fn test_show_collection_inner() -> Result<(), Box<i32>> {
        println!("准备测试: test_update_records");
        let collection_name = COLLECTION_NAME.to_string();

        let mg_db = get_mgdb(DB_NAME.to_string());
        let info = mg_db._show_collection_inner(&collection_name);
        println!("primary_key_map:{:#?}", info);

        println!("测试完毕: test_update_records");
        Ok(())
    }
}