# 13 - 错误处理

> 返回 [DESIGN.md](../DESIGN.md)

## Rust 后端

```rust
enum AppError {
    DatabaseError(String),       // SQL 错误
    ValidationError(String),     // 数据校验失败
    NotFound(String),            // 资源不存在
    Unauthorized,                // 密码错误/未解锁
    BalanceError(String),        // 复式记账不平衡
    ClosedAccountError,          // 向已关闭账户记账
    IoError(String),             // 文件读写错误
}

// 所有 Command 返回 Result<T, String>
// 错误信息面向用户友好，日志中保留详细信息
```

## 前端

- API 调用失败时显示 Toast 通知（shadcn Toast）
- 表单校验错误内联显示在对应字段下方
- 关键操作（删除交易、关闭账户）需要二次确认对话框
- 数据库锁定状态自动检测，未解锁时跳转至解锁页
