use std;
use super::aux::UncheckedSliceExt;

const MTF_VALUE_ARRAY: [u8; 256] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_VALUE_ARRAY.txt"));
const MTF_INDEX_ARRAY: [u8; 256] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_INDEX_ARRAY.txt"));
const MTF_NEXT_ARRAY: [u8; 256] = include!(
    concat!(env!("OUT_DIR"), "/", "MTF_NEXT_ARRAY.txt"));

pub struct MTFEncoder {
    value_array: [u8; 256],
    index_array: [u8; 256],
}

pub struct MTFDecoder {
    value_array: [u8; 256],
}

impl MTFEncoder {
    pub fn new() -> MTFEncoder {
        MTFEncoder {
            value_array: MTF_VALUE_ARRAY,
            index_array: MTF_INDEX_ARRAY,
        }
    }

    pub fn encode(&mut self, value: u8) -> u8 {
        unsafe {
            let index = *self.index_array.xget(value);
            let next_index = *MTF_NEXT_ARRAY.xget(index);
            let next_value = *self.value_array.xget(next_index);
            std::ptr::swap(self.index_array.xget_mut(value), self.index_array.xget_mut(next_value));
            std::ptr::swap(self.value_array.xget_mut(index), self.value_array.xget_mut(next_index));
            return index;
        }
    }
}

impl MTFDecoder {
    pub fn new() -> MTFDecoder {
        return MTFDecoder {
            value_array: MTF_VALUE_ARRAY,
        };
    }

    pub fn decode(&mut self, index: u8) -> u8 {
        unsafe {
            let value = *self.value_array.xget(index) as u8;
            let next_index = *MTF_NEXT_ARRAY.xget(index);
            std::ptr::swap(self.value_array.xget_mut(index), self.value_array.xget_mut(next_index));
            return value;
        }
    }
}
