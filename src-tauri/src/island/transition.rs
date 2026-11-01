use super::geometry::{frame_matches, Rect};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WindowId {
    pub pointer: usize,
    pub number: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Token {
    pub generation: u64,
    pub serial: u64,
    pub window: WindowId,
}

pub(super) struct Entry<T> {
    pub token: Token,
    pub value: T,
}

pub(super) struct Slot<T> {
    serial: u64,
    pub active: Option<Entry<T>>,
    pub pending: Option<Entry<T>>,
    pub dirty: bool,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            serial: 0,
            active: None,
            pending: None,
            dirty: false,
        }
    }
}

impl<T> Slot<T> {
    pub fn enqueue(
        &mut self,
        generation: u64,
        window: WindowId,
        value: T,
    ) -> (Token, bool, Option<Entry<T>>) {
        self.serial = self
            .serial
            .checked_add(1)
            .expect("Island transition serial exhausted");
        let token = Token {
            generation,
            serial: self.serial,
            window,
        };
        let entry = Entry { token, value };
        if self.active.is_some() {
            // ponytail: rapid reversal waits <=220ms; revisit only if native retargeting is required.
            (token, false, self.pending.replace(entry))
        } else {
            self.active = Some(entry);
            (token, true, None)
        }
    }

    pub fn serial_matches(&self, token: Token) -> bool {
        self.serial == token.serial
    }

    pub fn current(&self, token: Token, enabled: bool, generation: u64, window: WindowId) -> bool {
        enabled
            && generation == token.generation
            && window == token.window
            && self.serial_matches(token)
            && self
                .active
                .as_ref()
                .is_some_and(|entry| entry.token == token)
    }

    pub fn release(&mut self, token: Token) -> Option<Entry<T>> {
        if self
            .active
            .as_ref()
            .is_some_and(|entry| entry.token == token)
        {
            self.active.take()
        } else {
            None
        }
    }

    pub fn next(&mut self) -> Option<Token> {
        if self.active.is_some() {
            return None;
        }
        self.active = self.pending.take();
        self.active.as_ref().map(|entry| entry.token)
    }

    pub fn invalidate(&mut self, retire: bool) -> (Option<Entry<T>>, Option<Entry<T>>) {
        self.serial = self
            .serial
            .checked_add(1)
            .expect("Island transition serial exhausted");
        self.dirty = !retire;
        let active = if retire { self.active.take() } else { None };
        (active, self.pending.take())
    }

    pub fn geometry_event(&mut self, display_changed: bool) -> bool {
        if self.active.is_some() {
            self.dirty |= display_changed;
            false
        } else {
            true
        }
    }
}

pub(super) fn duration(before: Rect, target: Rect, reduced_motion: bool) -> Option<f64> {
    if reduced_motion
        || frame_matches(before, target)
        || (before.width == target.width && before.height == target.height)
    {
        None
    } else if target.height > before.height
        || (target.height == before.height && target.width > before.width)
    {
        Some(0.22)
    } else {
        Some(0.19)
    }
}

pub(super) fn verified_frame(actual: Rect, target: Rect) -> Result<(), AppError> {
    if frame_matches(actual, target) {
        Ok(())
    } else {
        Err(AppError::configuration(
            "Mini Island window operation failed",
        ))
    }
}
