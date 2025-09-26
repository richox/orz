use std::hint::{assert_unchecked, unlikely};
use unchecked_index::UncheckedIndex;

use crate::{SYMRANK_NUM_SYMBOLS, unchecked};

#[derive(Clone, Copy)]
pub struct SymRankCoder {
    value_array: UncheckedIndex<[u16; SYMRANK_NUM_SYMBOLS]>,
    index_array: UncheckedIndex<[u16; SYMRANK_NUM_SYMBOLS]>,
    encoded_cnt: u32,
    encoded_idx_sum: u32,
}

impl SymRankCoder {
    pub fn new() -> Self {
        SymRankCoder {
            value_array: unchecked!([0; SYMRANK_NUM_SYMBOLS]),
            index_array: unchecked!([0; SYMRANK_NUM_SYMBOLS]),
            encoded_cnt: 0,
            encoded_idx_sum: 1000000, // for fast initialization
        }
    }

    pub fn init(&mut self, value_array: &[u16]) {
        for (i, &value) in value_array.iter().enumerate() {
            self.value_array[i] = value;
            self.index_array[self.value_array[i] as usize] = i as u16;
        }
    }

    pub fn encode(&mut self, v: u16, vunlikely: u16) -> u16 {
        unsafe { assert_unchecked((v as usize) < SYMRANK_NUM_SYMBOLS) };

        let i = self.index_array[v as usize];
        let iunlikely = self.index_array[vunlikely as usize];
        self.update(v, i);

        if unlikely(i == iunlikely) {
            return SYMRANK_NUM_SYMBOLS as u16 - 1;
        }
        i - (i > iunlikely) as u16
    }

    pub fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        unsafe { assert_unchecked((i as usize) < SYMRANK_NUM_SYMBOLS) };
        unsafe { assert_unchecked((vunlikely as usize) < SYMRANK_NUM_SYMBOLS) };

        let iunlikely = self.index_array[vunlikely as usize];
        let i = if unlikely(i == SYMRANK_NUM_SYMBOLS as u16 - 1) {
            iunlikely
        } else {
            i + !(i < iunlikely) as u16
        };
        let v = self.value_array[i as usize];
        self.update(v, i);
        v
    }

    fn update(&mut self, v: u16, i: u16) {
        unsafe { assert_unchecked((i as usize) < SYMRANK_NUM_SYMBOLS) };
        unsafe { assert_unchecked((v as usize) < SYMRANK_NUM_SYMBOLS) };

        // adjust encoded_cnt and encoded_idx_sum
        if self.encoded_cnt > SYMRANK_NUM_SYMBOLS as u32 {
            self.encoded_cnt = self.encoded_cnt * 9 / 10;
            self.encoded_idx_sum = self.encoded_idx_sum * 9 / 10;
        }
        self.encoded_cnt += 1;
        self.encoded_idx_sum += i as u32;

        let next_i = i
            .saturating_sub(i / 16 + (self.encoded_idx_sum / 16 / self.encoded_cnt) as u16)
            .max(i / 2);

        match i - next_i {
            0 => {}
            1 => {
                let ni1 = next_i;
                let nv1 = self.value_array[ni1 as usize];
                self.index_array[v as usize] = ni1;
                self.value_array[i as usize] = nv1;
                self.index_array[nv1 as usize] = i;
                self.value_array[ni1 as usize] = v;
            }
            n => {
                let ni2 = next_i;
                let ni1 = next_i + n / 2;
                let nv1 = self.value_array[ni1 as usize];
                let nv2 = self.value_array[ni2 as usize];
                self.value_array[i as usize] = nv1;
                self.index_array[nv1 as usize] = i;
                self.value_array[ni1 as usize] = nv2;
                self.index_array[nv2 as usize] = ni1;
                self.value_array[ni2 as usize] = v;
                self.index_array[v as usize] = ni2;
            }
        }
    }
}
