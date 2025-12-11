//! 数据存储模块 - 展示Rust的并发安全机制
//!
//! Rust特点展示:
//! - Arc (原子引用计数) 实现多线程共享所有权
//! - RwLock (读写锁) 实现并发访问控制
//! - 生命周期和所有权
//! - Option类型处理可能为空的值

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 存储的值，包含数据和可选的过期时间
///
/// Rust特点: 结构体组合多个字段，Option表示可选值
#[derive(Debug, Clone)]
pub struct StoredValue {
    /// 实际数据
    data: Vec<u8>,
    /// 过期时间点 - None表示永不过期
    expires_at: Option<Instant>,
}

impl StoredValue {
    /// 创建新的存储值
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            expires_at: None,
        }
    }

    /// 创建带过期时间的存储值
    ///
    /// Rust特点: 方法链式调用，返回Self实现构建器模式
    pub fn with_expiry(mut self, ttl: Duration) -> Self {
        self.expires_at = Some(Instant::now() + ttl);
        self
    }

    /// 检查是否已过期
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => Instant::now() > expires_at,
            None => false,
        }
    }

    /// 获取数据的引用
    ///
    /// Rust特点: 返回引用避免不必要的复制
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// 获取剩余生存时间(毫秒)
    pub fn ttl_ms(&self) -> Option<i64> {
        self.expires_at.map(|expires_at| {
            let now = Instant::now();
            if now > expires_at {
                -1
            } else {
                (expires_at - now).as_millis() as i64
            }
        })
    }
}

/// 键值存储 - 线程安全的数据存储
///
/// Rust特点:
/// - Arc允许多个所有者共享数据
/// - RwLock允许多个读取者或单个写入者
/// - 类型系统在编译期保证线程安全
#[derive(Debug, Clone)]
pub struct Store {
    /// 内部存储
    ///
    /// Arc<RwLock<...>> 是Rust中实现线程安全共享状态的惯用方式
    inner: Arc<RwLock<HashMap<String, StoredValue>>>,
}

impl Store {
    /// 创建新的空存储
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置键值对
    ///
    /// Rust特点:
    /// - &self 表示不可变借用，但内部使用RwLock实现内部可变性
    /// - write() 获取写锁，保证独占访问
    pub fn set(&self, key: String, value: Vec<u8>) {
        let mut store = self.inner.write().unwrap();
        store.insert(key, StoredValue::new(value));
    }

    /// 设置键值对，带过期时间
    pub fn set_with_expiry(&self, key: String, value: Vec<u8>, ttl: Duration) {
        let mut store = self.inner.write().unwrap();
        store.insert(key, StoredValue::new(value).with_expiry(ttl));
    }

