use core::cmp::Ordering;
use core::ops::Deref;
use core::ops::DerefMut;
use core::slice;
use x64::mem::MemoryRegion;

pub struct PhysMemMap<const MAX: usize> {
    pub regions: [MemoryRegion; MAX],
    pub len: usize,
}

impl<const MAX: usize> PhysMemMap<MAX> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            regions: [MemoryRegion::null(); MAX],
            len: 0,
        }
    }
}

impl<const MAX: usize> PhysMemMap<MAX> {
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[inline]
    pub fn capacity(&self) -> usize {
        MAX
    }
}

impl<const MAX: usize> PhysMemMap<MAX> {
    pub fn add(&mut self, region: MemoryRegion) {
        // Merge with entry if overlapping or if tail of one is head of other
        for entry in &mut self.regions[..self.len] {
            if let Some(combined) = *entry + region {
                *entry = combined;
                self.minimize();
                return;
            }
        }
        if self.len < MAX {
            self.regions[self.len] = region;
            self.len += 1;
            self.minimize();
        }
    }
    pub fn minimize(&mut self) {
        self.sort_start_addr();
        for i in 0..self.len {
            if i > 0 && (self.regions[i - 1] + self.regions[i]).is_some() {
                self.regions[i - 1] += self.regions[i];
                self.regions[i] = MemoryRegion::null();
            }
            if i + 1 < self.len && (self.regions[i] + self.regions[i + 1]).is_some() {
                self.regions[i] += self.regions[i + 1];
                self.regions[i + 1] = MemoryRegion::null();
            }
        }
        self.regions[..self.len].sort_unstable_by(|r0, r1| {
            if r0.is_null() {
                Ordering::Greater
            } else if r1.is_null() {
                Ordering::Less
            } else {
                r0.start().cmp(&r1.start())
            }
        });
        self.len = self.regions.iter().position(|r| r.is_null()).unwrap_or(MAX);
    }
    pub fn sort_start_addr(&mut self) {
        self.regions[..self.len].sort_unstable_by_key(|r| r.start());
    }
    pub fn sort_size(&mut self) {
        self.regions[..self.len].sort_unstable_by_key(|r| r.size());
    }
}

impl<const MAX: usize> PhysMemMap<MAX> {
    pub fn iter(&self) -> slice::Iter<MemoryRegion> {
        self.regions[..self.len].iter()
    }
}

impl<const MAX: usize> Default for PhysMemMap<MAX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX: usize> Deref for PhysMemMap<MAX> {
    type Target = [MemoryRegion];

    fn deref(&self) -> &Self::Target {
        &self.regions[..self.len]
    }
}

impl<const MAX: usize> DerefMut for PhysMemMap<MAX> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.regions[..self.len]
    }
}

impl<'a, const MAX: usize> IntoIterator for &'a PhysMemMap<MAX> {
    type Item = &'a MemoryRegion;
    type IntoIter = slice::Iter<'a, MemoryRegion>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
