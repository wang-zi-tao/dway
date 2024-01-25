use bevy::prelude::*;
use calloop::{
    channel::{Channel, Sender},
    signals::Signal,
};
pub use calloop::{generic::Generic, EventSource, Interest, Mode, PostAction, Readiness};
use log::info;
use nix::{
    libc::{epoll_create, epoll_event, epoll_wait},
    poll::{PollFd, PollFlags},
};
use smart_default::SmartDefault;
use std::{
    any::{type_name, TypeId},
    os::fd::AsRawFd,
    sync::mpsc,
    time::{Duration, Instant},
};

pub struct Poll {}

pub unsafe fn poll() {
    // let fd = PollFd::new(fd, PollFlags::POLLIN|PollFlags::POLLERR);
    let fd = epoll_create(16);

    let mut events = [epoll_event { events: 0, u64: 0 }; 16];
    loop {
        let nfds = epoll_wait(fd, events.as_mut_ptr(), events.len() as i32, 0);
        for i in 0..nfds {
            let ev = &events[i as usize];
            let fd = ev.u64;
        }
    }
}

pub type Register = Box<dyn FnOnce(&mut calloop::LoopHandle<'static, ()>) + Send + Sync>;

pub enum EventLoopOperate {
    RequestUpdate,
    FrameFinish,
    Register(Register),
}

pub struct EventLoop {
    channel: Option<Channel<Register>>,
    // signal: Signal,
    sender: Sender<Register>,
    tx: Option<mpsc::Sender<()>>,
}

pub enum EventLoopControl {
    Continue,
    ContinueImmediate,
    ContinueWithTimeout(Duration),
    Stop,
}

pub struct EventLoopRunner(Channel<Register>);
impl EventLoopRunner {
    pub fn start_thread(
        self,
        eventloop: &mut EventLoop,
        mut callback: impl FnMut() -> EventLoopControl + Send + Sync + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        eventloop.tx.replace(tx);
        std::thread::Builder::new()
            .name("eventloop".to_string())
            .spawn(move || {
                self.run(Duration::from_secs(2), move || {
                    while rx.try_recv().is_ok() {}
                    let r = callback();
                    let _ = rx.recv();
                    r
                });
            })
            .unwrap();
    }

    pub fn run(self, duration: Duration, mut callback: impl FnMut() -> EventLoopControl + 'static) {
        let mut event_loop = calloop::EventLoop::<()>::try_new().unwrap();
        let mut handle = event_loop.handle();
        let signal = event_loop.get_signal();
        let _ = event_loop.handle().insert_source(self.0, move |e, _, _| {
            match e {
                calloop::channel::Event::Msg(r) => {
                    r(&mut handle);
                }
                calloop::channel::Event::Closed => {
                    info!("stoping eventloop");
                    signal.stop();
                }
            };
        });
        let mut timeout = duration;
        loop {
            if let Err(e) = event_loop.dispatch(timeout, &mut ()){
                error!("{e}");
            }
            match callback() {
                EventLoopControl::Continue => {
                    timeout = duration;
                }
                EventLoopControl::Stop => break,
                EventLoopControl::ContinueWithTimeout(t) => {
                    timeout = t;
                }
                EventLoopControl::ContinueImmediate => {
                    timeout = Duration::default();
                },
            };
        }
        info!("eventloop stopped");
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLoop {
    pub fn new() -> Self {
        let (sender, channel) = calloop::channel::channel();
        Self {
            sender,
            channel: Some(channel),
            tx: None,
        }
    }

    pub fn add<F: AsRawFd + 'static + Send + Sync>(
        &mut self,
        source: Generic<F, std::io::Error>,
        mut callback: impl for<'l> FnMut(Readiness, &'l mut F) + 'static + Send + Sync,
    ) {
        let _ = self.sender.send(Box::new(move |handle| {
            if let Err(e) = handle.insert_source(source, move |event, metadata, _| {
                if event.error {
                    return Ok(PostAction::Remove);
                }
                callback(event, metadata);
                Ok(PostAction::Continue)
            }) {
                error!("failed to insert source: {e:?}")
            };
        }));
    }

    pub fn add_fd_to_read(&mut self, fd: &impl AsRawFd) {
        self.add(
            Generic::new(fd.as_raw_fd(), Interest::READ, Mode::Level),
            |_, _| {},
        );
    }

    pub fn start(&mut self, callback: impl FnMut() -> EventLoopControl + Send + Sync + 'static) {
        let runner = self.runner();
        runner.start_thread(self, callback);
    }

    pub fn runner(&mut self) -> EventLoopRunner {
        let channel = self.channel.take().unwrap();
        EventLoopRunner(channel)
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        info!("stop eventloop");
    }
}

fn on_frame_finish(eventloop: NonSendMut<EventLoop>) {
    if let Some(tx) = &eventloop.tx {
        let _ = tx.send(());
    }
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

impl Plugin for EventLoopPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_non_send_resource(EventLoop::new());
        match &self.mode {
            EventLoopPluginMode::WinitMode => {
                debug!(
                    "require resource: [{:X?}] {}",
                    TypeId::of::<winit::event_loop::EventLoop<()>>(),
                    type_name::<winit::event_loop::EventLoop<()>>(),
                );

                let winit_eventloop_proxy = app
                    .world
                    .non_send_resource::<winit::event_loop::EventLoop<()>>()
                    .create_proxy();
                let mut eventloop = app.world.non_send_resource_mut::<EventLoop>();
                eventloop.start(Box::new(move || {
                    let _ = winit_eventloop_proxy.send_event(());
                    EventLoopControl::Continue
                }));
            }
            EventLoopPluginMode::ManualMode => {}
        }
        app.add_systems(Last, on_frame_finish);
    }
}
