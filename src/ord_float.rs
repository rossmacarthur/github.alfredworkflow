use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct OrdFloat(pub f64);

impl Ord for OrdFloat {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for OrdFloat {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrdFloat {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
    }
}

impl Eq for OrdFloat {}
