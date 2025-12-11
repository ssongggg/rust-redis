# Rust Redis

一个用 Rust 实现的简单 Redis 服务器，展示 Rust 语言的核心特性。

## 🎯 项目目标

通过实现一个简单但功能完整的 Redis 服务器来学习和展示 Rust 语言的特点。

## 🦀 Rust 语言特点展示

### 1. 所有权系统 (Ownership)
```rust
// 所有权转移 - TcpStream的所有权从调用者转移到Connection
pub fn new(stream: TcpStream) -> Self {
    Self { stream, ... }
}
```

### 2. 借用和引用 (Borrowing & References)
```rust
// &str 是不可变借用，避免数据复制
pub fn get(&self, key: &str) -> Option<Vec<u8>> { ... }

// &mut 是可变借用，允许修改数据
pub fn parse(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> { ... }
```

### 3. 枚举和模式匹配 (Enum & Pattern Matching)
```rust
// 枚举表示不同的RESP数据类型
pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Vec<u8>),
    Null,
    Array(Vec<RespValue>),
}

// match 必须穷尽所有情况
match value {
    RespValue::SimpleString(s) => ...,
    RespValue::Integer(i) => ...,
    // 编译器确保处理所有变体
}
```

### 4. 错误处理 (Error Handling)
```rust
// Result类型表示可能失败的操作
pub fn incr(&self, key: &str, delta: i64) -> Result<i64, String> { ... }

// ? 操作符简化错误传播
let value = self.store.get(&key)?;
```

### 5. Trait 系统
```rust
// 使用 thiserror 派生 Error trait
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("IO错误: {0}")]
    Io(#[from] io::Error),
    ...
}
```

### 6. 并发安全
```rust
// Arc<RwLock<...>> 实现线程安全的共享状态
pub struct Store {
    inner: Arc<RwLock<HashMap<String, StoredValue>>>,
}

// 多个连接可以安全地共享 Store
let store = Store::new();
let store_clone = store.clone(); // Arc 的克隆只增加引用计数
```

### 7. 异步编程 (Async/Await)
```rust
// async fn 定义异步函数
pub async fn handle(&mut self, store: &Store) -> RedisResult<()> {
    loop {
        let command = self.read_command().await?;  // .await 等待异步操作
        ...
    }
}

// tokio::spawn 创建并发任务
tokio::spawn(async move {
    connection.handle(&store).await;
});
```

### 8. 生命周期 (Lifetimes)
```rust
// 'a 标注确保执行器不会比 store 活得更久
pub struct CommandExecutor<'a> {
    store: &'a Store,
}
```

## 📦 支持的命令

### 连接命令
- `PING [message]` - 测试连接
- `ECHO message` - 回显消息
- `QUIT` - 关闭连接

### 字符串命令
- `GET key` - 获取值
- `SET key value [EX seconds] [PX milliseconds] [NX|XX]` - 设置值
- `GETSET key value` - 设置新值并返回旧值
- `APPEND key value` - 追加字符串
- `STRLEN key` - 获取字符串长度
- `INCR key` / `INCRBY key increment` - 递增
- `DECR key` / `DECRBY key decrement` - 递减
- `MGET key [key ...]` - 批量获取
- `MSET key value [key value ...]` - 批量设置

### 键命令
- `DEL key [key ...]` - 删除键
- `EXISTS key [key ...]` - 检查键是否存在
- `EXPIRE key seconds` / `PEXPIRE key milliseconds` - 设置过期时间
- `TTL key` / `PTTL key` - 获取剩余生存时间
- `PERSIST key` - 移除过期时间
- `KEYS pattern` - 查找键
- `TYPE key` - 获取键类型
- `RENAME old new` - 重命名键

### 服务器命令
- `DBSIZE` - 获取键数量
- `FLUSHDB` - 清空数据库
- `INFO` - 获取服务器信息

## 🚀 快速开始

### 编译
```bash
cargo build --release
```

### 启动服务器
```bash
# 使用默认端口 6379
cargo run --bin redis-server

# 指定端口
cargo run --bin redis-server -- 6380
```

### 启动客户端
```bash
# 连接本地默认端口
cargo run --bin redis-client

# 连接指定地址和端口
cargo run --bin redis-client -- 127.0.0.1 6379
```

### 使用 redis-cli 测试
```bash
redis-cli -p 6379

127.0.0.1:6379> PING
PONG
127.0.0.1:6379> SET name "Rust Redis"
OK
127.0.0.1:6379> GET name
"Rust Redis"
127.0.0.1:6379> SET counter 0
OK
127.0.0.1:6379> INCR counter
(integer) 1
127.0.0.1:6379> INCRBY counter 10
(integer) 11
```

### 使用 telnet 测试
```bash
telnet localhost 6379
PING
+PONG
SET foo bar
+OK
GET foo
$3
bar
```

## 🧪 运行测试
```bash
cargo test
```

## 📁 项目结构

```
rust-redis/
├── Cargo.toml           # 项目配置
├── README.md            # 说明文档
└── src/
    ├── lib.rs           # 库入口
    ├── main.rs          # 服务器入口
    ├── client.rs        # 客户端入口
    ├── error.rs         # 错误处理
    ├── resp.rs          # RESP协议解析
    ├── store.rs         # 数据存储
    ├── command.rs       # 命令处理
    └── connection.rs    # 连接处理
```

## 📝 技术细节

### RESP 协议
Redis 使用 RESP (REdis Serialization Protocol) 进行客户端-服务器通信：

- 简单字符串: `+OK\r\n`
- 错误: `-ERR message\r\n`
- 整数: `:1000\r\n`
- 批量字符串: `$6\r\nfoobar\r\n`
- 数组: `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`

### 并发模型
- 使用 Tokio 异步运行时
- 每个客户端连接一个异步任务
- 使用 `Arc<RwLock<>>` 共享数据存储
- 后台任务定期清理过期键

## 📜 许可证

MIT License

