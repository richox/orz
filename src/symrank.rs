use std::cmp::Ordering;

use crate::{SYMRANK_NUM_SYMBOLS, unchecked};

use unchecked_index::UncheckedIndex;

const SYMRANK_NEXT_ARRAY: [u16; SYMRANK_NUM_SYMBOLS] =
    include!(concat!(env!("OUT_DIR"), "/", "SYMRANK_NEXT_ARRAY.txt"));

#[derive(Clone, Copy)]
pub struct SymRankCoder {
    value_array: UncheckedIndex<[u16; SYMRANK_NUM_SYMBOLS]>,
    index_array: UncheckedIndex<[u16; SYMRANK_NUM_SYMBOLS]>,
}

impl SymRankCoder {
    pub fn new() -> Self {
        SymRankCoder {
            value_array: unchecked!([0; SYMRANK_NUM_SYMBOLS]),
            index_array: unchecked!([0; SYMRANK_NUM_SYMBOLS]),
        }
    }

    pub fn init(&mut self, value_array: &[u16]) {
        for (i, &value) in value_array.iter().enumerate() {
            self.value_array[i] = value;
            self.index_array[self.value_array[i] as usize] = i as u16;
        }
    }

    pub unsafe fn encode(&mut self, v: u16, vunlikely: u16) -> u16 {
        let i = self.index_array[v as usize];
        let iunlikely = self.index_array[vunlikely as usize];

        self.update(v, i);
        match i.cmp(&iunlikely) {
            Ordering::Less => i,
            Ordering::Greater => i - 1,
            Ordering::Equal => SYMRANK_NUM_SYMBOLS as u16 - 1,
        }
    }

    pub unsafe fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        let iunlikely = self.index_array[vunlikely as usize];
        let i = match () {
            _ if i < iunlikely => i,
            _ if i < SYMRANK_NUM_SYMBOLS as u16 - 1 => i + 1,
            _ => iunlikely,
        };
        let v = self.value_array[i as usize];

        self.update(v, i);
        v
    }

    unsafe fn update(&mut self, v: u16, i: u16) {
        let symrank_next_array = unchecked!(&SYMRANK_NEXT_ARRAY);

        if i < 32 {
            let ni1 = symrank_next_array[i as usize];
            let nv1 = self.value_array[ni1 as usize];
            std::ptr::swap(
                &mut self.index_array[v as usize],
                &mut self.index_array[nv1 as usize],
            );
            std::ptr::swap(
                &mut self.value_array[i as usize],
                &mut self.value_array[ni1 as usize],
            );
        } else {
            let ni1 = symrank_next_array[i as usize];
            let ni2 = (i + ni1) / 2;

            let nv2 = self.value_array[ni2 as usize];
            std::ptr::swap(
                &mut self.index_array[v as usize],
                &mut self.index_array[nv2 as usize],
            );
            std::ptr::swap(
                &mut self.value_array[i as usize],
                &mut self.value_array[ni2 as usize],
            );

            let nv1 = self.value_array[ni1 as usize];
            std::ptr::swap(
                &mut self.index_array[v as usize],
                &mut self.index_array[nv1 as usize],
            );
            std::ptr::swap(
                &mut self.value_array[ni2 as usize],
                &mut self.value_array[ni1 as usize],
            );
        }
    }
}
