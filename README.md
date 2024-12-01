---

## **Minimongo 项目说明文档**

### **简介**
A pure Rust db for JSON, like mongo db
Minimongo 是一个轻量级的数据库库，使用 Rust 编写，设计目标是支持类似 MongoDB 的文档存储和查询操作，同时利用 Rust 的内存安全和性能优势。适合嵌入式环境或需要快速数据处理的小型应用。

---

### **特性**
- **JSON 文档存储**：支持存储和查询结构化数据。
- **轻量级实现**：无需依赖外部数据库，嵌入式使用。
- **Rust 编写**：具备高性能与内存安全性。
- **基础 CRUD 操作**：支持创建、读取、更新和删除操作。

---

### **安装**
将 Minimongo 添加到您的项目中，请在 `Cargo.toml` 中添加以下依赖项：
```toml
[dependencies]
minimongo = "0.1.0" # 替换为实际版本号
```

---

### **快速入门**
1. **初始化数据库**
   ```rust
   use minimongo::Database;

   fn main() {
       let mut db = Database::new();
       println!("数据库已初始化!");
   }
   ```

2. **插入文档**
   ```rust
   db.insert("users", json!({ "name": "Alice", "age": 30 }));
   ```

3. **查询数据**
   ```rust
   let results = db.find("users", json!({ "age": { "$gt": 25 } }));
   println!("{:?}", results);
   ```

---

## **API 文档**

### **核心类型**
- `Database`：数据库对象，用于管理集合和操作文档。
- `Collection`：表示一个集合，存储具体的 JSON 文档。
- `Document`：表示一个 JSON 对象，支持键值对存储。

### **主要方法**
#### `Database::new`
**功能**：创建一个新的数据库实例。  
**示例**：
```rust
let db = Database::new();
```

#### `Database::insert`
**功能**：向指定集合中插入一个文档。  
**参数**：
- `collection`: 集合名称。
- `document`: 插入的文档（JSON 格式）。
  **示例**：
```rust
db.insert("users", json!({ "name": "Alice" }));
```

#### `Database::find`
**功能**：查询集合中的文档，支持条件查询。  
**参数**：
- `collection`: 集合名称。
- `query`: 查询条件（JSON 格式）。
  **示例**：
```rust
let result = db.find("users", json!({ "age": { "$gt": 18 } }));
```

#### `Database::update`
**功能**：更新集合中的文档。  
**参数**：
- `collection`: 集合名称。
- `query`: 条件匹配文档。
- `update`: 更新内容。
  **示例**：
```rust
db.update("users", json!({ "name": "Alice" }), json!({ "age": 31 }));
```

#### `Database::delete`
**功能**：删除集合中的文档。  
**参数**：
- `collection`: 集合名称。
- `query`: 条件匹配文档。
  **示例**：
```rust
db.delete("users", json!({ "age": { "$lt": 20 } }));
```

---

### **贡献指南**
如果您希望参与开发或改进 Minimongo：
1. 克隆仓库：
   ```bash
   git clone https://github.com/wsds/minimongo.git
   ```
2. 提交 PR 或报告 Issue。

---

如果需要更多具体的代码文件分析，请告诉我进一步细化！