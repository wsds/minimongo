use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, LazyLock, RwLock};
use enum_stringify::EnumStringify;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use regex::Regex;
use crate::minimongo::executor::DEFAULT_LIMIT;
use crate::minimongo::query_helper::MyRegex;

#[derive(Debug, Serialize, Deserialize)]
pub enum UpdateType {
    CreateOnlY,
    UpdateOnly,
    Merge,
}


#[derive(Debug, Serialize, Deserialize, Default)]
pub enum MainAction {
    CREATE {
        one: bool,
        update_type: UpdateType,
        target_collection: String,
        value_ref: ValueRef,
    },
    SELECT {
        one: bool,
        target_collection: String,
    },
    GROUP {
        target_collection: String,
        by: String,
    },
    #[default]
    NONE,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ValueRef {
    Ref(String),
    Value(Value),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Number {
    Int64(i64),
    Float64(f64),
    Infinity,
    NegInfinity,
    ValueRef(ValueRef),
} //todo add value_ref

impl Number {
    pub fn is_string(&self) -> bool {
        match self {
            Number::Int64(_) => {}
            Number::Float64(_) => {}
            Number::Infinity => {}
            Number::NegInfinity => {}
            Number::ValueRef(value_ref) => {
                match value_ref {
                    ValueRef::Ref(_) => {}
                    ValueRef::Value(_) => {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    pub fn is_ref(&self) -> bool {
        match self {
            Number::Int64(_) => {}
            Number::Float64(_) => {}
            Number::Infinity => {}
            Number::NegInfinity => {}
            Number::ValueRef(value_ref) => {
                match value_ref {
                    ValueRef::Ref(_) => {
                        return true;
                    }
                    ValueRef::Value(_) => {}
                }
            }
        }
        return false;
    }

    pub fn to_f64(&self) -> f64 {
        let f64: f64 = match self {
            Number::Int64(int64) => { *int64 as f64 }
            Number::Float64(f64) => { *f64 }
            Number::Infinity => { f64::INFINITY }
            Number::NegInfinity => { f64::NEG_INFINITY }
            Number::ValueRef(_) => { -1.0 }
        };
        f64
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExpressionEntity {
    IN { value_ref: ValueRef },
    EQUAL { value_ref: ValueRef },
    RANGE { max: Number, min: Number },
    REGEX { reg: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConditionExpression {
    expression: String,
    pub target_field: String,
    pub expression_entity: ExpressionEntity,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConditionOperation {
    NOT(u64),
    OR(u64),
    AND(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConditionResult {
    OPERATION(ConditionOperation),
    IDS(BTreeSet<u32>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Condition {
    OPERATION(ConditionOperation),
    EXPRESSION(ConditionExpression),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Where {
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expression {
    pub string: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
enum WriteAction {
    #[default]
    NONE,
    DELETE,
    UPDATE {
        expressions: Vec<Expression>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Field {
    Name(String),
    Expression(Expression),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldDefine {
    pub fields: Vec<Field>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum ReturnAction {
    #[default]
    None,
    RETURN(String),
    RETURNS(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum OrderDirection {
    #[default]
    ASC, //升序
    DESC, //降序
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderBy {
    pub field: String,
    pub limit: ValueRef,
    pub skip: ValueRef,
    pub order_direction: OrderDirection,
} //todo value_ref

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Query {
    pub main_action: MainAction,
    pub wheres: Option<Where>,
    pub having: Option<Where>,
    pub write_action: WriteAction,
    pub field: Option<FieldDefine>,
    pub order_by: Option<OrderBy>,
    pub as_action: String,
    pub return_action: ReturnAction,
    // #[serde(skip_serializing, skip_deserializing)]
    // next: Option<Arc<Query>>,
}

#[derive(EnumStringify, PartialEq)]
enum KeyWord {
    //语句关键字
    CREATE,
    SELECT,
    GROUP,
    AS,
    ORDERBY,
    RETURN,
    UPDATE,
    DELETE,
    FIELD,
    WHERE,
    HAVING,
}

#[derive(EnumStringify, PartialEq)]
enum BaseWord {
    //辅助关键字
    LIMIT,
    SKIP,
    ONE,
    CREATEONLY,
    UPDATEONLY,
    MERGE,
    BY,
    IN,
    REGEX,
    AND,
    OR,
    NOT,
    ASC, //升序
    DESC, //降序
}

pub fn parse_query(query: &str) -> Vec<Query> {
    let mut query_chain: Vec<Query> = Vec::new();
    let mut lines = query.lines().map(|line| line.trim());
    let mut current_query = Query::default();

    let mut current_keyword = KeyWord::SELECT;
    let mut current_words: Vec<&str> = Vec::new();

    while let Some(line) = lines.next() {
        let line_trim = line.trim();
        if line_trim.is_empty() || line_trim.starts_with('#') {
            continue; // 跳过空行和注释
        }

        let mut words: Vec<&str> = line_trim.split_whitespace().collect();

        let first_word = words[0];
        if let Ok(keyword) = KeyWord::try_from(first_word) {
            if !current_words.is_empty() {
                let end = parse_line(&current_keyword, &current_words, &mut current_query);
                current_words.clear();
                if keyword == KeyWord::RETURN {
                    // println!("返回指令");
                    let _end = parse_line(&keyword, &words, &mut current_query);
                }

                if end {
                    query_chain.push(current_query);
                    current_query = Query::default();
                }
            }

            current_words.append(&mut words);
            current_keyword = keyword;
            // println!("发现keywords： {}", keyword)
        } else {
            current_words.append(&mut words);
            // println!("内容行")
        }
    }
    return query_chain;
}

fn parse_line(keyword: &KeyWord, words: &Vec<&str>, query: &mut Query) -> bool {
    // println!("处理keywords： {}", keyword);
    // println!("内容： {:#?}", current_words);
    match keyword {
        KeyWord::CREATE => { parse_create(words, query); }
        KeyWord::SELECT => { parse_select(words, query); }
        KeyWord::GROUP => { parse_group(words, query); }
        KeyWord::AS => {
            let as_action = words[1].to_string();
            query.as_action = as_action;
            return true;
        }
        KeyWord::ORDERBY => { parse_order_by(words, query); }
        KeyWord::RETURN => { parse_return(words, query); }
        KeyWord::UPDATE => { parse_update(words, query); }
        KeyWord::DELETE => { parse_delete(words, query); }
        KeyWord::FIELD => { parse_field(words, query); }
        KeyWord::WHERE => { parse_where(words, query); }
        KeyWord::HAVING => { parse_having(words, query); }
        // _ => {}
    }
    false
}

fn parse_create(words: &Vec<&str>, query: &mut Query) {
    if words.len() < 3 {
        return;
    }
    let mut index_offset = 0;
    let one = words.contains(&"ONE");
    if one {
        index_offset = 1;
    }
    let update_type = if words.contains(&"CREATEONLY") {
        UpdateType::CreateOnlY
    } else if words.contains(&"UPDATEONLY") {
        UpdateType::UpdateOnly
    } else {
        UpdateType::Merge
    };
    let target_collection = words[1 + index_offset].to_string();

    query.main_action = MainAction::CREATE {
        one,
        update_type,
        target_collection,
        value_ref: ValueRef::Ref("bbb".to_string()), //todo
    };
}

fn parse_select(words: &Vec<&str>, query: &mut Query) {
    if words.len() < 2 {
        return;
    }
    let mut index_offset = 0;
    let one = words.contains(&"ONE");
    if one {
        index_offset = 1;
    }
    let target_collection = words[1 + index_offset].to_string();
    query.main_action = MainAction::SELECT { one, target_collection };
}

fn parse_group(words: &Vec<&str>, query: &mut Query) {
    if words.len() != 4 {
        return;
    }

    let target_collection = words[1].to_string();
    let by = words[3].to_string();

    query.main_action = MainAction::GROUP { target_collection, by };
}

fn parse_order_by(words: &Vec<&str>, query: &mut Query) {
    let len = words.len();
    if len < 2 {
        return;
    }
    let field = words[1].to_string();
    let mut limit;
    let mut skip;

    let skip_index = words.iter().position(|&w| w == "SKIP").unwrap_or(0);
    if len > skip_index + 1 && skip_index > 0 {
        let skip_str = words[skip_index + 1];
        skip = parse_value(skip_str);
    } else {
        skip = ValueRef::Value(Value::from(0));
    }

    let limit_index = words.iter().position(|&w| w == "LIMIT").unwrap_or(0);
    if len > limit_index + 1 && limit_index > 0 {
        let limit_str = words[limit_index + 1];
        limit = parse_value(limit_str);
    } else {
        limit = ValueRef::Value(Value::from(DEFAULT_LIMIT));
    }

    let mut order_direction = OrderDirection::ASC;
    if words.contains(&"DESC") {
        order_direction = OrderDirection::DESC;
    }

    query.order_by = Some(OrderBy { field, limit, skip, order_direction });
}

fn parse_return(words: &Vec<&str>, query: &mut Query) {
    if words.len() < 2 {
        return;
    }
    let return_fields_str = words[1..].join("");
    let return_fields: Vec<&str> = return_fields_str.split(",").collect();
    query.return_action = if return_fields.len() == 1 {
        ReturnAction::RETURN(return_fields[0].to_string())
    } else {
        ReturnAction::RETURNS(return_fields.iter().map(|s| s.to_string()).collect())
    };
}

fn parse_update(words: &Vec<&str>, query: &mut Query) {
    match query.write_action {
        WriteAction::NONE => {}
        _ => { return; }
    }

    let expressions_str = words[1..].join("");
    let expressions_vec: Vec<&str> = expressions_str.split(",").collect();
    let expressions: Vec<Expression> = expressions_vec.iter().filter(|s| s.len() > 0).map(|s| Expression { string: s.to_string() }).collect();

    query.write_action = WriteAction::UPDATE {
        expressions,
    }
}

fn parse_delete(_words: &Vec<&str>, query: &mut Query) {
    match query.write_action {
        WriteAction::NONE => {}
        _ => { return; }
    }
    query.write_action = WriteAction::DELETE;
}

static EXPR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let reg: Regex = Regex::new(r"[+\-*/()=]").unwrap();
    reg
});

fn is_expression(input: &str) -> bool {
    EXPR_REGEX.is_match(input)
}

fn parse_field(words: &Vec<&str>, query: &mut Query) {
    if words.len() < 2 {
        return;
    }

    let expressions_str = words[1..].join("");
    let expressions_vec: Vec<&str> = expressions_str.split(",").collect();
    let mut fields: Vec<Field> = Vec::new();
    for expression_str in expressions_vec {
        if expression_str.len() == 0 {
            continue;
        }
        let field = if is_expression(expression_str) {
            Field::Expression(Expression {
                string: expression_str.to_string()
            })
        } else {
            Field::Name(expression_str.to_string())
        };
        fields.push(field);
    }

    query.field = Some(FieldDefine {
        fields,
    });
}

fn parse_where(words: &Vec<&str>, query: &mut Query) {
    query.wheres = Some(parse_condition_block(words));
}

fn parse_having(words: &Vec<&str>, query: &mut Query) {
    query.having = Some(parse_condition_block(words));
}

static LOGIC_KEYWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let reg: Regex = Regex::new(r"^(OR|AND|NOT)(?:\[(\d+)])?$").unwrap();
    reg
});

fn parse_keyword_and_number(input: &str) -> Option<(String, u64)> {
    if let Some(cap) = LOGIC_KEYWORD_REGEX.captures(input) {
        let keyword = cap[1].to_string();
        let number = cap.get(2).map_or(0, |m| m.as_str().parse::<u64>().unwrap_or(0));
        return Some((keyword, number));
    }
    None
}

fn parse_condition_block(words: &Vec<&str>) -> Where {
    let block_words = words[1..].to_vec();

    let mut conditions = Vec::new();

    // let current_condition= Condition::def
    let mut current_words: Vec<&str> = Vec::new();
    let mut is_not = false;

    // while let Some(word) = block_words.iter().next() {
    for word in block_words {
        match parse_keyword_and_number(word) {
            None => {
                match word {
                    "IN" => { current_words.push(" IN "); }
                    "REGEX" => { current_words.push(" REGEX "); }
                    &_ => { current_words.push(word); }
                }
            }
            Some((keyword, number)) => {
                let condition_operation = match keyword.as_str() {
                    "NOT" => { ConditionOperation::NOT(number) }
                    "OR" => { ConditionOperation::OR(number) }
                    "AND" => { ConditionOperation::AND(number) }
                    &_ => { ConditionOperation::OR(number) }
                };
                // println!("处理逻辑运算符 {:?}", condition_operation);
                if current_words.len() > 0 {
                    let expression = current_words.join("");
                    let condition_expression = parse_condition_expression(expression);
                    conditions.push(Condition::EXPRESSION(condition_expression));
                    current_words.clear();
                }

                conditions.push(Condition::OPERATION(condition_operation));
            }
        }
    }
    if current_words.len() > 0 {
        let expression = current_words.join("");
        let condition_expression = parse_condition_expression(expression);
        conditions.push(Condition::EXPRESSION(condition_expression));
    }

    Where { conditions }
}

fn parse_value(value_ref_str: &str) -> ValueRef {
    let value_ref_str = value_ref_str.trim().trim_matches('"');
    let value_ref = if value_ref_str.starts_with('$') {
        ValueRef::Ref(value_ref_str[1..].to_string())
    } else {
        let value = string_to_json_value(value_ref_str);

        ValueRef::Value(value)
    };
    value_ref
}

fn string_to_json_value(input: &str) -> Value {
    if let Ok(parsed_int) = input.parse::<i64>() {
        Value::Number(serde_json::Number::from(parsed_int))
    } else if let Ok(parsed_float) = input.parse::<f64>() {
        Value::Number(serde_json::Number::from_f64(parsed_float).unwrap())
    } else {
        json!(input)
    }
}

fn parse_condition_expression(expression: String) -> ConditionExpression {
    let expression = expression.trim().to_string();

    if expression.contains(" IN ") {
        // 处理 IN 表达式
        let parts: Vec<&str> = expression.split(" IN ").collect();
        let target_field = parts[0].trim().to_string();
        let value_ref_str = parts[1].trim();
        let value_ref = parse_value(value_ref_str);

        ConditionExpression {
            expression,
            target_field,
            expression_entity: ExpressionEntity::IN { value_ref },
        }
    } else if expression.contains(" REGEX ") {
        // 处理 REGEX 表达式
        let parts: Vec<&str> = expression.split(" REGEX ").collect();
        let target_field = parts[0].trim().to_string();
        let reg = parts[1].trim().to_string();

        ConditionExpression {
            expression,
            target_field,
            expression_entity: ExpressionEntity::REGEX { reg },
        }
    } else if expression.contains('=') {
        // 处理 EQUAL 表达式
        let parts: Vec<&str> = expression.split('=').collect();
        let target_field = parts[0].trim().to_string();
        let value_ref_str = parts[1].trim();
        let value_ref = parse_value(value_ref_str);
        ConditionExpression {
            expression,
            target_field,
            expression_entity: ExpressionEntity::EQUAL { value_ref },
        }
    } else if expression.contains('>') || expression.contains('<') {
        let (min, target_field, max) = parse_range_expression(expression.as_str());
        ConditionExpression {
            expression,
            target_field,
            expression_entity: ExpressionEntity::RANGE { max, min },
        }
    } else {
        panic!("Unsupported expression type");
    }
}

fn string_to_number(input: &str) -> Number {
    if let Ok(parsed_int) = input.parse::<i64>() {
        Number::Int64(parsed_int)
    } else if let Ok(parsed_float) = input.parse::<f64>() {
        Number::Float64(parsed_float)
    } else {
        let value_ref = if input.starts_with('$') {
            ValueRef::Ref(input[1..].to_string())
        } else {
            ValueRef::Value(Value::String(input.to_string()))
        };
        Number::ValueRef(value_ref)
    }
}

fn parse_range_expression(expression: &str) -> (Number, String, Number) {
    let mut asc_parts = expression.split('<').collect::<Vec<&str>>();
    let mut desc_parts = expression.split('>').collect::<Vec<&str>>();
    // println!("test_string_parse:{:#?}", asc_parts);
    // println!("test_string_parse:{:#?}", desc_parts);
    let mut is_asc = true;
    let parts = if asc_parts.len() > 1 { asc_parts } else if desc_parts.len() > 1 {
        is_asc = false;
        desc_parts
    } else { vec![] };
    let mut target_field = "".to_string();
    let mut min = Number::NegInfinity;
    let mut max = Number::Infinity;
    let mut target_index = parts.len();
    for (index, part) in parts.iter().enumerate() {
        let number = string_to_number(part.trim());
        // println!("parse: {:#?} @ {}", number, index);
        if number.is_string() {
            target_field = part.trim().to_string();
            target_index = index;
        } else if is_asc {
            if index < target_index {
                min = number;
            } else if index > target_index {
                max = number;
            }
        } else if !is_asc {
            if index < target_index {
                max = number;
            } else if index > target_index {
                min = number;
            }
        }
    }
    (min, target_field, max)
}


#[cfg(test)]
mod tests {
    use crate::minimongo::query::{parse_condition_expression, parse_query, parse_range_expression};

    //cargo test do_some_test_02 -- --show-output
    const SQL_STR_1: &str = include_str!("Test1.SQL");

    #[test]
    fn do_some_test_02() -> Result<(), Box<i32>> {
        println!("do_some_test_02");
        println!("{}", SQL_STR_1);
        println!("do_some_test_02 done 88888888888888888888");
        Ok(())
    }

    //cargo test test_parse_query -- --show-output
    #[test]
    fn test_parse_query() -> Result<(), Box<i32>> {
        println!("test_parse_query");
        // println!("{}", SQL_STR_1);
        let query_chain = parse_query(SQL_STR_1);
        println!("{:#?}", query_chain);
        println!("test_parse_query done");
        Ok(())
    }


    //cargo test test_parse_condition_expression -- --show-output
    #[test]
    fn test_parse_condition_expression() {
        let expressions = vec![
            "x IN $list_x",
            "name=Lily",
            "90.2>price>10.5",
            "name REGEX `^A.*`",
            "x > 2",
            "x < 2",
            "321 < x < 2",
            "321 > x < 2",
            "321 > x < 2666",
            "321 < x > 2",
        ];

        for expression in expressions {
            let parsed = parse_condition_expression(expression.to_string());
            println!("{:#?}", parsed);
        }
    }

    //cargo test test_string_parse -- --show-output
    #[test]
    fn test_string_parse() {
        // let expression_str = "321 < x < 2666";
        // let expression_str = "3221 > x > 2666";
        // let expression_str = "x > 2666";
        // let expression_str = "x < 2666";
        let expression_str = "-656 > XXX";
        // let expression_str = "-656 < XXX";

        let (min, target_field, max) = parse_range_expression(expression_str);
        println!("parse: {:?} < {} < {:?}", min, target_field, max);
    }
}
