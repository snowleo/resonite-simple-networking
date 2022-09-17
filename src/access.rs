pub type UserId = u64;

static READ_BIT: u64 = 1;
static BIT_MASK: u64 = u64::MAX - (READ_BIT);

pub trait AccessCheck {
    fn base(&self) -> UserId;
    fn read_only(&self) -> bool;
    fn write_only(&self) -> bool;
    fn as_read_only(&self) -> UserId;
    fn as_write_only(&self) -> UserId;
}

impl AccessCheck for UserId {
    #[inline]
    fn base(&self) -> UserId {
        return self & BIT_MASK;
    }

    #[inline]
    fn read_only(&self) -> bool {
        return self & READ_BIT == READ_BIT;
    }

    #[inline]
    fn write_only(&self) -> bool {
        return self & READ_BIT != READ_BIT;
    }

    #[inline]
    fn as_read_only(&self) -> UserId {
        return self.base() + READ_BIT;
    }

    #[inline]
    fn as_write_only(&self) -> UserId {
        return self.base();
    }
}

#[cfg(test)]
mod tests {
    use crate::access::AccessCheck;

    #[test]
    fn access_check() {
        assert_eq!((0_u64).read_only(), false);
        assert_eq!((1_u64).read_only(), true);
        assert_eq!((0_u64).write_only(), true);
        assert_eq!((1_u64).write_only(), false);
        assert_eq!((0_u64).base(), 0_u64);
        assert_eq!((1_u64).base(), 0_u64);
        assert_eq!((2_u64).base(), 2_u64);
        assert_eq!((3_u64).base(), 2_u64);
        assert_eq!((0_u64.as_read_only()).read_only(), true);
        assert_eq!((0_u64.as_write_only()).read_only(), false);
        assert_eq!((0_u64.as_read_only()).write_only(), false);
        assert_eq!((0_u64.as_write_only()).write_only(), true);
    }
}