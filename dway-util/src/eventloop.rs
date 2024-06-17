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
use bevy::{app::AppExit, prelude::*, utils::HashMap, window::RequestRedraw};
use nix::sys::{
    time::TimeSpec,
    timer::{Expiration, TimerSetTimeFlags},
    timerfd::{ClockId, TimerFd, TimerFlags},
};
pub use polling::AsSource;
use polling::{Event, Events, PollMode};
use smart_default::SmartDefault;

pub const FD_KEY_BEGIN: usize = 1024;
pub const FD_KEY_TIMER: usize = 1;

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
        #[derive(Debug, Default, Clone)]
        pub struct PollerResponse {
            pub timeout: bool,
            pub fd_event: bool,
            pub timer_event: bool,
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
        self.poller.clone().add(fd)
    }

    pub unsafe fn add_raw<Fd: AsRawFd>(&mut self, fd: &Fd) -> PollerRawGuard {
        self.poller.clone().add_raw(fd)
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

    unsafe fn do_add_raw(&self, fd: RawFd) {
        debug!("add file descriptor ({fd:?}) into EPOLL");
        let key = Self::fd_key(fd);
        self.fds.lock().unwrap().insert(key, ListenFd { fd });
        self.raw
            .add_with_mode(fd, Event::readable(key), PollMode::Level)
            .unwrap();
    }

    pub unsafe fn add_raw<Fd: AsRawFd>(self: Arc<Self>, fd: &Fd) -> PollerRawGuard {
        let raw_fd = fd.as_raw_fd();
        unsafe {
            self.do_add_raw(raw_fd);
        }
        PollerRawGuard {
            fd: raw_fd,
            poller: self,
        }
    }

    pub fn add<Fd: AsSource>(self: Arc<Self>, fd: Fd) -> PollerGuard<Fd> {
        let raw_fd = fd.as_fd().as_raw_fd();
        unsafe {
            self.do_add_raw(raw_fd);
        }
        PollerGuard { fd, poller: self }
    }

    pub fn remove(&self, fd: impl AsSource + AsRawFd) -> Result<()> {
        debug!("remove file descriptor ({:?}) from EPOLL", fd.as_raw_fd());
        let key = Self::fd_key(fd.as_raw_fd());
        self.fds.lock().unwrap().remove(&key);
        self.raw.delete(fd)?;
        Ok(())
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
                    FD_KEY_BEGIN => {
                        response.timeout = true;
                        debug!("reset timer");
                        self.timer.wait()?;
                    }
                    _ => {
                        if let Some(listen_fd) = self.fds.lock().unwrap().get(&event.key) {
                            debug!("poll fd {:?}", listen_fd.fd);
                            response.fd_event = true;
                        }
                    }
                }
            }

            let finish = callback.as_ref().map(|c| c(&response)).unwrap_or_default();
            if !finish {
                tx.send(response)?;
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
    Flush,
}

impl Plugin for EventLoopPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_non_send_resource(Poller::new());
        match &self.mode {
            EventLoopPluginMode::WinitMode => {
                app.add_systems(Last, on_frame_finish.in_set(PollerSystems::Flush));
                let winit_eventloop_proxy = app
                    .world
                    .non_send_resource::<winit::event_loop::EventLoop<RequestRedraw>>()
                    .create_proxy();
                let mut poller = app.world.non_send_resource_mut::<Poller>();
                poller.launch(Some(Box::new(move |_event| {
                    let _ = winit_eventloop_proxy.send_event(RequestRedraw);
                    true
                })));
            }
            EventLoopPluginMode::ManualMode => {}
        }
    }
}
