//! [`NonEmpty<T>`]: a wrapper around `Vec<T>` that statically guarantees at
//! least one element.
//!
//! Constructed only via [`NonEmpty::try_from_vec`], which rejects an empty
//! input. Once built, the rest of the program can rely on the
//! "at-least-one-element" invariant without re-checking. Slice-level access
//! (indexing, iteration, slice methods) is available via
//! [`Deref<Target = [T]>`](Deref).

use std::ops::Deref;

/// A non-empty `Vec<T>`. Constructible only from a non-empty source.
///
/// The "non-empty" invariant is enforced at construction time, so callers can
/// use [`first`](Self::first) without an `Option` and avoid empty-collection
/// guards everywhere downstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T>(Vec<T>);

impl<T> NonEmpty<T> {
    /// Wrap a `Vec` if it is non-empty; otherwise return [`EmptyError`].
    pub fn try_from_vec(v: Vec<T>) -> Result<Self, EmptyError> {
        if v.is_empty() {
            Err(EmptyError)
        } else {
            Ok(Self(v))
        }
    }

    /// Borrow the first element. Always present — that's the whole point.
    pub fn first(&self) -> &T {
        &self.0[0]
    }

    /// Consume the wrapper, yielding the inner `Vec`.
    #[allow(dead_code)] // public API completeness; not currently used
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }

    /// Number of elements (always `>= 1`).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate by reference.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    /// Iterate by mutable reference.
    #[allow(dead_code)] // public API completeness; not currently used
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.0.iter_mut()
    }

    /// Bounds-checked access (`None` for out-of-range index).
    pub fn get(&self, i: usize) -> Option<&T> {
        self.0.get(i)
    }

    /// Borrow the underlying slice.
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}

impl<T> Deref for NonEmpty<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<'a, T> IntoIterator for &'a NonEmpty<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Returned by [`NonEmpty::try_from_vec`] when given an empty `Vec`.
#[derive(Debug, thiserror::Error)]
#[error("collection must not be empty")]
pub struct EmptyError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_vec_rejects_empty() {
        // Arrange
        let v: Vec<i32> = vec![];

        // Act
        let result = NonEmpty::try_from_vec(v);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn try_from_vec_accepts_non_empty() {
        // Arrange
        let v = vec![1, 2, 3];

        // Act
        let result = NonEmpty::try_from_vec(v);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn first_returns_first_element() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![10, 20, 30]).unwrap();

        // Act
        let first = ne.first();

        // Assert
        assert_eq!(*first, 10);
    }

    #[test]
    fn first_on_singleton_returns_only_element() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec!["solo"]).unwrap();

        // Act
        let first = ne.first();

        // Assert
        assert_eq!(*first, "solo");
    }

    #[test]
    fn len_reflects_element_count() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![1, 2, 3, 4]).unwrap();

        // Act
        let n = ne.len();

        // Assert
        assert_eq!(n, 4);
    }

    #[test]
    fn iter_yields_all_elements_in_order() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![1, 2, 3]).unwrap();

        // Act
        let collected: Vec<i32> = ne.iter().copied().collect();

        // Assert
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn into_iter_ref_yields_all_elements_in_order() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![1, 2, 3]).unwrap();

        // Act — exercise `IntoIterator for &NonEmpty<T>`
        let collected: Vec<i32> = (&ne).into_iter().copied().collect();

        // Assert
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn get_in_range_returns_some() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![10, 20, 30]).unwrap();

        // Act
        let elem = ne.get(1);

        // Assert
        assert_eq!(elem, Some(&20));
    }

    #[test]
    fn get_out_of_range_returns_none() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![10, 20]).unwrap();

        // Act
        let elem = ne.get(5);

        // Assert
        assert!(elem.is_none());
    }

    #[test]
    fn deref_allows_slice_indexing() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![10, 20, 30]).unwrap();

        // Act
        let middle = ne[1];

        // Assert
        assert_eq!(middle, 20);
    }

    #[test]
    fn deref_allows_slice_methods() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![1, 2, 3]).unwrap();

        // Act — `.contains` is a slice method
        let has_two = ne.contains(&2);

        // Assert
        assert!(has_two);
    }

    #[test]
    fn as_slice_borrows_underlying_data() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![1, 2, 3]).unwrap();

        // Act
        let slice: &[i32] = ne.as_slice();

        // Assert
        assert_eq!(slice, &[1, 2, 3]);
    }

    #[test]
    fn into_vec_preserves_elements() {
        // Arrange
        let ne = NonEmpty::try_from_vec(vec![1, 2, 3]).unwrap();

        // Act
        let v = ne.into_vec();

        // Assert
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn iter_mut_allows_mutation() {
        // Arrange
        let mut ne = NonEmpty::try_from_vec(vec![1, 2, 3]).unwrap();

        // Act
        for x in ne.iter_mut() {
            *x *= 2;
        }

        // Assert
        let collected: Vec<i32> = ne.iter().copied().collect();
        assert_eq!(collected, vec![2, 4, 6]);
    }
}
