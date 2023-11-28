use std::collections::VecDeque;

use crate::prelude::*;

#[derive(Component, Default, Reflect)]
pub struct EventQueue<T: Sized + Send + Sync + 'static> {
    queue: VecDeque<T>,
}

impl<T: Sized + Send + Sync + 'static> EventQueue<T> {
    pub fn receive(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    pub fn send(&mut self, value: T) {
        self.queue.push_back(value)
    }
}
