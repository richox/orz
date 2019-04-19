use std;
use super::auxility::UncheckedSliceExt;

const MTF_VALUE_ARRAY: [u16; 357] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_VALUE_ARRAY.txt"));
const MTF_INDEX_ARRAY: [u16; 357] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_INDEX_ARRAY.txt"));
const MTF_NEXT_ARRAY: [u8; 357] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_NEXT_ARRAY.txt"));

pub struct MTFCoder {
    vs: [u16; 357],
    is: [u16; 357],
}

impl MTFCoder {
    pub fn new() -> MTFCoder {
        return MTFCoder {
            vs: MTF_VALUE_ARRAY,
            is: MTF_INDEX_ARRAY,
        };
    }

    pub fn encode(&mut self, value: u16, value_unlikely: u16) -> u16 {
        unsafe {
            let index = self.is.nocheck()[value as usize];
            let index_unlikely = self.is.nocheck()[value_unlikely as usize];

            let next_index = MTF_NEXT_ARRAY.nocheck()[index as usize];
            let next_value = self.vs.nocheck()[next_index as usize];
            std::ptr::swap(self.is.get_unchecked_mut(value as usize), self.is.get_unchecked_mut(next_value as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(index as usize), self.vs.get_unchecked_mut(next_index as usize));

            return match index.cmp(&index_unlikely) {
                std::cmp::Ordering::Equal   => MTF_VALUE_ARRAY.len() as u16 - 1,
                std::cmp::Ordering::Less    => index,
                std::cmp::Ordering::Greater => index - 1,
            };
        }
    }

    pub fn decode(&mut self, index: u16, value_unlikely: u16) -> u16 {
        unsafe {
            let index_unlikely = self.is.nocheck()[value_unlikely as usize];
            let index = match index {
                _ if index + 1 == MTF_VALUE_ARRAY.len() as u16 => index_unlikely,
                _ if index + 1 <= index_unlikely               => index,
                _ if index + 1 >  index_unlikely               => index + 1,
                _ => unreachable!(),
            };

            let value = self.vs.nocheck()[index as usize];
            let next_index = MTF_NEXT_ARRAY.nocheck()[index as usize];
            let next_value = self.vs.nocheck()[next_index as usize];
            std::ptr::swap(self.is.get_unchecked_mut(value as usize), self.is.get_unchecked_mut(next_value as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(index as usize), self.vs.get_unchecked_mut(next_index as usize));
            return value;
        }
    }
}
