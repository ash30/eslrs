use super::format::*;

use std::cell::{Cell, OnceCell};

pub(crate) struct LazyParseCell<T, E> {
    data: Cell<Bytes>,
    value: OnceCell<Result<T, E>>,
}

impl<T> LazyParseCell<T, T::Error>
where
    T: EventFormat,
{
    pub(crate) fn new(data: Bytes) -> LazyParseCell<T, T::Error> {
        Self {
            data: Cell::new(data),
            value: OnceCell::new(),
        }
    }

    pub(crate) fn get(&self) -> &Result<T, T::Error> {
        self.value.get_or_init(|| {
            let data = self.data.replace(Bytes::new());
            T::try_from_raw(&data)
        })
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.value.get().is_none()
    }
}
