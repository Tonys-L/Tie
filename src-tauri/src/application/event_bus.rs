//! 后端内部事件总线（ADR-007）
//!
//! 职责：传递写操作完成信号（DomainEvent），解耦 service 层与副作用（schedule_auto_sync 等）
//!
//! 调用方：
//! - `note_service`/`reminder_service`/`template_service`：写方法接收 `&dyn EventPublisher`，
//!   完成业务后 emit `DomainEvent`
//! - `lib.rs` setup：注册监听器，接收事件并触发 `schedule_auto_sync`
//!
//! 依赖：无（纯 Rust，不依赖 Tauri / tokio）
//!
//! 设计要点：
//! - `EventPublisher` trait 是切换 seam（依赖倒置）
//! - 当前同步实现 `EventBus`，未来可替换为 `ChannelPublisher`（tokio::sync::broadcast），
//!   service 签名零改动

use std::sync::{Arc, Mutex};

/// 写操作类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteAction {
    Created,
    Updated,
    Deleted,
}

/// 领域事件：写操作完成信号
///
/// 按实体 + 操作类型粒度。payload 携带实体 id，监听器可按需决定副作用。
#[derive(Clone, Debug)]
pub enum DomainEvent {
    NoteWritten { action: WriteAction, id: String },
    ReminderWritten { action: WriteAction, id: String },
    TemplateWritten { action: WriteAction, id: String },
}

/// 事件发布者抽象（依赖倒置 seam）
///
/// service 层依赖此 trait 而非具体类型，便于测试 mock 和未来替换实现。
pub trait EventPublisher: Send + Sync {
    fn emit(&self, event: DomainEvent);
}

/// 同步事件总线（当前实现）
///
/// 内部用 `Arc<Mutex<Vec<...>>>` 存储 handler 列表。
/// `emit` 同步遍历调用所有 handler；`subscribe` 注册新 handler。
///
/// 未来若需异步：新建 `ChannelPublisher` impl `EventPublisher`（内部用 tokio::sync::broadcast），
/// service 签名零改动，迁移成本集中在 event_bus.rs + lib.rs。
pub struct EventBus {
    handlers: Arc<Mutex<Vec<Box<dyn Fn(&DomainEvent) + Send + Sync>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 注册监听器
    pub fn subscribe(&self, handler: Box<dyn Fn(&DomainEvent) + Send + Sync>) {
        let mut handlers = self.handlers.lock().expect("EventBus handlers poisoned");
        handlers.push(handler);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPublisher for EventBus {
    fn emit(&self, event: DomainEvent) {
        let handlers = self.handlers.lock().expect("EventBus handlers poisoned");
        for handler in handlers.iter() {
            handler(&event);
        }
    }
}

/// 测试用 mock publisher：记录所有 emit 调用，便于 service 单测断言
///
/// 仅在 `cfg(test)` 下编译，service 单测通过 `&dyn EventPublisher` 传入。
#[cfg(test)]
pub struct MockEventPublisher {
    pub events: Arc<Mutex<Vec<DomainEvent>>>,
}

#[cfg(test)]
impl MockEventPublisher {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn events_clone(&self) -> Arc<Mutex<Vec<DomainEvent>>> {
        self.events.clone()
    }
}

#[cfg(test)]
impl Default for MockEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl EventPublisher for MockEventPublisher {
    fn emit(&self, event: DomainEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_emit_with_no_subscribers_does_not_panic() {
        let bus = EventBus::new();
        bus.emit(DomainEvent::NoteWritten {
            action: WriteAction::Created,
            id: "note-1".to_string(),
        });
    }

    #[test]
    fn test_subscribe_receives_emitted_events() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        bus.subscribe(Box::new(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        bus.emit(DomainEvent::NoteWritten {
            action: WriteAction::Created,
            id: "note-1".to_string(),
        });
        bus.emit(DomainEvent::ReminderWritten {
            action: WriteAction::Updated,
            id: "rem-1".to_string(),
        });

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_multiple_subscribers_all_receive_event() {
        let bus = EventBus::new();
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let c1_clone = c1.clone();
        let c2_clone = c2.clone();
        bus.subscribe(Box::new(move |_| {
            c1_clone.fetch_add(1, Ordering::SeqCst);
        }));
        bus.subscribe(Box::new(move |_| {
            c2_clone.fetch_add(1, Ordering::SeqCst);
        }));

        bus.emit(DomainEvent::TemplateWritten {
            action: WriteAction::Deleted,
            id: "tpl-1".to_string(),
        });

        assert_eq!(c1.load(Ordering::SeqCst), 1);
        assert_eq!(c2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_payload_preserved() {
        let bus = EventBus::new();
        let captured_id = Arc::new(Mutex::new(String::new()));
        let captured_action = Arc::new(Mutex::new(WriteAction::Created));
        let id_clone = captured_id.clone();
        let action_clone = captured_action.clone();
        bus.subscribe(Box::new(move |event| {
            if let DomainEvent::NoteWritten { action, id } = event {
                *id_clone.lock().unwrap() = id.clone();
                *action_clone.lock().unwrap() = action.clone();
            }
        }));

        bus.emit(DomainEvent::NoteWritten {
            action: WriteAction::Updated,
            id: "note-42".to_string(),
        });

        assert_eq!(*captured_id.lock().unwrap(), "note-42");
        assert_eq!(*captured_action.lock().unwrap(), WriteAction::Updated);
    }

    #[test]
    fn test_eventbus_as_trait_object() {
        let bus = EventBus::new();
        let publisher: &dyn EventPublisher = &bus;
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        bus.subscribe(Box::new(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        publisher.emit(DomainEvent::NoteWritten {
            action: WriteAction::Created,
            id: "note-1".to_string(),
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_mock_publisher_records_events() {
        let mock = MockEventPublisher::new();
        let publisher: &dyn EventPublisher = &mock;

        publisher.emit(DomainEvent::NoteWritten {
            action: WriteAction::Created,
            id: "n1".to_string(),
        });
        publisher.emit(DomainEvent::ReminderWritten {
            action: WriteAction::Deleted,
            id: "r1".to_string(),
        });

        let events = mock.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        match &events[0] {
            DomainEvent::NoteWritten { action, id } => {
                assert_eq!(*action, WriteAction::Created);
                assert_eq!(id, "n1");
            }
            _ => panic!("expected NoteWritten"),
        }
        match &events[1] {
            DomainEvent::ReminderWritten { action, id } => {
                assert_eq!(*action, WriteAction::Deleted);
                assert_eq!(id, "r1");
            }
            _ => panic!("expected ReminderWritten"),
        }
    }
}
