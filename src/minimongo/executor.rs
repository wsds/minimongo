use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use redb::{AccessGuard, MultimapTableDefinition, ReadableMultimapTable, ReadableTable, ReadableTableMetadata, ReadOnlyMultimapTable, ReadOnlyTable, ReadTransaction, TableDefinition, WriteTransaction};
use regex::Regex;
use serde_json::Value;
use common::helper::hash_to_u32;
use serde::{Deserialize, Serialize};
use crate::minimongo::lazy_set::{LazySet, MAX_FULL_LEN};
use crate::minimongo::minimongo::{Collection, MgDb};
use crate::minimongo::query::{Condition, ConditionExpression, ConditionOperation, ConditionResult, Expression, ExpressionEntity, Field, MainAction, Number, OrderBy, OrderDirection, parse_query, Query, ReturnAction, ValueRef, Where};
use crate::minimongo::query_helper::{MyF64, open_table_read};

#[derive(Debug, Serialize, Deserialize)]
enum ValuePack {
    List(Vec<Value>),
    Value(Value),
    IdList(Vec<u32>),
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryContext {
    variables: HashMap<String, ValuePack>, // 存储 AS 结果，键为变量名
    params: BTreeMap<String, Value>,
}

pub const DEFAULT_LIMIT: usize = 10;

impl MgDb {
    pub fn query_records(&self, query: &String, params: BTreeMap<String, Value>) -> BTreeMap<String, Value> {
        let query_chain = parse_query(query.as_str());
        // println!("{query_chain:#?}");

        let mut context = QueryContext {
            variables: HashMap::new(),
            params,
        };

        let mut result_name_set = BTreeSet::new();
        let mut final_result = BTreeMap::new();

        for query in query_chain {
            match query.main_action {
                MainAction::CREATE { .. } => {
                    self.execute_create(&query, &mut context);
                }
                MainAction::SELECT { .. } => {
                    self.execute_select(&query, &mut context);
                }
                MainAction::GROUP { .. } => {
                    self.execute_group(&query, &mut context);
                }
                _ => {}
            }

            match query.return_action {
                ReturnAction::None => {}
                ReturnAction::RETURN(ref result_name) => {
                    result_name_set.insert(result_name.clone());
                }
                ReturnAction::RETURNS(ref result_names) => {
                    let _: Vec<_> = result_names.iter().map(|r| result_name_set.insert(r.clone())).collect();
                }
            }
        }
        // println!("context: {context:#?}");

        // println!("RETURN: {result_name_set:#?}");
        for result_name in result_name_set {
            if let Some(value_pack) = context.variables.remove(&result_name) {
                match value_pack {
                    ValuePack::List(list) => {
                        let value = Value::Array(list);
                        final_result.insert(result_name, value);
                    }
                    ValuePack::Value(value) => {
                        final_result.insert(result_name, value);
                    }
                    ValuePack::IdList(_) => {}
                }
            }
        }
        // println!("final_result: {final_result:#?}");


        final_result
    }

