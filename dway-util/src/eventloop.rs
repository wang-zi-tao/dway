use std::{
    any::{type_name, TypeId},
    num::NonZero,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    sync::{
        mpsc::{self, channel},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use backtrace::Backtrace;
use bevy::{
    app::{AppExit, MainScheduleOrder},
    ecs::schedule::{ExecutorKind, ScheduleLabel},
    prelude::*,
    utils::HashMap,
    window::RequestRedraw, winit::WakeUp,
};
use nix::{
    libc::listen,
    sys::{
        time::TimeSpec,
        timer::{Expiration, TimerSetTimeFlags},
        timerfd::{ClockId, TimerFd, TimerFlags},
    },
};
pub use polling::AsSource;
use polling::{Event, Events, PollMode};
use smallvec::SmallVec;
use smart_default::SmartDefault;

pub const FD_KEY_BEGIN: usize = 1024;
pub const FD_KEY_TIMER: usize = 1;

pub type FdCallback = Arc<dyn Fn(&mut World) + Send + Sync>;
pub type PollerCallback = Box<dyn Fn(&PollerResponse) -> bool + Send + Sync + 'static>;

structstruck::strike! {
    pub struct Poller {
        poller: Arc<PollerInner>,
        tx: Option<mpsc::Sender<
        #[derive(Debug, Default, Clone)]
        pub struct PollerRequest{
            pub quit: bool,
            pub add_timer: Option<Instant>,
        }>>,
        rx: Option<mpsc::Receiver<
        #[derive(Default, Clone)]
        pub struct PollerResponse {
            pub timeout: bool,
            pub fd_event: bool,
            pub timer_event: bool,
            pub commands: SmallVec<[FdCallback; 4]>,
        } >>,
    }
}

pub struct PollerRawGuard {
    fd: RawFd,
    poller: Arc<PollerInner>,
}

impl std::fmt::Debug for PollerRawGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerRawGuard")
            .field("fd", &self.fd)
            .finish()
    }
}
impl Drop for PollerRawGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = self.poller.remove_raw(self.fd);
        }
    }
}

pub struct PollerGuard<Fd: AsSource> {
    fd: Fd,
    poller: Arc<PollerInner>,
}

impl<Fd: AsSource + std::fmt::Debug> std::fmt::Debug for PollerGuard<Fd> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerGuard").field("fd", &self.fd).finish()
    }
}

impl<Fd: AsSource> std::ops::Deref for PollerGuard<Fd> {
    type Target = Fd;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}

impl<Fd: AsSource> std::ops::DerefMut for PollerGuard<Fd> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fd
    }
}

impl<Fd: AsSource> Drop for PollerGuard<Fd> {
    fn drop(&mut self) {
        let _ = self.poller.remove(self.fd.source());
    }
}

impl Poller {
    pub fn new() -> Self {
        let poller = polling::Poller::new().unwrap();
        let timer = TimerFd::new(ClockId::CLOCK_REALTIME, TimerFlags::TFD_CLOEXEC).unwrap();
        unsafe {
            poller
                .add_with_mode(
                    &timer.as_fd(),
                    Event::readable(FD_KEY_BEGIN),
                    PollMode::Edge,
                )
                .unwrap()
        };
        Self {
            poller: Arc::new(PollerInner {
                raw: poller,
                timer,
                fds: Default::default(),
                timeout: Duration::from_secs(1),
            }),
            tx: None,
            rx: None,
        }
    }

    pub fn inner(&self) -> &Arc<PollerInner> {
        &self.poller
    }

    pub fn handle(&mut self) -> Self {
        Self {
            poller: self.poller.clone(),
            tx: self.tx.clone(),
            rx: None,
        }
    }

    pub fn take_recevier(&mut self) -> Option<mpsc::Receiver<PollerResponse>> {
        self.rx.take()
    }

    pub fn take(&mut self) -> Self {
        Self {
            poller: self.poller.clone(),
            tx: self.tx.clone(),
            rx: self.rx.take(),
        }
    }

    pub fn get_runner(&mut self, callback: Option<PollerCallback>) -> impl FnOnce() {
        let (tx, thread_rx) = channel();
        let (thread_tx, rx) = channel();
        self.tx = Some(tx);
        self.rx = Some(rx);
        let poller = self.poller.clone();
        move || poller.run(thread_rx, thread_tx, callback).unwrap()
    }

