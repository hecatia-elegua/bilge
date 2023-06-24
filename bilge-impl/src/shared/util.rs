pub trait Single<T> {
    /// consumes the collection. then, if the collection contained exactly one element, 
    /// it is returned as `SingleResult::Single`. if it contained more than one element or was empty, 
    /// `SingleResult::MoreThanOne` or `SingleResult::Empty` are returned, respectively.
    /// this can be used to validate that a collection has exactly one element.
    fn single(self) -> SingleResult<T>; 
}

impl<T, I> Single<T> for I where I: Iterator<Item = T> {
    fn single(mut self) -> SingleResult<T> {
        match (self.next(), self.next()) {
            (Some(elem), None) => SingleResult::Single(elem),
            (Some(_), Some(_)) => SingleResult::MoreThanOne,
            (None, _) => SingleResult::Empty, //this assumes iterator is fused.
        }
    }
}

pub enum SingleResult<T> {
    Single(T),
    MoreThanOne,
    Empty,
}