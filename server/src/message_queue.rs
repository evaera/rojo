use std::{
    collections::HashMap,
    sync::{
        mpsc,
        atomic::{AtomicUsize, Ordering},
        RwLock,
        Mutex,
    },
};

/// A unique identifier, not guaranteed to be generated in any order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(usize);

/// Generate a new ID, which has no defined ordering.
pub fn get_listener_id() -> ListenerId {
    static LAST_ID: AtomicUsize = AtomicUsize::new(0);

    ListenerId(LAST_ID.fetch_add(1, Ordering::SeqCst))
}

/// A message queue with persistent history that can be subscribed to.
///
/// Definitely non-optimal, but a simple design that works well for the
/// synchronous web server Rojo uses, Rouille.
#[derive(Default)]
pub struct MessageQueue<T> {
   messages: RwLock<Vec<T>>,
   message_listeners: Mutex<HashMap<ListenerId, mpsc::Sender<()>>>,
}

impl<T: Clone> MessageQueue<T> {
    pub fn new() -> MessageQueue<T> {
        MessageQueue {
            messages: RwLock::new(Vec::new()),
            message_listeners: Mutex::new(HashMap::new()),
        }
    }

    pub fn push_messages(&self, new_messages: &[T]) {
        let message_listeners = self.message_listeners.lock().unwrap();

        {
            let mut messages = self.messages.write().unwrap();
            messages.extend_from_slice(new_messages);
        }

        for listener in message_listeners.values() {
            listener.send(()).unwrap();
        }
    }

    pub fn subscribe(&self, sender: mpsc::Sender<()>) -> ListenerId {
        let id = get_listener_id();

        let mut message_listeners = self.message_listeners.lock().unwrap();
        message_listeners.insert(id, sender);

        id
    }

    pub fn unsubscribe(&self, id: ListenerId) {
        let mut message_listeners = self.message_listeners.lock().unwrap();
        message_listeners.remove(&id);
    }

    pub fn get_message_cursor(&self) -> u32 {
        self.messages.read().unwrap().len() as u32
    }

    pub fn get_messages_since(&self, cursor: u32) -> (u32, Vec<T>) {
        let messages = self.messages.read().unwrap();

        let current_cursor = messages.len() as u32;

        // Cursor is out of bounds or there are no new messages
        if cursor >= current_cursor {
            return (current_cursor, Vec::new());
        }

        (current_cursor, messages[(cursor as usize)..].to_vec())
    }
}