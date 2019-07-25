use std;
use super::auxility::UncheckedSliceExt;

pub const MTF_NUM_SYMBOLS: usize = include!(concat!(env!("OUT_DIR"), "/", "MTF_VALUE_ARRAY.txt")).len();
const MTF_VALUE_ARRAY: [u16; MTF_NUM_SYMBOLS] = include!(concat!(env!("OUT_DIR"), "/", "MTF_VALUE_ARRAY.txt"));
const MTF_INDEX_ARRAY: [u16; MTF_NUM_SYMBOLS] = include!(concat!(env!("OUT_DIR"), "/", "MTF_INDEX_ARRAY.txt"));
const MTF_NEXT_ARRAY:  [u8;  MTF_NUM_SYMBOLS] = include!(concat!(env!("OUT_DIR"), "/", "MTF_NEXT_ARRAY.txt"));

pub struct MTFCoder {
    vs: [u16; MTF_NUM_SYMBOLS],
    is: [u16; MTF_NUM_SYMBOLS],
}

impl MTFCoder {
    pub fn new() -> MTFCoder {
        return MTFCoder {
            vs: MTF_VALUE_ARRAY,
            is: MTF_INDEX_ARRAY,
        };
    }

    pub unsafe fn encode(&mut self, v: u16, vunlikely: u16) -> u16 {
        let i = self.is.nc()[v as usize];
        let iunlikely = self.is.nc()[vunlikely as usize];

        self.update(v, i);
        return match i.cmp(&iunlikely) {
            std::cmp::Ordering::Less    => i,
            std::cmp::Ordering::Greater => i - 1,
            std::cmp::Ordering::Equal   => MTF_VALUE_ARRAY.len() as u16 - 1,
        };
    }

    pub unsafe fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        let iunlikely = self.is.nc()[vunlikely as usize];
        let i = match i {
            _ if i < iunlikely => i,
            _ if i < MTF_VALUE_ARRAY.len() as u16 - 1 => i + 1,
            _ => iunlikely,
        };
        let v = self.vs.nc()[i as usize];

        self.update(v, i);
        return v;
    }

    unsafe fn update(&mut self, v: u16, i: u16) {
        if i < 32 {
            let ni1 = MTF_NEXT_ARRAY.nc()[i as usize] as u16;
            let nv1 = self.vs.nc()[ni1 as usize];
            std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv1 as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(i as usize), self.vs.get_unchecked_mut(ni1 as usize));

        } else {
            let ni1 = MTF_NEXT_ARRAY.nc()[i as usize] as u16;
            let ni2 = (i + ni1 as u16) / 2;
            let nv2 = self.vs.nc()[ni2 as usize];
            std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv2 as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(i as usize), self.vs.get_unchecked_mut(ni2 as usize));
            let nv1 = self.vs.nc()[ni1 as usize];
            std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv1 as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(ni2 as usize), self.vs.get_unchecked_mut(ni1 as usize));
        }
    }
}
