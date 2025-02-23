use std::{
    fs,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, IntoRawFd, RawFd},
        unix::{net::{UnixListener, UnixStream}, process::CommandExt},
    },
    process::Child,
    sync::{Arc, Mutex, Weak},
};

use bevy::utils::{HashMap, HashSet};
use dway_util::eventloop::{PollerGuard, PollerInner};
use nix::errno::Errno;
pub use x11rb::protocol::xproto::Window as XWindowID;
use x11rb::{
    connection::Connection,
    protocol::{
        composite::{ConnectionExt as CompositeConnectionExt, Redirect},
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ConnectionExt as XprotoConnectionExt,
            CursorWrapper, EventMask, FontWrapper, PropMode, WindowClass,
        },
    },
    rust_connection::{ConnectionError, DefaultStream, RustConnection, Stream},
    wrapper::ConnectionExt as RustConnectionExt,
};

use super::events::XWaylandError;
use crate::{
    client::{self, ClientData, ClientEvents},
    prelude::*,
    x11::atoms::Atoms,
};

#[derive(Debug)]
pub enum XWaylandThreadEvent {
    XWaylandEvent(x11rb::protocol::Event),
    CreateConnection(
        Weak<(RustConnection<UnixStreamWrapper>, Atoms)>,
        x11rb::protocol::xproto::Window,
    ),
    Disconnect(anyhow::Error),
}

#[derive(Component, Clone)]
pub struct XWaylandDisplayWrapper {
    pub inner: Arc<Mutex<XWaylandDisplay>>,
}

