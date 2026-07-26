//! 应用层数据目录路径解析（单一所有者）。
//!
//! 设计意图：将 `data_dir_path` 从 `commands/system_commands` 提升到 application 层，
//! 消除 service 层（如 image_service）反向依赖 commands 层的违规。
//! commands 层和 service 层都正向依赖本模块。
//!
//! 调用方：lib.rs setup、commands/system_commands、application/image_service。

use std::path::PathBuf;

/// 数据目录路径解析（exe 同级目录/data）
///
/// 统一路径解析逻辑，供 commands 和 service 共用，避免多处重复解析导致路径规则漂移。
///
/// 注意：不做 `canonicalize()`，与原始内联代码行为一致。`current_exe()` 已返回
/// 绝对路径，`parent().join("data")` 保持绝对性；且本函数在 lib.rs setup 的
/// `create_dir_all` 之前调用，canonicalize 要求路径存在会失败。
pub fn data_dir_path() -> Result<PathBuf, String> {
    Ok(std::env::current_exe()
        .map_err(|e| format!("获取 exe 路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .join("data"))
}