    fn execute_create(&self, query: &Query, context: &mut QueryContext) {}
    fn execute_select(&self, query: &Query, context: &mut QueryContext) {
        if let MainAction::SELECT { target_collection, one } = &query.main_action {
            let collection = self.get_collection(target_collection);
            let read_txn = self.db.begin_read().unwrap();

            let mut filtered_record_ids_option = None;

            if let Some(wheres) = &query.wheres {
                let filtered_record_ids = filter_records(&collection, wheres, context, &read_txn);
                // println!("filtered_record_ids: {filtered_record_ids:#?}");
                filtered_record_ids_option = Some(filtered_record_ids);
            }

            let mut ordered_ids: Vec<u32> = if let Some(order_by) = &query.order_by {
                order_record_ids(&collection, order_by, &read_txn, filtered_record_ids_option, context)
            } else {
                match filtered_record_ids_option {
                    None =>
                        { default_record_ids(&collection, &read_txn) }
                    Some(record_id_map) =>
                        {
                            let limit = DEFAULT_LIMIT;
                            record_id_map.iter().take(limit).cloned().collect()
                        }
                }
            };

            // println!("ordered_ids: {ordered_ids:#?}");
            if *one {
                ordered_ids = ordered_ids.iter().take(1).cloned().collect();
                // println!("ordered_ids ONE: {ordered_ids:#?}");
            }


            let mut field_name_list = Vec::new();

            if let Some(field_define) = &query.field {
                for field in &field_define.fields {
                    match field {
                        Field::Name(field_name) => {
                            field_name_list.push(field_name.clone());
                        }
                        Field::Expression(Expression { string }) => {
                            if string == "*" {
                                field_name_list.push("ALL".to_string());
                            }
                        }
                    }
                }
            } else {
                field_name_list.push("ALL".to_string());
            }

            // println!("field_name_list: {field_name_list:#?}");

            export_data(ordered_ids, field_name_list, collection, context, read_txn, &query.as_action, one);
        }
    }

