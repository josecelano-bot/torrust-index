use crate::traits::Coordinate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CoordinateRange<C: Coordinate> {
    pub lo: C,
    pub hi: C,
}

impl<C: Coordinate> CoordinateRange<C> {
    #[must_use]
    pub const fn new(lo: C, hi: C) -> Self {
        Self { lo, hi }
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.lo >= self.hi
    }

    #[must_use]
    pub fn clamp(self, domain_lo: C, domain_hi: C) -> Self {
        let lo = if self.lo < domain_lo {
            domain_lo
        } else {
            self.lo
        };
        let hi = if self.hi > domain_hi {
            domain_hi
        } else {
            self.hi
        };
        Self { lo, hi }
    }

    #[must_use]
    pub fn overlaps(self, lo: C, hi: C) -> bool {
        self.lo < hi && self.hi > lo
    }

    #[must_use]
    pub fn covers(self, lo: C, hi: C) -> bool {
        self.lo <= lo && self.hi >= hi
    }

    #[must_use]
    pub fn clip(self, lo: C, hi: C) -> Self {
        let clipped_lo = if self.lo > lo { self.lo } else { lo };
        let clipped_hi = if self.hi < hi { self.hi } else { hi };
        Self {
            lo: clipped_lo,
            hi: clipped_hi,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CoordinateRange;

    #[test]
    fn clamp_limits_range_to_domain() {
        let range = CoordinateRange::new(2u8, 12u8).clamp(4u8, 10u8);
        assert_eq!(range, CoordinateRange::new(4u8, 10u8));
    }

    #[test]
    fn overlaps_is_false_for_touching_ranges() {
        assert!(!CoordinateRange::new(0u8, 4u8).overlaps(4u8, 8u8));
    }

    #[test]
    fn clip_returns_intersection() {
        let clipped = CoordinateRange::new(2u8, 10u8).clip(4u8, 8u8);
        assert_eq!(clipped, CoordinateRange::new(4u8, 8u8));
    }
}