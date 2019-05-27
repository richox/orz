use std;
use super::auxility::UncheckedSliceExt;

pub const MTF_NUM_SYMBOLS: usize = 357;
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
        let i = self.is.nocheck()[v as usize];
        let iunlikely = self.is.nocheck()[vunlikely as usize];

        self.update(v, i);
        return match i.cmp(&iunlikely) {
            std::cmp::Ordering::Less    => i,
            std::cmp::Ordering::Greater => i - 1,
            std::cmp::Ordering::Equal   => MTF_VALUE_ARRAY.len() as u16 - 1,
        };
    }

    pub unsafe fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        let iunlikely = self.is.nocheck()[vunlikely as usize];
        let i = match i {
            _ if i < iunlikely => i,
            _ if i < MTF_VALUE_ARRAY.len() as u16 - 1 => i + 1,
            _ => iunlikely,
        };
        let v = self.vs.nocheck()[i as usize];

        self.update(v, i);
        return v;
    }

    unsafe fn update(&mut self, v: u16, i: u16) {
        let ni = MTF_NEXT_ARRAY.nocheck()[i as usize];
        let nv = self.vs.nocheck()[ni as usize];
        std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv as usize));
        std::ptr::swap(self.vs.get_unchecked_mut(i as usize), self.vs.get_unchecked_mut(ni as usize));
    }
}