    fn execute_group(&self, query: &Query, context: &mut QueryContext) {}
}

fn export_data(ordered_ids: Vec<u32>, field_name_list: Vec<String>, collection: Collection, context: &mut QueryContext, read_txn: ReadTransaction, as_action: &String, one: &bool) {
    let collection_table = open_table_read::<u32, String>(&collection.collection_name, &read_txn);
    let mut results = Vec::new();
    for id in ordered_ids {
        let record_option = collection_table.get(id).unwrap();
        if let Some(record_lock) = record_option {
            let record_str = record_lock.value();
            let mut record: Value = serde_json::from_str(record_str.as_str()).unwrap();
            if let Value::Object(ref mut record_map) = record {
                if field_name_list.contains(&"ALL".to_string()) {
                    results.push(record);
                } else {
                    let mut sub_record = serde_json::Map::new();

                    for field_name in &field_name_list {
                        if let Some(value) = record_map.remove(field_name) {
                            sub_record.insert(field_name.clone(), value);
                        }
                    }
                    results.push(Value::from(sub_record));
                }
            }
        }
    }

    if *one {
        if results.len() >= 1 {
            let one_value = results.swap_remove(0);
            context.variables.insert(as_action.clone(), ValuePack::Value(one_value));
        } else {
            context.variables.insert(as_action.clone(), ValuePack::Value(Value::Null));
        }
    } else {
        context.variables.insert(as_action.clone(), ValuePack::List(results));
    }
}

impl MgDb {
    fn get_collection(&self, collection_name: &String) -> Collection {
        let cloned_collection;
        {
            let collection_map_lock = self.collection_map.read().unwrap();
            let collection = collection_map_lock.get(collection_name).unwrap();
            cloned_collection = collection.clone();
            drop(collection_map_lock);
        }
        cloned_collection
    }
}

fn paginate<T>(vec: &Vec<T>, skip: usize, limit: usize) -> Vec<T>
where
    T: Clone,
{
    vec.iter().skip(skip).take(limit).cloned().collect()
}

fn default_record_ids(collection: &Collection, read_txn: &ReadTransaction) -> Vec<u32> {
    let collection_table = open_table_read::<u32, String>(&collection.collection_name, &read_txn);
    let mut table_iter = collection_table.iter().unwrap();
    let limit = DEFAULT_LIMIT;
    let lock_ids = table_iter.take(limit);
    let ids: Vec<u32> = lock_ids.map(|id_result| id_result.unwrap().0.value()).collect();
    ids
}

fn order_record_ids(collection: &Collection, order_by: &OrderBy, read_txn: &ReadTransaction, mut filtered_record_ids_option: Option<BTreeSet<u32>>, context: &mut QueryContext) -> Vec<u32> {
    let mut skip: usize = 0;
    let mut limit: usize = DEFAULT_LIMIT;
    let skip_value = resolve_one_value_ref(&order_by.skip, context);
    if let Value::Number(skip_number) = skip_value {
        skip = skip_number.as_u64().unwrap() as usize;
    }
    let limit_value = resolve_one_value_ref(&order_by.limit, context);
    if let Value::Number(limit_number) = limit_value {
        limit = limit_number.as_u64().unwrap() as usize;
    }

    let order_field_type = check_field_type(collection, &order_by.field);
    let ordered_ids: Vec<u32> = match order_field_type {
        ConditionFieldType::F64 => {
            let field_id = hash_to_u32(&order_by.field);

            match filtered_record_ids_option {
                None => {
                    let collection_name_index = format!("{}@f64@{}", collection.collection_name, order_by.field);
                    let index_table = open_table_read::<(MyF64, u32), ()>(&collection_name_index, &read_txn);
                    let mut index_table_iter = index_table.iter().unwrap();
                    let ordered_ids = match order_by.order_direction {
                        OrderDirection::ASC => {
                            let lock_ids = index_table_iter.skip(skip).take(limit);
                            lock_ids.map(|id_result| id_result.unwrap().0.value().1).collect()
                        }
                        OrderDirection::DESC => {
                            let lock_ids = index_table_iter.rev().skip(skip).take(limit);
                            lock_ids.map(|id_result| id_result.unwrap().0.value().1).collect()
                        }
                    };
                    ordered_ids
                }
                Some(record_ids) => {
                    let mut results = Vec::new();
                    let collection_name_f64 = format!("{}#f64#", collection.collection_name);
                    let mut f64_table = open_table_read::<(u32, u32), f64>(&collection_name_f64, &read_txn);
                    for &record_id in &record_ids {
                        if let Some(value_lock) = f64_table.get((record_id, field_id)).unwrap() {
                            let value = value_lock.value();
                            results.push((value, record_id));
                        }
                    }
                    match order_by.order_direction {
                        OrderDirection::ASC => {
                            results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                        }
                        OrderDirection::DESC => {
                            results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                        }
                    }
                    let all_ordered_ids: Vec<u32> = results.iter().map(|(_n, id)| *id).collect();
                    let ordered_ids = paginate(&all_ordered_ids, skip, limit);
                    ordered_ids
                }
            }
        }
        _ => { //todo order by string
            Vec::new()
        }
    };
    ordered_ids
}

fn filter_records(collection: &Collection, wheres: &Where, context: &mut QueryContext, read_txn: &ReadTransaction) -> BTreeSet<u32> {
    let mut record_ids = BTreeSet::new();

    let mut condition_results = Vec::new();

    for condition in &wheres.conditions {
        match condition {
            Condition::OPERATION(operation) => {
                let operation_1 = operation.clone();
                let condition_result = ConditionResult::OPERATION(operation_1);
                condition_results.push(condition_result);
            }
            Condition::EXPRESSION(expression) => {
                let ids = filter_records_by_condition(collection, expression, context, read_txn);
                let condition_result = ConditionResult::IDS(ids);
                condition_results.push(condition_result);
            }
        }
    }
    // println!("判断 condition_results: {condition_results:#?}");

    record_ids = resolve_condition_results_if(condition_results, read_txn, collection);

    // println!("合并结果: {record_ids:#?}");

    record_ids
}

fn resolve_condition_results_recursive(condition_results: &[ConditionResult]) -> LazySet {
    if condition_results.len() == 1 {
        if let ConditionResult::IDS(ids) = &condition_results[0] {
            return LazySet::new(ids.clone());
        } else {
            return LazySet::new(BTreeSet::new());
        }
    }

    let mut min_priority = u64::MAX;
    let mut min_priority_index = None;

    for (i, result) in condition_results.iter().enumerate() {
        if let ConditionResult::OPERATION(operation) = result {
            let priority = match operation {
                ConditionOperation::NOT(p) | ConditionOperation::OR(p) | ConditionOperation::AND(p)
                => *p,
            };
            if priority < min_priority {
                min_priority = priority;
                min_priority_index = Some(i);
            }
        }
    }

    if let Some(operation_index) = min_priority_index {
        let left = &condition_results[..operation_index];
        let right = &condition_results[operation_index + 1..];

        let left_result = resolve_condition_results_recursive(left);
        let right_result = resolve_condition_results_recursive(right);

        if let ConditionResult::OPERATION(operation) = &condition_results[operation_index] {
            let result = match operation {
                ConditionOperation::NOT(_) => LazySet::complement(right_result.evaluate()),
                _ => {
                    let result_0 = left_result.merge(&right_result, operation.clone());
                    result_0
                }
            };
            return result;
        }
    }
    LazySet::new(BTreeSet::new())
}

fn resolve_condition_results(condition_results: Vec<ConditionResult>) -> BTreeSet<u32> {
    let result = resolve_condition_results_recursive(&condition_results);
    let evaluated = result.evaluate();
    evaluated
}

fn resolve_condition_results_if(condition_results: Vec<ConditionResult>, read_txn: &ReadTransaction, collection: &Collection) -> BTreeSet<u32> {
    let result = resolve_condition_results_recursive(&condition_results);
    let get_set_len = || {
        let collection_table = open_table_read::<u32, String>(&collection.collection_name, &read_txn);
        collection_table.len().unwrap_or(0) as u32
    };
    let get_full_set = || {
        let collection_table = open_table_read::<u32, String>(&collection.collection_name, &read_txn);
        let mut table_iter = collection_table.iter().unwrap();
        let lock_ids = table_iter.take(MAX_FULL_LEN as usize);
        let ids: BTreeSet<u32> = lock_ids.map(|id_result| id_result.unwrap().0.value()).collect();
        ids
    };
    let evaluated = result.evaluate_if(get_full_set, get_set_len);
    evaluated
}

fn filter_records_by_condition(collection: &Collection, condition: &ConditionExpression, context: &mut QueryContext, read_txn: &ReadTransaction) -> BTreeSet<u32> {
    // let record_ids = BTreeSet::new();

    let condition_field_type = check_field_type(collection, &condition.target_field);
    let record_ids = match condition_field_type {
        ConditionFieldType::PrimaryKey => {
            let collection_name_primary = format!("{}@primary", collection.collection_name);
            let primary_key_table = open_table_read::<&str, u32>(&collection_name_primary, &read_txn);
            filter_id_from_table_string_unique(&primary_key_table, &condition.expression_entity, context)
        }
        ConditionFieldType::F64 => {
            let collection_name_index = format!("{}@f64@{}", collection.collection_name, condition.target_field);
            let index_table = open_table_read::<(MyF64, u32), ()>(&collection_name_index, &read_txn);
            filter_id_from_table_f64(&index_table, &condition.expression_entity, context)
        }
        ConditionFieldType::String => {
            let collection_name_index = format!("{}@string@{}", collection.collection_name, condition.target_field);
            let index_table_define: MultimapTableDefinition<&str, u32> = MultimapTableDefinition::new(collection_name_index.as_str());
            let mut index_table = read_txn.open_multimap_table(index_table_define).unwrap();
            filter_id_from_table_string(&index_table, &condition.expression_entity, context)
        }
        ConditionFieldType::StringUnique => {
            let collection_name_index = format!("{}@stringU@{}", collection.collection_name, condition.target_field);
            let mut index_table = open_table_read::<&str, u32>(&collection_name_index, &read_txn);
            filter_id_from_table_string_unique(&index_table, &condition.expression_entity, context)
        }
        ConditionFieldType::NoIndex => {
            BTreeSet::new()
        }
    };
    // println!("判断 1 record_ids: {record_ids:#?}");

    record_ids
}

fn number_to_f64(number: &Number, context: &QueryContext) -> f64 {
    if let Number::ValueRef(value_ref) = number {
        let value = resolve_one_value_ref(value_ref, context);
        if let Value::Number(json_number) = value {
            let f64 = json_number.as_f64().unwrap_or(-1.0);
            return f64;
        }
    }

    number.to_f64()
}

fn filter_id_from_table_f64(table: &ReadOnlyTable<(MyF64, u32), ()>, expression_entity: &ExpressionEntity, context: &mut QueryContext) -> BTreeSet<u32> {
    let mut record_ids = match expression_entity {
        ExpressionEntity::IN { .. } => { BTreeSet::new() }
        ExpressionEntity::EQUAL { .. } => { BTreeSet::new() }
        ExpressionEntity::RANGE { max, min } => {
            let max_f64 = number_to_f64(max, context);
            let min_f64 = number_to_f64(min, context);

            let mut range_cursor = table.range((MyF64(min_f64), 0)..=(MyF64(max_f64), u32::MAX)).unwrap();
            let ids_map = range_cursor.map(|v| v.unwrap().0.value().1);
            let record_ids: BTreeSet<_> = ids_map.collect();
            record_ids
        }
        ExpressionEntity::REGEX { .. } => { BTreeSet::new() }
    };

    record_ids
}

fn filter_id_from_table_string(table: &ReadOnlyMultimapTable<&str, u32>, expression_entity: &ExpressionEntity, context: &mut QueryContext) -> BTreeSet<u32> {
    let mut record_ids = BTreeSet::new();

    match expression_entity {
        ExpressionEntity::IN { value_ref } => {
            let value_list = resolve_list_value_ref(value_ref, context);
            let string_list: Vec<String> = value_list.iter().map(|v| v.as_str().unwrap_or("None").to_string()).collect();
            for key in string_list {
                let values = table.get(key.as_str()).unwrap();
                let mut ids: BTreeSet<u32> = values.map(|v| v.unwrap().value()).collect();
                record_ids.append(&mut ids);
            }
        }
        ExpressionEntity::EQUAL { value_ref } => {
            let value = resolve_one_value_ref(value_ref, context);
            // println!("判断是否等于value_ref: {value:#?}");

            if let Value::String(key) = value {
                let values = table.get(key.as_str()).unwrap();
                let mut ids: BTreeSet<u32> = values.map(|v| v.unwrap().value()).collect();
                // println!("判断 ids: {ids:#?}");

                record_ids.append(&mut ids);
                // println!("判断 0 record_ids: {record_ids:#?}");
            }
        }
        ExpressionEntity::RANGE { .. } => {}
        ExpressionEntity::REGEX { reg } => {
            let this_reg_result = Regex::new(reg.as_str());
            if let Ok(this_reg) = this_reg_result {
                let mut iter = table.iter().unwrap();
                while let Some(kv) = iter.next() {
                    if let Ok((key_lock, values)) = kv {
                        let key = key_lock.value();
                        if this_reg.is_match(key) {
                            let mut ids: BTreeSet<u32> = values.map(|v| v.unwrap().value()).collect();
                            record_ids.append(&mut ids)
                        }
                    }
                }
            }
        }
    }
    record_ids
}


fn filter_id_from_table_string_unique(table: &ReadOnlyTable<&str, u32>, expression_entity: &ExpressionEntity, context: &mut QueryContext) -> BTreeSet<u32> {
    let mut record_ids = BTreeSet::new();

    match expression_entity {
        ExpressionEntity::IN { value_ref } => {
            let value_list = resolve_list_value_ref(value_ref, context);
            let string_list: Vec<&str> = value_list.iter().map(|v| v.as_str().unwrap_or("None")).collect();
            for key in string_list {
                let record_id_option = table.get(key).unwrap();
                match record_id_option {
                    None => {}
                    Some(record_id_value) => {
                        let record_id = record_id_value.value();
                        record_ids.insert(record_id);
                    }
                }
            }
        }
        ExpressionEntity::EQUAL { value_ref } => {
            let value = resolve_one_value_ref(value_ref, context);
            if let Value::String(key) = value {
                let record_id_option = table.get(key.as_str()).unwrap();
                match record_id_option {
                    None => {}
                    Some(record_id_value) => {
                        let record_id = record_id_value.value();
                        record_ids.insert(record_id);
                    }
                }
            }
        }
        ExpressionEntity::RANGE { .. } => {}
        ExpressionEntity::REGEX { reg } => {
            let this_reg_result = Regex::new(reg.as_str());
            if let Ok(this_reg) = this_reg_result {
                let mut iter = table.iter().unwrap();
                while let Some(kv) = iter.next() {
                    if let Ok((key_lock, value_lock)) = kv {
                        let key = key_lock.value();
                        if this_reg.is_match(key) {
                            let record_id = value_lock.value();
                            record_ids.insert(record_id);
                        }
                    }
                }
            }
        }
    }
    record_ids
}

fn resolve_one_value_ref(value_ref: &ValueRef, context: &QueryContext) -> Value {
    let mut value = Value::Null;
    match value_ref {
        ValueRef::Ref(key) => {
            let mut value_option = context.params.get(key);
            if value_option.is_none() {
                let value_pack_option = context.variables.get(key);
                if let Some(value_pack) = value_pack_option {
                    match value_pack {
                        ValuePack::List(list) => { value = list[0].clone(); }
                        ValuePack::Value(one_value) => { value = one_value.clone(); }
                        ValuePack::IdList(_) => {}
                    }
                }
            } else {
                value = value_option.unwrap_or(&Value::Null).clone();
            }
        }
        ValueRef::Value(one_value) => { value = one_value.clone(); }
    }
    value
}

fn resolve_list_value_ref(value_ref: &ValueRef, context: &mut QueryContext) -> Vec<Value> {
    let mut value_list = Vec::new();
    let mut this_value = Value::Null;
    match value_ref {
        ValueRef::Ref(key) => {
            let mut value_option = context.params.get(key);
            if value_option.is_none() {
                let value_pack_option = context.variables.get(key);
                if let Some(value_pack) = value_pack_option {
                    match value_pack {
                        ValuePack::List(list) => { value_list = list.clone(); }
                        ValuePack::Value(one_value) => {
                            this_value = one_value.clone();
                        }
                        ValuePack::IdList(_) => {}
                    }
                }
            } else {
                let one_value = value_option.unwrap_or(&Value::Null).clone();
                this_value = one_value.clone();
            }
        }
        ValueRef::Value(one_value) => { this_value = one_value.clone(); }
    }
    match this_value {
        Value::Null => {}
        Value::Array(list) => {
            value_list = list;
        }
        _ => {
            value_list.push(this_value);
        }
    }

    value_list
}

fn check_field_type(collection: &Collection, target_field: &String) -> ConditionFieldType {
    if *collection.primary_key == *target_field {
        ConditionFieldType::PrimaryKey
    } else if collection.indexes_string_unique_list.contains(target_field) {
        ConditionFieldType::StringUnique
    } else if collection.indexes_string_list.contains(target_field) {
        ConditionFieldType::String
    } else if collection.indexes_f64_list.contains(target_field) {
        ConditionFieldType::F64
    } else {
        ConditionFieldType::NoIndex
    }
}

enum ConditionFieldType {
    PrimaryKey,
    F64,
    String,
    StringUnique,
    NoIndex,
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use serde_json::{json, Value};
    use crate::minimongo::executor::{paginate, resolve_condition_results};
    use crate::minimongo::minimongo::get_mgdb;
    use crate::minimongo::minimongo::tests::DB_NAME;
    use crate::minimongo::query::{ConditionOperation, ConditionResult};

