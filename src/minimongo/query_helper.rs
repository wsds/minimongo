use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use redb::{Key, ReadOnlyTable, ReadTransaction, Table, TableDefinition, TypeName, Value, WriteTransaction};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};

#[derive(Debug)]
pub struct MyRegex(Regex);

// impl Serialize for MyRegex {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let str = self.0.to_string();
//         Ok(str)
//     }
// }
//
//
// impl Deserialize for MyRegex{
//     fn deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>
//     {
//
//         deserializer.deserialize_string(|v|{})
//     }
//
//     fn deserialize_in_place<'de, D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
//     where
//         D: Deserializer<'de>
//     {
//         deserializer.deserialize_string(|v|{})
//     }
// }

pub struct MyF64(pub f64);

impl Debug for MyF64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Value for MyF64 {
    type SelfType<'a> = MyF64;
    type AsBytes<'a> = [u8; std::mem::size_of::<MyF64>()]
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        Some(std::mem::size_of::<MyF64>())
    }

    fn from_bytes<'a>(data: &'a [u8]) -> MyF64
    where
        Self: 'a,
    {
        let f = f64::from_le_bytes(data.try_into().unwrap());
        MyF64(f)
    }

    fn as_bytes<'a, 'b: 'a>(
        value: &'a Self::SelfType<'b>,
    ) -> [u8; std::mem::size_of::<MyF64>()]
    where
        Self: 'a,
        Self: 'b,
    {
        value.0.to_le_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new(stringify!(MyF64))
    }
}


impl Key for MyF64 {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        let f1 = Self::from_bytes(data1).0;
        let f2 = Self::from_bytes(data2).0;
        f1.total_cmp(&f2)
    }
}

pub fn open_table_write<'a, K: Key + 'static, V: Value + 'static>(table_name: &'a String, write_txn: &'a WriteTransaction) -> Table<'a, K, V> {
    let table_define: TableDefinition<'a, K, V> = TableDefinition::new(table_name.as_str());
    let table = write_txn.open_table(table_define).unwrap();
    table
}

pub fn open_table_read<'a, K: Key + 'static, V: Value + 'static>(table_name:&'a String, read_txn: &'a ReadTransaction) -> ReadOnlyTable<K, V> {
    let table_define: TableDefinition<'a, K, V> = TableDefinition::new(table_name.as_str());
    let table = read_txn.open_table(table_define).unwrap();
    table
}
