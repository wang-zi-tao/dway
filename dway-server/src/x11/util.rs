use x11rb::protocol::xproto::GetGeometryReply;

use crate::util::rect::IRect;

pub fn geo_to_irect(geo:GetGeometryReply)->IRect{
    IRect::new(geo.x as i32,geo.y as i32,geo.width as i32,geo.height as i32)
}
