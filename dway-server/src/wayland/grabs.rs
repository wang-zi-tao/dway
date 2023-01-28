use smithay::{utils::{Logical, Point}, input::{SeatHandler, pointer::{GrabStartData, PointerGrab}}, };

use super::{DWayState, shell::WindowElement};

pub struct MoveSurfaceGrab {
    pub start_data: GrabStartData<DWayState>,
    pub window: WindowElement,
    pub initial_window_location: Point<i32, Logical>,
}
impl PointerGrab<DWayState> for MoveSurfaceGrab{
    fn motion(
        &mut self,
        data: &mut DWayState,
        handle: &mut smithay::input::pointer::PointerInnerHandle<'_, DWayState>,
        focus: Option<(<DWayState as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        todo!()
    }

    fn relative_motion(
        &mut self,
        data: &mut DWayState,
        handle: &mut smithay::input::pointer::PointerInnerHandle<'_, DWayState>,
        focus: Option<(<DWayState as SeatHandler>::PointerFocus, Point<i32, Logical>)>,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        todo!()
    }

    fn button(&mut self, data: &mut DWayState, handle: &mut smithay::input::pointer::PointerInnerHandle<'_, DWayState>, event: &smithay::input::pointer::ButtonEvent) {
        todo!()
    }

    fn axis(&mut self, data: &mut DWayState, handle: &mut smithay::input::pointer::PointerInnerHandle<'_, DWayState>, details: smithay::input::pointer::AxisFrame) {
        todo!()
    }

    fn start_data(&self) -> &GrabStartData<DWayState> {
        todo!()
    }
}
