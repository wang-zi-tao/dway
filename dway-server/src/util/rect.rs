use bevy::{prelude::IVec2, reflect::Reflect};

#[derive(Default, Debug, Reflect, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRect {
    pub min: IVec2,
    pub max: IVec2,
}

impl IRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            min: IVec2::new(x, y),
            max: IVec2::new(x + w, y + h),
        }
    }
    pub fn from_pos_size(pos: IVec2, size: IVec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }
    pub fn pos(&self) -> IVec2 {
        self.min
    }
    pub fn size(&self) -> IVec2 {
        self.max - self.min
    }
    pub fn empty(&self) -> bool {
        let size = self.size();
        size.x <= 0 && size.y <= 0
    }
    pub fn intersection(&self, other: IRect) -> Self {
        let intersection = Self {
            min: IVec2 {
                x: i32::max(self.min.x, other.min.x),
                y: i32::max(self.min.y, other.min.y),
            },
            max: IVec2 {
                x: i32::min(self.max.x, other.max.x),
                y: i32::min(self.max.y, other.max.y),
            },
        };
        if intersection.size().x > 0 && intersection.size().y > 0 {
            intersection
        } else {
            Self::from_pos_size(intersection.pos(), Default::default())
        }
    }
    pub fn union(self, other: IRect) -> Self {
        if self.size().x <= 0 || self.size().y <= 0 {
            other
        } else if other.size().x <= 0 || other.size().y <= 0 {
            self
        } else {
            Self {
                min: IVec2 {
                    x: i32::min(self.min.x, other.min.x),
                    y: i32::min(self.min.y, other.min.y),
                },
                max: IVec2 {
                    x: i32::max(self.max.x, other.max.x),
                    y: i32::max(self.max.y, other.max.y),
                },
            }
        }
    }
    pub fn area(&self) -> isize {
        let size = self.size();
        size.x as isize * size.y as isize
    }
    pub fn x(&self) -> i32 {
        self.min.x
    }
    pub fn y(&self) -> i32 {
        self.min.y
    }
    pub fn width(&self) -> i32 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> i32 {
        self.max.y - self.min.y
    }
    pub fn set_x(&mut self, value: i32) {
        let w = self.width();
        self.min.x = value;
        self.max.x = self.x() + w;
    }
    pub fn set_y(&mut self, value: i32) {
        let h = self.height();
        self.min.y = value;
        self.max.y = self.y() + h;
    }
    pub fn set_width(&mut self, value: i32) {
        self.max.x = self.min.x + value;
    }
    pub fn set_height(&mut self, value: i32) {
        self.max.y = self.min.y + value;
    }
    pub fn include_point(&self, pos: IVec2) -> bool {
        self.min.x <= pos.x && self.min.y <= pos.y && pos.x < self.max.x && pos.y < self.max.y
    }
    pub fn set_pos(&mut self, pos: IVec2) {
        let size = self.max - self.min;
        self.min = pos;
        self.max = pos + size;
    }
    pub fn set_size(&mut self, size: IVec2) {
        self.max = self.min + size;
    }
    pub fn offset(self, offset: IVec2) -> Self {
        Self::from_pos_size(offset + self.pos(), self.size())
    }
    pub fn top_left(self) -> IVec2 {
        self.min
    }
    pub fn bottom_right(&self) -> IVec2 {
        self.max
    }
    pub fn top_right(&self) -> IVec2 {
        IVec2::new(self.x() + self.width(), self.y())
    }
    pub fn buttom_left(&self) -> IVec2 {
        IVec2::new(self.x(), self.y() + self.height())
    }
    pub fn center(&self) -> IVec2 {
        (self.min + self.max) / 2
    }
}
