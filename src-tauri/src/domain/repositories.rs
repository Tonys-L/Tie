use super::{Note, Reminder, Template};

/// Note 仓储接口（聚合 CRUD：领域层定义契约，基础设施层实现）
///
/// 仅承载聚合根的标识性 CRUD（save/find_by_id/find_all/delete）+ 简单状态过滤（find_archived）。
/// 跨聚合投影/搜索/统计查询见 [`NoteQuery`]（CQRS 风味拆分，ADR-010）。
///
/// 遵循依赖倒置原则：领域层定义接口，基础设施层实现。
/// 替换 SQLite 为其他数据库只需实现此 trait + NoteQuery。
pub trait NoteRepository: Send + Sync {
    /// 保存便签（新增或更新）
    fn save(&self, note: &Note) -> Result<(), String>;

    /// 根据 ID 查找便签（默认过滤墓碑，业务逻辑用）
    fn find_by_id(&self, id: &str) -> Result<Option<Note>, String>;

    /// 根据 ID 查找便签（含墓碑，供 sync import 仲裁用，INV-032）
    ///
    /// 与 find_by_id 的区别：墓碑（deleted_at IS NOT NULL）也会返回。
    /// 仅供 sync_json_io::import_from_json 使用，让墓碑参与 last-write-wins 仲裁。
    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Note>, String>;

    /// 查找所有便签（默认过滤墓碑，业务逻辑用）
    fn find_all(&self) -> Result<Vec<Note>, String>;

    /// 查找所有便签（含墓碑，供 sync export 写出墓碑 JSON 用，INV-032）
    ///
    /// 仅供 sync_json_io::export_to_json 使用，让墓碑 JSON 写出到 sync 目录，
    /// 其他设备 import 时能传播删除。
    fn find_all_including_deleted(&self) -> Result<Vec<Note>, String>;

    /// 删除便签（硬删除，仅供墓碑清理使用，INV-032）
    ///
    /// 正常业务删除走 domain Note::delete() + save 软删除路径。
    /// 此方法执行 DELETE FROM，仅供 sync_tombstone_cleanup 调用清理超阈值墓碑。
    fn physical_delete(&self, id: &str) -> Result<(), String>;

    /// 删除便签（保留向后兼容，新代码应使用 Note::delete() + save 软删除）
    ///
    /// 注：此方法保留为硬删除语义，但 service 层已不再调用，
    /// 改为 domain Note::delete() + save 软删除。保留是为了 future-proof
    /// （如未来需要清理非墓碑数据的场景）。
    fn delete(&self, id: &str) -> Result<(), String>;

    /// 查找已归档的便签
    fn find_archived(&self) -> Result<Vec<Note>, String>;
}

/// Note 读投影查询接口（CQRS 风味拆分：FTS5 搜索 + 日历投影）
///
/// 与 [`NoteRepository`] 分离的原因：
/// - 这些方法不属于聚合根的标识性 CRUD，而是 UI / 报表读模型
/// - 测试 NoteRepository 写逻辑时无需 stub 这些方法（mock surface 缩小）
/// - 为未来独立读模型优化（如缓存/只读副本）留路径
pub trait NoteQuery: Send + Sync {
    /// 搜索便签（标题 + 内容 + 标签，跨活跃和归档）
    ///
    /// 回退规则契约（INV-021）：
    /// - 查询字符数 < 3：回退 LIKE 模糊匹配（trigram tokenizer 要求至少 3 字符）
    /// - 查询字符数 ≥ 3：FTS5 MATCH + snippet 高亮（`<mark>` 标签）
    /// - 结果排序：置顶优先 + 相关度
    /// - highlight 字段：FTS5 路径填充 snippet（title > content > tags 优先级），
    ///   LIKE 路径不填充（为 None）
    fn search_notes(&self, query: &str) -> Result<Vec<Note>, String>;

    /// 查询指定月份内有创建或更新活动的日期集合（日历视图用）
    fn find_activity_by_month(&self, year: i32, month: u32) -> Result<Vec<u32>, String>;
}

