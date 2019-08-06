use std;
use super::auxility::UncheckedSliceExt;

const MTF_NEXT_ARRAY: [u16; super::MTF_NUM_SYMBOLS] = include!(concat!(env!("OUT_DIR"), "/", "MTF_NEXT_ARRAY.txt"));

#[derive(Clone, Copy)]
pub struct MTFCoder {
    vs: [u16; super::MTF_NUM_SYMBOLS],
    is: [u16; super::MTF_NUM_SYMBOLS],
}

impl MTFCoder {
    pub unsafe fn from_vs(vs: &[u16]) -> MTFCoder {
        let mut mtf_vs = [0; super::MTF_NUM_SYMBOLS];
        let mut mtf_is = [0; super::MTF_NUM_SYMBOLS];
        for i in 0..super::MTF_NUM_SYMBOLS {
            mtf_vs.nc_mut()[i] = vs.nc()[i];
            mtf_is.nc_mut()[vs.nc()[i] as usize] = i as u16;
        }
        return MTFCoder {vs: mtf_vs, is: mtf_is};
    }

    pub unsafe fn encode(&mut self, v: u16, vunlikely: u16) -> u16 {
        let i = self.is.nc()[v as usize];
        let iunlikely = self.is.nc()[vunlikely as usize];

        self.update(v, i);
        return match i.cmp(&iunlikely) {
            std::cmp::Ordering::Less    => i,
            std::cmp::Ordering::Greater => i - 1,
            std::cmp::Ordering::Equal   => super::MTF_NUM_SYMBOLS as u16 - 1,
        };
    }

    pub unsafe fn decode(&mut self, i: u16, vunlikely: u16) -> u16 {
        let iunlikely = self.is.nc()[vunlikely as usize];
        let i = match i {
            _ if i < iunlikely => i,
            _ if i < super::MTF_NUM_SYMBOLS as u16 - 1 => i + 1,
            _ => iunlikely,
        };
        let v = self.vs.nc()[i as usize];

        self.update(v, i);
        return v;
    }

    unsafe fn update(&mut self, v: u16, i: u16) {
        if i < 32 {
            let ni1 = MTF_NEXT_ARRAY.nc()[i as usize];
            let nv1 = self.vs.nc()[ni1 as usize];
            std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv1 as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(i as usize), self.vs.get_unchecked_mut(ni1 as usize));

        } else {
            let ni1 = MTF_NEXT_ARRAY.nc()[i as usize];
            let ni2 = (i + ni1) / 2;
            let nv2 = self.vs.nc()[ni2 as usize];
            std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv2 as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(i as usize), self.vs.get_unchecked_mut(ni2 as usize));
            let nv1 = self.vs.nc()[ni1 as usize];
            std::ptr::swap(self.is.get_unchecked_mut(v as usize), self.is.get_unchecked_mut(nv1 as usize));
            std::ptr::swap(self.vs.get_unchecked_mut(ni2 as usize), self.vs.get_unchecked_mut(ni1 as usize));
        }
    }
}