impl lazy_static::__Deref for XWaylandDisplayWrapper {
    type Target = Arc<Mutex<XWaylandDisplay>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl XWaylandDisplayWrapper {
    pub fn new(inner: XWaylandDisplay) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

#[derive(Deref, Debug)]
pub struct UnixStreamWrapper(PollerGuard<DefaultStream>);
impl Stream for UnixStreamWrapper {
    fn poll(&self, mode: x11rb::rust_connection::PollMode) -> io::Result<()> {
        Stream::poll(&*self.0, mode)
    }

    fn read(
        &self,
        buf: &mut [u8],
        fd_storage: &mut Vec<x11rb::utils::RawFdContainer>,
    ) -> io::Result<usize> {
        Stream::read(&*self.0, buf, fd_storage)
    }

    fn write(&self, buf: &[u8], fds: &mut Vec<x11rb::utils::RawFdContainer>) -> io::Result<usize> {
        Stream::write(&*self.0, buf, fds)
    }
}

#[derive(Debug)]
pub struct XWaylandDisplay {
    pub display_number: u32,
    pub connection: Weak<(RustConnection<UnixStreamWrapper>, Atoms)>,
    pub channel: Arc<crossbeam_channel::Receiver<XWaylandThreadEvent>>,
    pub windows_entitys: HashMap<u32, Entity>,
    pub screen_windows: HashSet<u32>,
    pub wm_window: Option<x11rb::protocol::xproto::Window>,
    pub child: Child,
    pub client: wayland_server::Client,
}

impl XWaylandDisplay {
    pub fn find_window(&self, id: XWindowID) -> Result<Entity, XWaylandError> {
        self.windows_entitys
            .get(&id)
            .copied()
            .ok_or(XWaylandError::WindowNotExists(id))
    }
}

impl XWaylandDisplay {
    pub fn spawn(
        dway_server: &mut DWayServer,
        dway_entity: Entity,
        commands: &mut Commands,
        events: &ClientEvents,
        poller: Arc<PollerInner>,
    ) -> Result<Entity> {
        let (display_number, streams) =
            Self::get_number().ok_or_else(|| anyhow!("failed to alloc dissplay number"))?;
        let (x11_socket, x11_stream) = UnixStream::pair()?;
        let (wayland_socket, wayland_client_stream) = UnixStream::pair()?;

        let child = Self::spawn_xwayland(display_number, streams, x11_socket, wayland_socket)?;
        let (tx, rx) = crossbeam_channel::bounded(1024);
        dway_server.display_number = Some(display_number as usize);

        let mut entity_mut = commands.spawn((Name::new(format!("xwayland:{}", display_number)),));
        let entity = entity_mut.id();
        let guard = unsafe {
            poller.clone().add_raw(
                &wayland_client_stream,
                Some(Arc::new(move |world: &mut World| {
                    world.send_event(DispatchXWaylandDisplay(entity));
                    world.send_event(DispatchDisplay(dway_entity));
                })),
            )
        };
        let client = match dway_server.display.handle().insert_client(
            wayland_client_stream,
            Arc::new(ClientData::new(entity_mut.id(), events, guard)),
        ) {
            Ok(o) => o,
            Err(e) => {
                entity_mut.despawn();
                return Err(e.into());
            }
        };
        entity_mut.insert(client::Client::new(&client));
        entity_mut.set_parent(dway_entity);

        let this = Self {
            display_number,
            connection: Weak::default(),
            channel: Arc::new(rx),
            windows_entitys: Default::default(),
            wm_window: None,
            child,
            client,
            screen_windows: Default::default(),
        };
        entity_mut.insert(XWaylandDisplayWrapper {
            inner: Arc::new(Mutex::new(this)),
        });

        info!("spawn xwayland at :{}", display_number);
        std::thread::Builder::new()
            .name(format!("xwayland:{display_number}"))
            .spawn(move || {
                let result: Result<()> = (|| {
                    let (stream, _peer) = DefaultStream::from_unix_stream(x11_stream)?;
                    let stream = UnixStreamWrapper(poller.add(
                        stream,
                        Some(Arc::new(move |world: &mut World| {
                            world.send_event(DispatchXWaylandDisplay(entity));
                        })),
                    ));
                    let rust_connection = RustConnection::connect_to_stream(stream, 0)?;
                    let (atoms, wm_window) = Self::start_wm(&rust_connection)?;
                    let connection = Arc::new((rust_connection, atoms));
                    let _ = tx.send(XWaylandThreadEvent::CreateConnection(
                        Arc::downgrade(&connection),
                        wm_window,
                    ));
                    loop {
                        match connection.0.wait_for_event() {
                            Ok(event) => {
                                let _ = tx.send(XWaylandThreadEvent::XWaylandEvent(event));
                            }
                            Err(ConnectionError::IoError(e)) => {
                                error!("xwayland io error: {e}");
                                let _ = tx.send(XWaylandThreadEvent::Disconnect(anyhow!("{e}")));
                                return Err(e.into());
                            }
                            Err(e) => {
                                error!("xwayland error: {e}");
                            }
                        }
                    }
                })();
                if let Err(e) = result {
                    error!("xwayland connection error: {}", e);
                }
            })
            .unwrap();

        Ok(entity_mut.id())
    }

    pub fn start_wm(connection: &RustConnection<UnixStreamWrapper>) -> Result<(Atoms, u32)> {
        let screen = connection.setup().roots[0].clone();
        let atoms = Atoms::new(connection)?.reply()?;
        let font = FontWrapper::open_font(connection, "cursor".as_bytes())?;
        let cursor = CursorWrapper::create_glyph_cursor(
            connection,
            font.font(),
            font.font(),
            68,
            69,
            0,
            0,
            0,
            u16::MAX,
            u16::MAX,
            u16::MAX,
        )?;
        connection.change_window_attributes(
            screen.root,
            &ChangeWindowAttributesAux::default()
                .event_mask(
                    EventMask::SUBSTRUCTURE_REDIRECT
                        | EventMask::SUBSTRUCTURE_NOTIFY
                        | EventMask::PROPERTY_CHANGE
                        | EventMask::FOCUS_CHANGE,
                )
                .cursor(cursor.cursor()),
        )?;
        let win = connection.generate_id()?;
        connection.create_window(
            screen.root_depth,
            win,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &Default::default(),
        )?;
        let conn = &connection;
        conn.set_selection_owner(win, atoms.WM_S0, x11rb::CURRENT_TIME)?;
        conn.set_selection_owner(win, atoms._NET_WM_CM_S0, x11rb::CURRENT_TIME)?;
        conn.composite_redirect_subwindows(screen.root, Redirect::MANUAL)?;

        conn.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_SUPPORTED,
            AtomEnum::ATOM,
            &[
                atoms._NET_WM_STATE,
                atoms._NET_WM_STATE_MAXIMIZED_HORZ,
                atoms._NET_WM_STATE_MAXIMIZED_VERT,
                atoms._NET_WM_STATE_HIDDEN,
                atoms._NET_WM_STATE_FULLSCREEN,
                atoms._NET_WM_STATE_MODAL,
                atoms._NET_WM_STATE_FOCUSED,
                atoms._NET_ACTIVE_WINDOW,
                atoms._NET_WM_MOVERESIZE,
                atoms._NET_CLIENT_LIST,
                atoms._NET_CLIENT_LIST_STACKING,
            ],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_CLIENT_LIST,
            AtomEnum::WINDOW,
            &[],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_CLIENT_LIST_STACKING,
            AtomEnum::WINDOW,
            &[],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_ACTIVE_WINDOW,
            AtomEnum::WINDOW,
            &[0],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_SUPPORTING_WM_CHECK,
            AtomEnum::WINDOW,
            &[win],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            win,
            atoms._NET_SUPPORTING_WM_CHECK,
            AtomEnum::WINDOW,
            &[win],
        )?;
        conn.change_property8(
            PropMode::REPLACE,
            win,
            atoms._NET_WM_NAME,
            atoms.UTF8_STRING,
            "Smithay X WM".as_bytes(),
        )?;
        debug!(window = win, "Created WM Window");
        conn.flush()?;
        Ok((atoms, win))
    }

    fn spawn_xwayland(
        display_number: u32,
        streams: Vec<UnixListener>,
        x11_socket: UnixStream,
        wayland_socket: UnixStream,
    ) -> Result<Child> {
        let mut command =
            std::process::Command::new(std::env::var("XWAYLAND").unwrap_or("Xwayland".to_string()));
        command.args([
            &format!(":{display_number}"),
            "-rootless",
            "-terminate",
            "-wm",
            &x11_socket.as_raw_fd().to_string(),
        ]);
        command.env("WAYLAND_SOCKET", wayland_socket.as_raw_fd().to_string());

        unsafe {
            let wayland_socket_fd = wayland_socket.as_raw_fd();
            let wm_socket_fd = x11_socket.as_raw_fd();
            let socket_fds: Vec<_> = streams
                .into_iter()
                .map(|socket| socket.into_raw_fd())
                .collect();
            command.pre_exec(move || {
                // unset the CLOEXEC flag from the sockets we need to pass to xwayland
                Self::unset_cloexec(wayland_socket_fd)?;
                Self::unset_cloexec(wm_socket_fd)?;
                for &socket in socket_fds.iter() {
                    Self::unset_cloexec(socket)?;
                }
                Ok(())
            });
        }

        let child = command.spawn()?;
        Ok(child)
    }

    fn unset_cloexec(fd: RawFd) -> io::Result<()> {
        use nix::fcntl::{fcntl, FcntlArg, FdFlag};
        fcntl(fd, FcntlArg::F_SETFD(FdFlag::empty()))?;
        Ok(())
    }

    fn get_number() -> Option<(u32, Vec<UnixListener>)> {
        for d in 0..255 {
            if Self::lock_display(d).is_some() {
                if let Ok(Some(streams)) = Self::open_x11_sockets_for_display(d, true) {
                    return Some((d, streams));
                };
            }
        }
        None
    }

    fn lock_display(number: u32) -> Option<()> {
        let filename = format!("/tmp/.X{}-lock", number);
        let lockfile = ::std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filename);
        match lockfile {
            Ok(mut file) => {
                let ret = file.write_fmt(format_args!("{:>10}\n", ::nix::unistd::Pid::this()));
                if ret.is_err() {
                    ::std::mem::drop(file);
                    let _ = ::std::fs::remove_file(&filename);
                    None
                } else {
                    Some(())
                }
            }
            Err(_) => {
                let mut file = ::std::fs::File::open(&filename).ok()?;
                let mut spid = [0u8; 11];
                file.read_exact(&mut spid).ok()?;
                ::std::mem::drop(file);
                let pid = ::nix::unistd::Pid::from_raw(
                    ::std::str::from_utf8(&spid)
                        .ok()?
                        .trim()
                        .parse::<i32>()
                        .ok()?,
                );
                if let Err(Errno::ESRCH) = ::nix::sys::signal::kill(pid, None) {
                    if let Ok(()) = ::std::fs::remove_file(filename) {
                        return Some(());
                    } else {
                        return None;
                    }
                }
                None
            }
        }
    }

