use super::{Note, Reminder};

/// Note 仓储接口（领域层定义契约，基础设施层实现）
///
/// 遵循依赖倒置原则：领域层定义接口，基础设施层实现。
/// 替换 SQLite 为其他数据库只需实现此 trait。
pub trait NoteRepository: Send + Sync {
    /// 保存便签（新增或更新）
    fn save(&self, note: &Note) -> Result<(), String>;

    /// 根据 ID 查找便签
    fn find_by_id(&self, id: &str) -> Result<Option<Note>, String>;

    /// 查找所有便签
    fn find_all(&self) -> Result<Vec<Note>, String>;

    /// 删除便签
    fn delete(&self, id: &str) -> Result<(), String>;

    /// 查找已归档的便签
    fn find_archived(&self) -> Result<Vec<Note>, String>;

    /// 搜索便签（标题 + 内容 + 标签，跨活跃和归档）
    fn search_notes(&self, query: &str) -> Result<Vec<Note>, String>;

    /// 查询指定月份内有创建或更新活动的日期集合（日历视图用）
    fn find_activity_by_month(&self, year: i32, month: u32) -> Result<Vec<u32>, String>;
}

/// Reminder 仓储接口
pub trait ReminderRepository: Send + Sync {
    /// 保存提醒
    fn save(&self, reminder: &Reminder) -> Result<(), String>;

    /// 根据 ID 查找提醒
    fn find_by_id(&self, id: &str) -> Result<Option<Reminder>, String>;

    /// 查找全部提醒（用于同步导出）
    fn find_all(&self) -> Result<Vec<Reminder>, String>;

    /// 查找到期的提醒
    fn find_due(&self, now: &str) -> Result<Vec<Reminder>, String>;

    /// 根据便签 ID 查找提醒
    fn find_by_note_id(&self, note_id: &str) -> Result<Vec<Reminder>, String>;

    /// 删除提醒
    fn delete(&self, id: &str) -> Result<(), String>;

    /// 删除便签的所有提醒
    fn delete_by_note_id(&self, note_id: &str) -> Result<(), String>;

    /// 查询最近一条到期提醒的时间（pending 状态）
    fn find_next_due_time(&self) -> Result<Option<String>, String>;

    /// 查询指定时间范围内的提醒（日历视图用，含所有状态）
    fn find_by_date_range(&self, start: &str, end: &str) -> Result<Vec<Reminder>, String>;
}
