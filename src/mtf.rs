use std;
use super::aux::UncheckedSliceExt;

const MTF_VALUE_ARRAY: [u8; 256] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_VALUE_ARRAY.txt"));
const MTF_INDEX_ARRAY: [u8; 256] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_INDEX_ARRAY.txt"));
const MTF_NEXT_ARRAY: [u8; 256] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_NEXT_ARRAY.txt"));

pub struct MTFCoder {
    value_array: [u8; 256],
    index_array: [u8; 256],
}

impl MTFCoder {
    pub fn new() -> MTFCoder {
        MTFCoder {
            value_array: MTF_VALUE_ARRAY,
            index_array: MTF_INDEX_ARRAY,
        }
    }

    pub fn encode(&mut self, value: u8, value_unlikely: u8) -> u8 {
        unsafe {
            let index = self.index_array.nocheck()[value as usize];
            let index_unlikely = self.index_array.nocheck()[value_unlikely as usize];

            let next_index = MTF_NEXT_ARRAY.nocheck()[index as usize];
            let next_value = self.value_array.nocheck()[next_index as usize];
            std::ptr::swap(
                self.index_array.get_unchecked_mut(value as usize),
                self.index_array.get_unchecked_mut(next_value as usize));
            std::ptr::swap(
                self.value_array.get_unchecked_mut(index as usize),
                self.value_array.get_unchecked_mut(next_index as usize));

            if index == index_unlikely {
                return 255;
            }
            return index - (index > index_unlikely) as u8;
        }
    }

    pub fn decode(&mut self, index: u8, value_unlikely: u8) -> u8 {
        unsafe {
            let index_unlikely = self.index_array.nocheck()[value_unlikely as usize];
            let index =
                if index == 255 {
                    index_unlikely
                } else {
                    index + (index + 1 > index_unlikely) as u8
                };

            let value = self.value_array.nocheck()[index as usize];
            let next_index = MTF_NEXT_ARRAY.nocheck()[index as usize];
            let next_value = self.value_array.nocheck()[next_index as usize];
            std::ptr::swap(
                self.index_array.get_unchecked_mut(value as usize),
                self.index_array.get_unchecked_mut(next_value as usize));
            std::ptr::swap(
                self.value_array.get_unchecked_mut(index as usize),
                self.value_array.get_unchecked_mut(next_index as usize));
            return value;
        }
    }
}
