use bitflags::bitflags;

use crate::prelude::*;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
    pub struct Direction: u8 {
        const TOP =     0b00000001;
        const BOTTOM =  0b00000010;
        const LEFT =    0b00000100;
        const RIGHT =   0b00001000;
    }
}
