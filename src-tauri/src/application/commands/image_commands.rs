//! 图片存储命令：保存图片、获取图片目录路径。

use super::super::image_service;

/// 保存图片文件，返回文件名（如 "uuid.png"）
#[tauri::command]
pub fn save_image(data: Vec<u8>, ext: String) -> Result<String, String> {
    let allowed = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];
    if !allowed.contains(&ext.as_str()) {
        return Err(format!("不支持的图片格式: {}", ext));
    }
    let dir = image_service::image_dir()?;
    let id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{}.{}", id, ext);
    std::fs::write(dir.join(&filename), &data).map_err(|e| format!("保存图片失败: {}", e))?;
    Ok(filename)
}

/// 获取图片目录完整路径，前端用于拼接 convertFileSrc
#[tauri::command]
pub fn get_image_dir() -> Result<String, String> {
    let dir = image_service::image_dir()?;
    dir.to_str()
        .map(|s| s.to_string())
        .ok_or("路径转换失败".to_string())
}
