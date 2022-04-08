use std::cmp::Ordering;

use crate::SYMRANK_NUM_SYMBOLS;

use unchecked_index::unchecked_index;

const SYMRANK_NEXT_ARRAY: [u16; SYMRANK_NUM_SYMBOLS] =
    include!(concat!(env!("OUT_DIR"), "/", "SYMRANK_NEXT_ARRAY.txt"));

#[derive(Clone, Copy)]
pub struct SymRankCoder {
    value_array: [u16; SYMRANK_NUM_SYMBOLS],
    index_array: [u16; SYMRANK_NUM_SYMBOLS],
}
impl Default for SymRankCoder {
    fn default() -> SymRankCoder {
        SymRankCoder {
            value_array: [0; SYMRANK_NUM_SYMBOLS],
            index_array: [0; SYMRANK_NUM_SYMBOLS],
        }
    }
}
impl SymRankCoder {
    pub fn init(&mut self, value_array: &[u16]) {
        for (i, &value) in value_array.iter().enumerate() {
            self.value_array[i] = value;
            self.index_array[self.value_array[i] as usize] = i as u16;
        }
    }

    pub unsafe fn encode(&mut self, v: u16, vunlikely: u16) -> u16 {
        let self_index_array = &mut unchecked_index(&mut self.index_array);
        let i = self_index_array[v as usize];
        let iunlikely = self_index_array[vunlikely as usize];

        self.update(v, i);
        match i.cmp(&iunlikely) {
            Ordering::Less => i,
            Ordering::Greater => i - 1,
            Ordering::Equal => SYMRANK_NUM_SYMBOLS as u16 - 1,
        }
    }

    pub unsafe fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        let self_index_array = &unchecked_index(&mut self.index_array);
        let self_value_array = &unchecked_index(&mut self.value_array);

        let iunlikely = self_index_array[vunlikely as usize];
        let i = match i {
            _ if i < iunlikely => i,
            _ if i < SYMRANK_NUM_SYMBOLS as u16 - 1 => i + 1,
            _ => iunlikely,
        };
        let v = self_value_array[i as usize];

        self.update(v, i);
        v
    }

    unsafe fn update(&mut self, v: u16, i: u16) {
        let symrank_next_array = &unchecked_index(SYMRANK_NEXT_ARRAY);
        let self_index_array = &mut unchecked_index(&mut self.index_array);
        let self_value_array = &mut unchecked_index(&mut self.value_array);

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

            let nv2 = self_value_array[ni2 as usize];
            std::ptr::swap(
                &mut self_index_array[v as usize],
                &mut self_index_array[nv2 as usize],
            );
            std::ptr::swap(
                &mut self_value_array[i as usize],
                &mut self_value_array[ni2 as usize],
            );

            let nv1 = self_value_array[ni1 as usize];
            std::ptr::swap(
                &mut self_index_array[v as usize],
                &mut self_index_array[nv1 as usize],
            );
            std::ptr::swap(
                &mut self_value_array[ni2 as usize],
                &mut self_value_array[ni1 as usize],
            );
        }
    }
}