    fn open_x11_sockets_for_display(
        display: u32,
        open_abstract_socket: bool,
    ) -> Result<Option<Vec<UnixListener>>> {
        let lock_path = format!("/tmp/.X{}-lock", display);
        if fs::metadata(lock_path).is_ok() {
            return Ok(None);
        }
        let path = format!("/tmp/.X11-unix/X{}", display);
        let _ = ::std::fs::remove_file(&path);
        let sockets = vec![UnixListener::bind(path)?];
        //if open_abstract_socket {
        //    let abs_addr = socket::UnixAddr::new_abstract(path.as_bytes()).unwrap();
        //    sockets.push(UnixListener::bind_addr(socket_addr)?);
        //}
        Ok(Some(sockets))
    }
}

#[allow(missing_docs)]
pub mod atoms {
    x11rb::atom_manager! {
        /// Atoms used by the XWM and X11Surface types
        pub Atoms:
        AtomsCookie {
            // wayland-stuff
            WL_SURFACE_ID,

            // private
            _SMITHAY_CLOSE_CONNECTION,

            // data formats
            UTF8_STRING,

            // client -> server
            WM_HINTS,
            WM_PROTOCOLS,
            WM_TAKE_FOCUS,
            WM_DELETE_WINDOW,
            WM_CHANGE_STATE,
            _NET_WM_NAME,
            _NET_WM_MOVERESIZE,
            _NET_WM_PID,
            _NET_WM_WINDOW_TYPE,
            _NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
            _NET_WM_WINDOW_TYPE_DIALOG,
            _NET_WM_WINDOW_TYPE_MENU,
            _NET_WM_WINDOW_TYPE_NOTIFICATION,
            _NET_WM_WINDOW_TYPE_NORMAL,
            _NET_WM_WINDOW_TYPE_POPUP_MENU,
            _NET_WM_WINDOW_TYPE_SPLASH,
            _NET_WM_WINDOW_TYPE_TOOLBAR,
            _NET_WM_WINDOW_TYPE_TOOLTIP,
            _NET_WM_WINDOW_TYPE_UTILITY,
            _NET_WM_STATE_MODAL,
            _MOTIF_WM_HINTS,

            // server -> client
            WM_S0,
            WM_STATE,
            _NET_WM_CM_S0,
            _NET_SUPPORTED,
            _NET_ACTIVE_WINDOW,
            _NET_CLIENT_LIST,
            _NET_CLIENT_LIST_STACKING,
            _NET_WM_PING,
            _NET_WM_STATE,
            _NET_WM_STATE_MAXIMIZED_VERT,
            _NET_WM_STATE_MAXIMIZED_HORZ,
            _NET_WM_STATE_HIDDEN,
            _NET_WM_STATE_FULLSCREEN,
            _NET_WM_STATE_FOCUSED,
            _NET_SUPPORTING_WM_CHECK,
        }
    }
}