    const SQL_STR_2: &str = include_str!("Test2.SQL");
    const SQL_STR_3: &str = include_str!("Test3.SQL");

    //cargo test do_some_test_03 -- --show-output
    #[test]
    fn do_some_test_03() -> Result<(), Box<i32>> {
        println!("do_some_test_03");
        println!("{}", SQL_STR_2);
        println!("do_some_test_03 done 88888888888888888888");
        let A = "AAAA".to_string();
        let B = "AAAA".to_string();
        let c = A == B;
        Ok(())
    }

    //cargo test test_db_query_2 -- --show-output
    #[test]
    fn test_db_query_2() -> Result<(), Box<i32>> {
        println!("test_db_query");
        let mg_db = get_mgdb(DB_NAME.to_string());
        let params = BTreeMap::new();
        let query = SQL_STR_2.to_string();
        mg_db.query_records(&query, params);

        println!("test_db_query done");
        Ok(())
    }

    //cargo test test_db_query_3 -- --show-output
    #[test]
    fn test_db_query_3() -> Result<(), Box<i32>> {
        println!("test_db_query");
        let mg_db = get_mgdb(DB_NAME.to_string());
        let params_data = json!({
            "name": "Alice",
            "max_price": 97,
            "book_type": "History",
            "skip": 1
        });
        let params: BTreeMap<String, Value> = if let Value::Object(map) = params_data {
            map.into_iter().collect()
        } else {
            BTreeMap::new()
        };

        let query = SQL_STR_3.to_string();
        let final_result = mg_db.query_records(&query, params);

        let final_result_str = serde_json::to_string_pretty(&final_result).unwrap();
        println!("final_result_str: {final_result_str}");

        println!("test_db_query done");
        Ok(())
    }

