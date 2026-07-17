use super::ChatMessage;

/// 文本重写操作类型
///
/// 对应右键菜单的 AI 文本重写能力，每项映射一种处理风格。
pub enum RewriteOperation {
    /// 规整文本（口语→书面）
    Tidy,
    /// 转为待办清单
    TodoSplit,
    /// 更正式
    StyleFormal,
    /// 更精简
    StyleConcise,
    /// 更温和
    StyleMild,
}

impl RewriteOperation {
    /// 从字符串解析操作类型
    ///
    /// 支持的取值：`tidy` / `todo_split` / `style_formal` / `style_concise` / `style_mild`。
    /// 无效输入返回 `None`。
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "tidy" => Some(Self::Tidy),
            "todo_split" => Some(Self::TodoSplit),
            "style_formal" => Some(Self::StyleFormal),
            "style_concise" => Some(Self::StyleConcise),
            "style_mild" => Some(Self::StyleMild),
            _ => None,
        }
    }

    /// 返回该操作对应的指令描述（注入到 system 提示中）
    pub fn instruction(&self) -> &str {
        match self {
            Self::Tidy => "将以下文本规整为书面表达，去除口语化词汇",
            Self::TodoSplit => "将以下文本拆分为待办清单，每项以 '- [ ] ' 开头",
            Self::StyleFormal => "将以下文本改为更正式的书面表达，添加敬语",
            Self::StyleConcise => "精简以下文本，只保留核心信息，压缩到原字数的60%",
            Self::StyleMild => "将以下文本改为更温和的表达，'必须'→'建议'，'错了'→'可优化'",
        }
    }
}

/// 构造文本重写消息列表（system + user）
///
/// system 提示注入操作指令，要求 AI 仅返回处理后的文本；
/// user 消息为待处理文本。
pub fn build_rewrite_messages(text: &str, operation: RewriteOperation) -> Vec<ChatMessage> {
    let system_prompt = format!(
        "你是文本编辑助手。{}。仅返回处理后的文本，不附加解释。",
        operation.instruction()
    );
    vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(text),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rewrite_messages_returns_system_and_user() {
        let messages = build_rewrite_messages("明天开会", RewriteOperation::Tidy);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "明天开会");

        let system_content = &messages[0].content;
        assert!(system_content.contains("文本编辑助手"), "system 提示应包含角色定位");
        assert!(
            system_content.contains(RewriteOperation::Tidy.instruction()),
            "system 提示应包含操作指令"
        );
        assert!(system_content.contains("仅返回处理后的文本"), "system 提示应包含仅返回文本要求");
    }

    #[test]
    fn test_rewrite_operation_from_str() {
        assert!(matches!(RewriteOperation::from_str("tidy"), Some(RewriteOperation::Tidy)));
        assert!(matches!(
            RewriteOperation::from_str("todo_split"),
            Some(RewriteOperation::TodoSplit)
        ));
        assert!(matches!(
            RewriteOperation::from_str("style_formal"),
            Some(RewriteOperation::StyleFormal)
        ));
        assert!(matches!(
            RewriteOperation::from_str("style_concise"),
            Some(RewriteOperation::StyleConcise)
        ));
        assert!(matches!(
            RewriteOperation::from_str("style_mild"),
            Some(RewriteOperation::StyleMild)
        ));
        assert!(RewriteOperation::from_str("unknown").is_none(), "未知字符串应返回 None");
        assert!(RewriteOperation::from_str("").is_none(), "空字符串应返回 None");
    }

    #[test]
    fn test_rewrite_operation_instruction() {
        assert_eq!(
            RewriteOperation::Tidy.instruction(),
            "将以下文本规整为书面表达，去除口语化词汇"
        );
        assert_eq!(
            RewriteOperation::TodoSplit.instruction(),
            "将以下文本拆分为待办清单，每项以 '- [ ] ' 开头"
        );
        assert_eq!(
            RewriteOperation::StyleFormal.instruction(),
            "将以下文本改为更正式的书面表达，添加敬语"
        );
        assert_eq!(
            RewriteOperation::StyleConcise.instruction(),
            "精简以下文本，只保留核心信息，压缩到原字数的60%"
        );
        assert_eq!(
            RewriteOperation::StyleMild.instruction(),
            "将以下文本改为更温和的表达，'必须'→'建议'，'错了'→'可优化'"
        );
    }
}
