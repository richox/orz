use std;

const MTF_NEXT_ARRAY: [u16; super::MTF_NUM_SYMBOLS] = include!(concat!(env!("OUT_DIR"), "/", "MTF_NEXT_ARRAY.txt"));

#[derive(Clone, Copy)]
pub struct MTFCoder {
    vs: [u16; super::MTF_NUM_SYMBOLS],
    is: [u16; super::MTF_NUM_SYMBOLS],
}

impl MTFCoder {
    pub fn new() -> MTFCoder {
        return MTFCoder {
            vs: [0; super::MTF_NUM_SYMBOLS],
            is: [0; super::MTF_NUM_SYMBOLS],
        };
    }

    pub fn init(&mut self, vs: &[u16]) {
        for i in 0..super::MTF_NUM_SYMBOLS {
            self.vs[i] = vs[i];
            self.is[self.vs[i] as usize] = i as u16;
        }
    }

    pub unsafe fn encode(&mut self, v: u16, vunlikely: u16) -> u16 {
        let self_is = &mut unchecked_index::unchecked_index(&mut self.is);
        let i = self_is[v as usize];
        let iunlikely = self_is[vunlikely as usize];

        self.update(v, i);
        return match i.cmp(&iunlikely) {
            std::cmp::Ordering::Less    => i,
            std::cmp::Ordering::Greater => i - 1,
            std::cmp::Ordering::Equal   => super::MTF_NUM_SYMBOLS as u16 - 1,
        };
    }

    pub unsafe fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        let self_is = &mut unchecked_index::unchecked_index(&mut self.is);
        let self_vs = &mut unchecked_index::unchecked_index(&mut self.vs);

        let iunlikely = self_is[vunlikely as usize];
        let i = match i {
            _ if i < iunlikely => i,
            _ if i < super::MTF_NUM_SYMBOLS as u16 - 1 => i + 1,
            _ => iunlikely,
        };
        let v = self_vs[i as usize];

        self.update(v, i);
        return v;
    }

    unsafe fn update(&mut self, v: u16, i: u16) {
        let mtf_next_array = &unchecked_index::unchecked_index(&MTF_NEXT_ARRAY);
        let self_is = &mut unchecked_index::unchecked_index(&mut self.is);
        let self_vs = &mut unchecked_index::unchecked_index(&mut self.vs);

        if i < 32 {
            let ni1 = mtf_next_array[i as usize];
            let nv1 = self.vs[ni1 as usize];
            std::ptr::swap(&mut self.is[v as usize], &mut self.is[nv1 as usize]);
            std::ptr::swap(&mut self.vs[i as usize], &mut self.vs[ni1 as usize]);

        } else {
            let ni1 = mtf_next_array[i as usize];
            let ni2 = (i + ni1) / 2;

            let nv2 = self_vs[ni2 as usize];
            std::ptr::swap(&mut self_is[v as usize], &mut self_is[nv2 as usize]);
            std::ptr::swap(&mut self_vs[i as usize], &mut self_vs[ni2 as usize]);

            let nv1 = self_vs[ni1 as usize];
            std::ptr::swap(&mut self_is[v as usize], &mut self_is[nv1 as usize]);
            std::ptr::swap(&mut self_vs[ni2 as usize], &mut self_vs[ni1 as usize]);
        }
    }
}