    //cargo test test_lazy_set_condition_0 -- --show-output
    #[test]
    fn test_lazy_set_condition_0() {
        let set1: BTreeSet<u32> = [1, 2, 3].into_iter().collect();
        let set2: BTreeSet<u32> = [3, 4, 5].into_iter().collect();
        let set3: BTreeSet<u32> = [5, 6, 7].into_iter().collect();

        let condition_results = vec![
            ConditionResult::IDS(set1),
            ConditionResult::OPERATION(ConditionOperation::OR(1)),
            ConditionResult::IDS(set2),
            ConditionResult::OPERATION(ConditionOperation::AND(2)),
            ConditionResult::IDS(set3),
        ];

        let result = resolve_condition_results(condition_results);

        println!("Final Result: {:?}", result)
    }

    //cargo test test_lazy_set_condition_1 -- --show-output
    #[test]
    fn test_lazy_set_condition_1() {
        let set1: BTreeSet<u32> = [1000000002, 1000000005, 1000000008].into_iter().collect();
        let set2: BTreeSet<u32> = [1000000003, 1000000006, 1000000009].into_iter().collect();

        let condition_results = vec![
            ConditionResult::IDS(set1),
            ConditionResult::OPERATION(ConditionOperation::OR(0)),
            ConditionResult::IDS(set2),
        ];

        let result = resolve_condition_results(condition_results);

        println!("Final Result: {:?}", result)
    }

    //cargo test test_paginate -- --show-output
    #[test]
    fn test_paginate() {
        let data = vec![1, 2, 3, 4, 5, 6, 7];
        let skip = 15;
        let limit = 5;

        let result = paginate(&data, skip, limit);
        println!("{:#?}", result);
    }

    fn test_take_from_json() {
        let mut v = json!({ "x": "y" });
        assert_eq!(v["x"].take(), json!("y"));
        assert_eq!(v, json!({ "x": null }));
        let result = v.take();
        println!("{:#?}", result);
    }

    //cargo test test_vec_remove -- --show-output
    #[test]
    fn test_vec_remove() {
        let mut v = vec!["foo", "bar", "baz", "qux"];

        assert_eq!(v.swap_remove(1), "bar");
        assert_eq!(v, ["foo", "qux", "baz"]);
        println!("删除前 {:#?}", v);

        assert_eq!(v.swap_remove(10), "bar");

        println!("删除后 {:#?}", v);
    }
}
