//! 窗口重叠解析器：纯物理函数 + Tauri 副作用执行（从 window_manager 拆出）
//!
//! 职责：
//! - `compute_overlaps`：纯函数，计算便签位置重叠的偏移结果（无 Tauri 依赖，可单测）
//! - `resolve_overlaps`：遍历偏移结果，执行 Tauri `set_position` 副作用
//!
//! 调用方：
//! - `window_manager::restore_all_windows`：启动恢复时调用 resolve_overlaps 防遮挡
//!
//! 依赖：
//! - `domain::{Note, value_objects::WindowState}`
//! - `tauri::AppHandle`（仅 resolve_overlaps）
//!
//! 设计要点：
//! - 物理计算与 Tauri 副作用分离，纯函数可直接单测
//! - 偏移量 = 重复序号 × 30px（x 和 y 同时偏移），形成层叠效果
//! - 仅移动窗口位置，不修改 DB 中的 window_state（下次启动仍会检测并偏移）

use std::collections::HashMap;

use tauri::{AppHandle, Manager};

use crate::domain::Note;

/// 计算便签位置重叠的偏移结果（纯函数，无 Tauri 依赖）
///
/// 对相同位置的便签按出现顺序级联偏移 30px（x 和 y 同时偏移）。
/// 第一个同位置便签不偏移，后续每个递增 30px。
///
/// 返回需要偏移的便签列表：(note_id, new_pos_x, new_pos_y)。
/// 不偏移的便签（首位）不在返回列表中。
pub fn compute_overlaps(notes: &[&Note]) -> Vec<(String, i32, i32)> {
    let mut seen_positions: HashMap<(i32, i32), usize> = HashMap::new();
    const OFFSET_PX: i32 = 30;
    let mut result = Vec::new();

    for note in notes {
        let key = (note.window_state.pos_x, note.window_state.pos_y);
        let dup_index = seen_positions.entry(key).or_insert(0);
        if *dup_index > 0 {
            let offset = (*dup_index as i32) * OFFSET_PX;
            result.push((
                note.id.clone(),
                note.window_state.pos_x + offset,
                note.window_state.pos_y + offset,
            ));
        }
        *dup_index += 1;
    }
    result
}

/// 检测位置重叠的便签窗口，对后续同位置便签级联偏移
///
/// 偏移量 = 重复序号 × 30px（x 和 y 同时偏移），形成层叠效果。
/// 仅移动窗口位置，不修改 DB 中的 window_state（下次启动仍会检测并偏移）。
///
/// 委托 `compute_overlaps` 计算偏移结果，再遍历执行 Tauri `set_position` 副作用。
pub fn resolve_overlaps(app: &AppHandle, notes: &[&Note]) {
    for (note_id, new_x, new_y) in compute_overlaps(notes) {
        let label = format!("note-{}", note_id);
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(new_x as f64, new_y as f64),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Note;
    use crate::domain::value_objects::WindowState;

    /// 构造测试用 Note（指定 id + 位置）
    fn make_note(id: &str, pos_x: i32, pos_y: i32) -> Note {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.id = id.to_string();
        note.window_state = WindowState {
            pos_x,
            pos_y,
            width: 320,
            height: 280,
        };
        note
    }

    #[test]
    fn test_compute_overlaps_no_overlap() {
        // 所有位置唯一 → 返回空 Vec
        let n1 = make_note("n1", 100, 100);
        let n2 = make_note("n2", 200, 200);
        let n3 = make_note("n3", 300, 300);
        let notes: Vec<&Note> = vec![&n1, &n2, &n3];

        let result = compute_overlaps(&notes);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_overlaps_two_same_position() {
        // 2 个同位置 → 第 2 个偏移 30px
        let n1 = make_note("n1", 100, 100);
        let n2 = make_note("n2", 100, 100);
        let notes: Vec<&Note> = vec![&n1, &n2];

        let result = compute_overlaps(&notes);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "n2");
        assert_eq!(result[0].1, 130); // 100 + 30
        assert_eq!(result[0].2, 130);
    }

    #[test]
    fn test_compute_overlaps_three_same_position() {
        // 3 个同位置 → 第 2 个偏移 30px，第 3 个偏移 60px
        let n1 = make_note("n1", 50, 50);
        let n2 = make_note("n2", 50, 50);
        let n3 = make_note("n3", 50, 50);
        let notes: Vec<&Note> = vec![&n1, &n2, &n3];

        let result = compute_overlaps(&notes);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "n2");
        assert_eq!(result[0].1, 80); // 50 + 30
        assert_eq!(result[1].0, "n3");
        assert_eq!(result[1].1, 110); // 50 + 60
    }

    #[test]
    fn test_compute_overlaps_multiple_groups() {
        // 多组不同位置的重叠 → 各组独立计算
        // 组 A: (100,100) 出现 2 次
        // 组 B: (200,200) 出现 3 次
        let n1 = make_note("n1", 100, 100);
        let n2 = make_note("n2", 200, 200);
        let n3 = make_note("n3", 100, 100); // 组 A 第 2 个 → +30
        let n4 = make_note("n4", 200, 200); // 组 B 第 2 个 → +30
        let n5 = make_note("n5", 200, 200); // 组 B 第 3 个 → +60
        let notes: Vec<&Note> = vec![&n1, &n2, &n3, &n4, &n5];

        let result = compute_overlaps(&notes);
        assert_eq!(result.len(), 3);
        // n3: 组 A 第 2 个 → (130, 130)
        assert_eq!(result[0].0, "n3");
        assert_eq!(result[0].1, 130);
        // n4: 组 B 第 2 个 → (230, 230)
        assert_eq!(result[1].0, "n4");
        assert_eq!(result[1].1, 230);
        // n5: 组 B 第 3 个 → (260, 260)
        assert_eq!(result[2].0, "n5");
        assert_eq!(result[2].1, 260);
    }

    #[test]
    fn test_compute_overlaps_empty_input() {
        // 空输入 → 空结果
        let notes: Vec<&Note> = vec![];
        let result = compute_overlaps(&notes);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_overlaps_single_note() {
        // 单个便签 → 无重叠，空结果
        let n1 = make_note("n1", 100, 100);
        let notes: Vec<&Note> = vec![&n1];
        let result = compute_overlaps(&notes);
        assert!(result.is_empty());
    }
}
