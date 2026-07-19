//! 图片文件存储服务：图片文件名提取、孤儿图片清理、图片目录管理。
//!
//! 设计意图：从 `commands` 中下沉的纯业务逻辑，无 Tauri 依赖，
//! 可被 `commands/image_commands` 和 `commands/note_commands` 共用。

use std::collections::HashSet;
use std::path::PathBuf;

/// 从内容中提取所有图片文件名
///
/// 匹配格式：`img:uuid.png`，支持 `img:filename{width=N}` 宽度参数语法。
/// `{width=N}` 不属于文件名，会被截断。
pub fn extract_image_filenames(content: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    // 查找所有 img:xxx.ext 模式
    for part in content.split("img:").skip(1) {
        // 取到第一个空白或 ) 或 ] 或 ( 或 { 为止
        let filename: String = part.chars()
            .take_while(|c| !c.is_whitespace() && *c != ')' && *c != ']' && *c != '(' && *c != '{')
            .collect();
        if !filename.is_empty() {
            let lower = filename.to_lowercase();
            if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
                || lower.ends_with(".gif") || lower.ends_with(".webp") || lower.ends_with(".bmp")
            {
                names.insert(filename);
            }
        }
    }
    names
}

/// 对比新旧内容，删除不再被引用的图片文件
///
/// 用于便签内容更新时清理孤儿图片。`new_content` 为空表示便签被删除。
pub fn cleanup_removed_images(old_content: &str, new_content: &str) {
    let old_images = extract_image_filenames(old_content);
    let new_images = extract_image_filenames(new_content);

    // 找出被移除的图片
    let removed: Vec<_> = old_images.difference(&new_images).collect();
    if removed.is_empty() {
        return;
    }

    if let Ok(dir) = image_dir() {
        for filename in removed {
            let filepath = dir.join(filename);
            if let Err(e) = std::fs::remove_file(&filepath) {
                eprintln!("[图片清理] 删除失败: {}, 文件: {:?}", e, filepath);
            } else {
                eprintln!("[图片清理] 已删除: {}", filename);
            }
        }
    }
}

/// 获取图片存储目录（exe 同级 data/sync/images/）
///
/// 目录不存在时会自动创建。
pub fn image_dir() -> Result<PathBuf, String> {
    let dir = std::env::current_exe()
        .map_err(|e| format!("获取 exe 路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .join("data")
        .join("sync")
        .join("images");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建图片目录失败: {}", e))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_filenames_basic() {
        let content = "![alt](img:abc.png) and img:def.jpg{width=300}";
        let names = extract_image_filenames(content);
        assert_eq!(names.len(), 2);
        assert!(names.contains("abc.png"));
        assert!(names.contains("def.jpg"));
    }

    #[test]
    fn test_extract_image_filenames_empty() {
        let names = extract_image_filenames("普通文本无图片");
        assert!(names.is_empty());
    }

    #[test]
    fn test_extract_image_filenames_unsupported_ext() {
        let names = extract_image_filenames("img:file.txt img:another.pdf");
        assert!(names.is_empty());
    }

    #[test]
    fn test_extract_image_filenames_case_insensitive() {
        let names = extract_image_filenames("img:PIC.PNG");
        assert!(names.contains("PIC.PNG"));
    }

    #[test]
    fn test_cleanup_removed_images_no_op_when_empty() {
        // 无图片内容时不应 panic
        cleanup_removed_images("", "");
    }
}
