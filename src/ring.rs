use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Ring<T> {
    cursor: usize,
    data: Vec<T>,
}

/// Some structure with a static amount
impl<T> Ring<T> {
    /// Initialize the ring with a static array of data.
    /// The array may not be empty.
    pub fn new(data: Vec<T>) -> Result<Ring<T>> {
        if data.is_empty() {
            bail!("Ring cannot work with an empty Vec");
        }

        Ok(Ring { cursor: 0, data })
    }

    /// Get the current entry in the ring.
    /// This panics if the ring is empty.
    pub fn get(&mut self) -> &T {
        &self.data[self.cursor]
    }

    /// Move the cursor to the next element and return the element.
    /// This panics if the ring is empty.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> &T {
        // If we're at the end of the array, move to the start.
        if self.data.get(self.cursor + 1).is_none() {
            self.cursor = 0;
        } else {
            self.cursor += 1;
        }

        &self.data[self.cursor]
    }

    /// Move the cursor to the previous element and return the element.
    /// This panics if the ring is empty.
    pub fn prev(&mut self) -> &T {
        // If we're at the start of the array, move to the end.
        if self.cursor == 0 {
            self.cursor = self.data.len() - 1;
        } else {
            self.cursor -= 1;
        }

        &self.data[self.cursor]
    }

    /// Move the cursor to the first element that matches the given criteria.
    /// If none is found, do nothing and return `None`.
    pub fn find<Filter>(&mut self, find: Filter) -> Option<&T>
    where
        Filter: Fn(&(usize, &T)) -> bool,
    {
        let (index, _) = self.data.iter().enumerate().find(find)?;

        self.cursor = index;
        Some(&self.data[self.cursor])
    }
}