    /// 获取值
    ///
    /// Rust特点:
    /// - Option<Vec<u8>> 明确表示可能不存在
    /// - read() 获取读锁，允许并发读取
    /// - Clone用于返回数据的副本，避免生命周期问题
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let store = self.inner.read().unwrap();
        store.get(key).and_then(|v| {
            if v.is_expired() {
                None
            } else {
                Some(v.data().to_vec())
            }
        })
    }

    /// 删除键
    ///
    /// 返回是否成功删除
    pub fn del(&self, key: &str) -> bool {
        let mut store = self.inner.write().unwrap();
        store.remove(key).is_some()
    }

    /// 批量删除键
    ///
    /// Rust特点: 迭代器和闭包的组合使用
    pub fn del_multi(&self, keys: &[String]) -> usize {
        let mut store = self.inner.write().unwrap();
        keys.iter()
            .filter(|key| store.remove(*key).is_some())
            .count()
    }

    /// 检查键是否存在
    pub fn exists(&self, key: &str) -> bool {
        let store = self.inner.read().unwrap();
        store.get(key).map_or(false, |v| !v.is_expired())
    }

    /// 批量检查键是否存在
    pub fn exists_multi(&self, keys: &[String]) -> usize {
        let store = self.inner.read().unwrap();
        keys.iter()
            .filter(|key| {
                store
                    .get(*key)
                    .map_or(false, |v| !v.is_expired())
            })
            .count()
    }

    /// 获取所有键
    ///
    /// Rust特点: 迭代器链式调用，惰性求值
    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let store = self.inner.read().unwrap();
        store
            .iter()
            .filter(|(_, v)| !v.is_expired())
            .filter(|(k, _)| Self::match_pattern(k, pattern))
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// 简单的模式匹配 (* 匹配任意字符)
    fn match_pattern(key: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // 简化的glob匹配
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            return key.contains(middle);
        }

        if pattern.starts_with('*') {
            return key.ends_with(&pattern[1..]);
        }

        if pattern.ends_with('*') {
            return key.starts_with(&pattern[..pattern.len() - 1]);
        }

        key == pattern
    }

    /// 获取键的剩余生存时间(毫秒)
    pub fn pttl(&self, key: &str) -> i64 {
        let store = self.inner.read().unwrap();
        match store.get(key) {
            Some(v) => {
                if v.is_expired() {
                    -2 // 键不存在
                } else {
                    v.ttl_ms().unwrap_or(-1) // -1表示永不过期
                }
            }
            None => -2, // 键不存在
        }
    }

    /// 设置键的过期时间
    pub fn expire(&self, key: &str, ttl: Duration) -> bool {
        let mut store = self.inner.write().unwrap();
        if let Some(v) = store.get_mut(key) {
            if !v.is_expired() {
                v.expires_at = Some(Instant::now() + ttl);
                return true;
            }
        }
        false
    }

    /// 移除键的过期时间
    pub fn persist(&self, key: &str) -> bool {
        let mut store = self.inner.write().unwrap();
        if let Some(v) = store.get_mut(key) {
            if v.expires_at.is_some() {
                v.expires_at = None;
                return true;
            }
        }
        false
    }

    /// 原子递增
    ///
    /// Rust特点: Result类型表示可能失败的操作
    pub fn incr(&self, key: &str, delta: i64) -> Result<i64, String> {
        let mut store = self.inner.write().unwrap();

        let current = store.get(key).and_then(|v| {
            if v.is_expired() {
                None
            } else {
                Some(v.data().to_vec())
            }
        });

        let value = match current {
            Some(data) => {
                let s = String::from_utf8(data)
                    .map_err(|_| "值不是有效的UTF-8字符串")?;
                let num: i64 = s.parse().map_err(|_| "值不是整数")?;
                num + delta
            }
            None => delta,
        };

        store.insert(
            key.to_string(),
            StoredValue::new(value.to_string().into_bytes()),
        );

        Ok(value)
    }

    /// 追加字符串
    pub fn append(&self, key: &str, value: &[u8]) -> usize {
        let mut store = self.inner.write().unwrap();

        let entry = store.entry(key.to_string()).or_insert_with(|| {
            StoredValue::new(Vec::new())
        });

        if entry.is_expired() {
            *entry = StoredValue::new(value.to_vec());
            value.len()
        } else {
            let mut data = entry.data.clone();
            data.extend_from_slice(value);
            let len = data.len();
            entry.data = data;
            len
        }
    }

    /// 获取字符串长度
    pub fn strlen(&self, key: &str) -> usize {
        let store = self.inner.read().unwrap();
        store
            .get(key)
            .filter(|v| !v.is_expired())
            .map_or(0, |v| v.data().len())
    }

    /// 清理过期的键
    ///
    /// Rust特点: retain方法实现原地过滤
    pub fn cleanup_expired(&self) -> usize {
        let mut store = self.inner.write().unwrap();
        let before = store.len();
        store.retain(|_, v| !v.is_expired());
        before - store.len()
    }

    /// 获取数据库大小(键的数量)
    pub fn dbsize(&self) -> usize {
        let store = self.inner.read().unwrap();
        store.iter().filter(|(_, v)| !v.is_expired()).count()
    }

    /// 清空所有数据
    pub fn flushdb(&self) {
        let mut store = self.inner.write().unwrap();
        store.clear();
    }

    /// 获取键的类型
    pub fn key_type(&self, key: &str) -> Option<&'static str> {
        let store = self.inner.read().unwrap();
        store.get(key).and_then(|v| {
            if v.is_expired() {
                None
            } else {
                Some("string") // 目前只支持字符串类型
            }
        })
    }

    /// 重命名键
    pub fn rename(&self, old_key: &str, new_key: &str) -> bool {
        let mut store = self.inner.write().unwrap();
        if let Some(value) = store.remove(old_key) {
            if !value.is_expired() {
                store.insert(new_key.to_string(), value);
                return true;
            }
        }
        false
    }
}

/// 实现Default trait
///
/// Rust特点: 使用派生或手动实现标准trait
impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let store = Store::new();
        store.set("key".to_string(), b"value".to_vec());
        assert_eq!(store.get("key"), Some(b"value".to_vec()));
    }

    #[test]
    fn test_del() {
        let store = Store::new();
        store.set("key".to_string(), b"value".to_vec());
        assert!(store.del("key"));
        assert_eq!(store.get("key"), None);
    }

    #[test]
    fn test_exists() {
        let store = Store::new();
        assert!(!store.exists("key"));
        store.set("key".to_string(), b"value".to_vec());
        assert!(store.exists("key"));
    }

    #[test]
    fn test_incr() {
        let store = Store::new();
        assert_eq!(store.incr("counter", 1), Ok(1));
        assert_eq!(store.incr("counter", 5), Ok(6));
        assert_eq!(store.incr("counter", -2), Ok(4));
    }

    #[test]
    fn test_expiry() {
        let store = Store::new();
        store.set_with_expiry(
            "key".to_string(),
            b"value".to_vec(),
            Duration::from_millis(100),
        );
        assert!(store.exists("key"));

        // 等待过期
        std::thread::sleep(Duration::from_millis(150));
        assert!(!store.exists("key"));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(Store::match_pattern("hello", "*"));
        assert!(Store::match_pattern("hello", "hel*"));
        assert!(Store::match_pattern("hello", "*llo"));
        assert!(Store::match_pattern("hello", "*ell*"));
        assert!(!Store::match_pattern("hello", "world"));
    }
}