/// Template 仓储接口（用户自定义便签模板）
///
/// 方法较少（4 个），不拆分 Query trait（YAGNI）。
pub trait TemplateRepository: Send + Sync {
    /// 保存模板（新增或更新）
    fn save(&self, template: &Template) -> Result<(), String>;

    /// 查找所有模板（按 sort_order 排序，默认过滤墓碑）
    fn find_all(&self) -> Result<Vec<Template>, String>;

    /// 查找所有模板（含墓碑，供 sync export 用，INV-032）
    fn find_all_including_deleted(&self) -> Result<Vec<Template>, String>;

    /// 根据 ID 查找模板（默认过滤墓碑）
    fn find_by_id(&self, id: &str) -> Result<Option<Template>, String>;

    /// 根据 ID 查找模板（含墓碑，供 sync import 仲裁用，INV-032）
    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Template>, String>;

    /// 物理删除模板（仅供墓碑清理使用，INV-032）
    fn physical_delete(&self, id: &str) -> Result<(), String>;

    /// 删除模板（保留向后兼容，新代码应使用 Template::delete() + save 软删除）
    fn delete(&self, id: &str) -> Result<(), String>;
}

/// Reminder 仓储接口（聚合 CRUD：领域层定义契约，基础设施层实现）
///
/// 仅承载聚合根的标识性 CRUD + 按聚合外键查询（find_by_note_id）。
/// scheduler / 日历视图的到期查询和时间范围查询见 [`ReminderQuery`]（CQRS 风味拆分，ADR-010）。
pub trait ReminderRepository: Send + Sync {
    /// 保存提醒
    fn save(&self, reminder: &Reminder) -> Result<(), String>;

    /// 根据 ID 查找提醒（默认过滤墓碑）
    fn find_by_id(&self, id: &str) -> Result<Option<Reminder>, String>;

    /// 根据 ID 查找提醒（含墓碑，供 sync import 仲裁用，INV-032）
    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Reminder>, String>;

    /// 查找全部提醒（默认过滤墓碑，用于业务查询）
    fn find_all(&self) -> Result<Vec<Reminder>, String>;

    /// 查找全部提醒（含墓碑，供 sync export 用，INV-032）
    fn find_all_including_deleted(&self) -> Result<Vec<Reminder>, String>;

    /// 根据便签 ID 查找提醒（默认过滤墓碑）
    fn find_by_note_id(&self, note_id: &str) -> Result<Vec<Reminder>, String>;

    /// 物理删除提醒（仅供墓碑清理使用，INV-032）
    fn physical_delete(&self, id: &str) -> Result<(), String>;

    /// 删除提醒（保留向后兼容，新代码应使用 Reminder::delete() + save 软删除）
    fn delete(&self, id: &str) -> Result<(), String>;

    /// 删除便签的所有提醒（保留向后兼容，新代码应逐个 Reminder::delete() + save 软删除）
    fn delete_by_note_id(&self, note_id: &str) -> Result<(), String>;
}

/// Reminder 读投影查询接口（CQRS 风味拆分：scheduler + 日历视图）
///
/// 与 [`ReminderRepository`] 分离的原因：
/// - find_due / find_next_due_time 是 scheduler 关注点，不属于聚合根 CRUD
/// - find_by_date_range 是日历视图读模型
/// - 测试 service 层写逻辑时无需 stub 这些方法
pub trait ReminderQuery: Send + Sync {
    /// 查找到期的提醒
    fn find_due(&self, now: &str) -> Result<Vec<Reminder>, String>;

    /// 查询最近一条到期提醒的时间（pending 状态）
    fn find_next_due_time(&self) -> Result<Option<String>, String>;

    /// 查询指定时间范围内的提醒（日历视图用，含所有状态）
    fn find_by_date_range(&self, start: &str, end: &str) -> Result<Vec<Reminder>, String>;
}
