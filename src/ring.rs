use anyhow::{bail, Result};

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

    /// Move the cursor to the next element and return the element.
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
        let find_result = self.data.iter().enumerate().find(find);

        let Some((index, _)) = find_result else {
            return None;
        };

        self.cursor = index;
        Some(&self.data[self.cursor])
    }
}
