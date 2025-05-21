use core::cmp::Ordering;
use core::ops::Deref;
use core::ops::DerefMut;
use core::slice;
use log::warn;
use x64::mem::PhysicalMemoryRegion;

pub struct PhysMemMap<const MAX: usize> {
    pub regions: [PhysicalMemoryRegion; MAX],
    pub len: usize,
}

impl<const MAX: usize> PhysMemMap<MAX> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            regions: [PhysicalMemoryRegion::null(); MAX],
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
    pub fn add(&mut self, region: PhysicalMemoryRegion) {
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
        } else {
            warn!("Skipping memory as we ran out of map location")
        }
    }
    pub fn sub(&mut self, other: PhysicalMemoryRegion) {
        let mut to_add = Self::new();
        for region in self.regions.iter_mut().filter(|region| !region.is_null()) {
            let intersection = region.intersect(other);
            if !intersection.is_null() {
                if region.start() == intersection.start() {
                    region.take_start(intersection.size());
                } else if region.end() == intersection.end() {
                    region.take_end(intersection.size());
                } else {
                    let head_size = intersection.start() - region.start();
                    let tail_size = region.size() - head_size - intersection.size();
                    region.take_end(tail_size + intersection.size());
                    to_add.add(PhysicalMemoryRegion::new(intersection.end(), tail_size));
                }
            }
        }
        for region in &to_add {
            self.add(*region);
        }
    }
    pub fn minimize(&mut self) {
        self.sort_start_addr();
        for i in 0..self.len {
            if i > 0 && (self.regions[i - 1] + self.regions[i]).is_some() {
                self.regions[i - 1] += self.regions[i];
                self.regions[i] = PhysicalMemoryRegion::null();
            }
            if i + 1 < self.len && (self.regions[i] + self.regions[i + 1]).is_some() {
                self.regions[i] += self.regions[i + 1];
                self.regions[i + 1] = PhysicalMemoryRegion::null();
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
    pub fn iter(&self) -> slice::Iter<PhysicalMemoryRegion> {
        self.regions[..self.len].iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<PhysicalMemoryRegion> {
        self.regions[..self.len].iter_mut()
    }
}

impl<const MAX: usize> Default for PhysMemMap<MAX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX: usize> Deref for PhysMemMap<MAX> {
    type Target = [PhysicalMemoryRegion];

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
    type Item = &'a PhysicalMemoryRegion;
    type IntoIter = slice::Iter<'a, PhysicalMemoryRegion>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, const MAX: usize> IntoIterator for &'a mut PhysMemMap<MAX> {
    type Item = &'a mut PhysicalMemoryRegion;
    type IntoIter = slice::IterMut<'a, PhysicalMemoryRegion>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