    pub fn launch(&mut self, callback: Option<PollerCallback>) {
        let (tx, thread_rx) = channel();
        let (thread_tx, rx) = channel();
        self.tx = Some(tx);
        self.rx = Some(rx);
        let poller = self.poller.clone();
        std::thread::Builder::new()
            .name("epoll".to_string())
            .spawn(move || poller.run(thread_rx, thread_tx, callback).unwrap())
            .unwrap();
    }

    pub fn add<Fd: AsSource>(&mut self, fd: Fd) -> PollerGuard<Fd> {
        self.poller.clone().add(fd, None)
    }

    pub unsafe fn add_raw<Fd: AsRawFd>(&mut self, fd: &Fd) -> PollerRawGuard {
        self.poller.clone().add_raw(fd, None)
    }

    pub fn add_with_callback<Fd: AsSource, F>(&mut self, fd: Fd, callback: F) -> PollerGuard<Fd>
    where
        F: Fn(&mut World) + Send + Sync + 'static,
    {
        self.poller.clone().add(fd, Some(Arc::new(callback)))
    }

    pub unsafe fn add_raw_with_callback<Fd: AsRawFd, F>(
        &mut self,
        fd: &Fd,
        callback: F,
    ) -> PollerRawGuard
    where
        F: Fn(&mut World) + Send + Sync + 'static,
    {
        self.poller.clone().add_raw(fd, Some(Arc::new(callback)))
    }

    pub fn delete(&mut self, fd: OwnedFd) {
        self.poller.raw.delete(fd).unwrap();
    }

    pub fn send(&mut self, event: PollerRequest) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(event);
        }
    }
}

structstruck::strike! {
    pub struct PollerInner {
        raw: polling::Poller,
        fds: Mutex<HashMap<usize, struct ListenFd{
            fd: RawFd,
            callback: Option<FdCallback>
        }>>,
        timeout: Duration,
        timer: TimerFd,
    }
}

impl Drop for PollerInner {
    fn drop(&mut self) {
        let _ = self.raw.delete(&self.timer);
    }
}

impl PollerInner {
    fn fd_key(fd: RawFd) -> usize {
        FD_KEY_BEGIN + fd.as_raw_fd() as usize
    }

    unsafe fn do_add_raw(&self, fd: RawFd, callback: Option<FdCallback>) {
        debug!("add file descriptor ({fd:?}) into EPOLL");
        let key = Self::fd_key(fd);
        self.fds
            .lock()
            .unwrap()
            .insert(key, ListenFd { fd, callback });
        self.raw
            .add_with_mode(fd, Event::readable(key), PollMode::Level)
            .unwrap();
    }

    pub unsafe fn add_raw<Fd: AsRawFd>(
        self: Arc<Self>,
        fd: &Fd,
        callback: Option<FdCallback>,
    ) -> PollerRawGuard {
        let raw_fd = fd.as_raw_fd();
        debug!("add file descriptor ({:?}) into EPOLL", raw_fd);
        unsafe {
            self.do_add_raw(raw_fd, callback);
        }
        PollerRawGuard {
            fd: raw_fd,
            poller: self,
        }
    }

    pub fn add<Fd: AsSource>(
        self: Arc<Self>,
        fd: Fd,
        callback: Option<FdCallback>,
    ) -> PollerGuard<Fd> {
        debug!(
            "add file descriptor ({:?}: {}) into EPOLL",
            fd.as_fd(),
            type_name::<Fd>()
        );
        let raw_fd = fd.as_fd().as_raw_fd();
        unsafe {
            self.do_add_raw(raw_fd, callback);
        }
        PollerGuard { fd, poller: self }
    }

    pub fn remove(&self, fd: impl AsSource + AsRawFd) -> Result<()> {
        unsafe { self.remove_raw(fd.as_raw_fd()) }
    }

    pub unsafe fn remove_raw(&self, fd: RawFd) -> Result<()> {
        debug!("remove file descriptor ({:?}) from EPOLL", fd);
        let key = Self::fd_key(fd);
        self.fds.lock().unwrap().remove(&key);
        unsafe {
            let borrow_fd = BorrowedFd::borrow_raw(fd);
            self.raw.delete(borrow_fd)?;
        }
        Ok(())
    }

    fn run(
        &self,
        rx: mpsc::Receiver<PollerRequest>,
        tx: mpsc::Sender<PollerResponse>,
        callback: Option<PollerCallback>,
    ) -> Result<()> {
        let mut events = Events::with_capacity(NonZero::new(64).unwrap());
        'outer: loop {
            events.clear();
            let wait_begin = Instant::now();
            let wait_timeout = wait_begin + self.timeout;

            self.raw.wait(&mut events, Some(self.timeout))?;
            debug!(
                "poller wait {:?} {:?}",
                &events,
                Instant::now() - wait_begin
            );

            let mut response = PollerResponse::default();
            if Instant::now() >= wait_timeout {
                response.timeout = true;
            }

            for event in events.iter() {
                match event.key {
                    FD_KEY_TIMER => {
                        response.timeout = true;
                        debug!("reset timer");
                        self.timer.wait()?;
                    }
                    _ => {
                        if let Some(listen_fd) = self.fds.lock().unwrap().get(&event.key) {
                            debug!("poll fd {:?}", listen_fd.fd);
                            response.fd_event = true;
                            if let Some(callback) = &listen_fd.callback {
                                response.commands.push(callback.clone());
                            }
                        }
                    }
                }
            }

            let finish = callback.as_ref().map(|c| c(&response)).unwrap_or_default();
            if !finish {
                let _ = tx.send(response);
            }

            let frame_begin = Instant::now();
            let message = rx.recv()?;
            debug!(
                "poller recv {:?} {:?}",
                &events,
                Instant::now() - frame_begin
            );

            if message.quit {
                break 'outer;
            }
            if let Some(time) = message.add_timer {
                let now = Instant::now();
                if time > now + Duration::from_millis(1) {
                    debug!("set timer {}ms", (time - now).as_millis());
                    self.timer.set(
                        Expiration::OneShot(TimeSpec::from_duration(time - now)),
                        TimerSetTimeFlags::TFD_TIMER_CANCEL_ON_SET,
                    )?;
                } else {
                    self.raw.notify()?;
                };
            }
        }
        Ok(())
    }
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct PollerSchedule;

fn on_frame_begin(world: &mut World) {
    let mut commands = SmallVec::<[_; 8]>::new();
    {
        let poller = world.non_send_resource_mut::<Poller>();
        for response in poller.rx.iter().flat_map(|rx| rx.try_iter()) {
            commands.extend(response.commands);
        }
    }
    for command in commands {
        command(world);
    }
}

fn on_frame_finish(mut poller: NonSendMut<Poller>, exit: EventReader<AppExit>) {
    let quit = !exit.is_empty();
    poller.send(PollerRequest {
        quit,
        ..Default::default()
    });
}

structstruck::strike! {
    #[derive(SmartDefault)]
    pub struct EventLoopPlugin {
        pub mode: #[derive(Default,Clone)] enum EventLoopPluginMode {
            #[default]
            WinitMode,
            ManualMode,
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PollerSystems {
    PollEvent,
    Flush,
}

impl Plugin for EventLoopPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_non_send_resource(Poller::new());
        match &self.mode {
            EventLoopPluginMode::WinitMode => {
                let mut poller_schedule = Schedule::new(PollerSchedule);
                poller_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
                poller_schedule.add_systems(on_frame_begin.in_set(PollerSystems::PollEvent));
                app.add_schedule(poller_schedule);
                let mut main_schedule_order = app.world_mut().resource_mut::<MainScheduleOrder>();
                main_schedule_order.insert_after(First, PollerSchedule);

                app.add_systems(Last, on_frame_finish.in_set(PollerSystems::Flush));
                let winit_eventloop_proxy = app
                    .world()
                    .non_send_resource::<winit::event_loop::EventLoop<WakeUp>>()
                    .create_proxy();
                let mut poller = app.world_mut().non_send_resource_mut::<Poller>();
                poller.launch(Some(Box::new(move |_event| {
                    let _ = winit_eventloop_proxy.send_event(WakeUp);
                    false
                })));
            }
            EventLoopPluginMode::ManualMode => {}
        }
    }
}
